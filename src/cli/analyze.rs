//! Analyze command — score job fit via agent CLI with keyword fallback.

use std::time::Duration;

use serde::{Deserialize, Serialize};
use snafu::ResultExt;

use crate::{
    agent::{CliBackend, CliExecutor},
    app_config,
    db::Database,
    error::{Result, TenkiError},
};

/// Analysis result returned to the caller.
#[derive(Debug, Serialize)]
struct AnalysisResult {
    ok: bool,
    action: &'static str,
    id: String,
    score: f64,
    method: String,
    breakdown: serde_json::Value,
    notes: String,
}

/// Score breakdown from the LLM (5-criteria model).
#[derive(Debug, Deserialize, Serialize, Clone)]
struct ScoreBreakdown {
    #[serde(default)]
    skills: f64,
    #[serde(default)]
    experience: f64,
    #[serde(default)]
    location: f64,
    #[serde(default)]
    domain: f64,
    #[serde(default)]
    growth: f64,
    #[serde(default)]
    total: f64,
    #[serde(default)]
    notes: String,
}

/// Run the analyze command for a given application.
pub async fn run(
    db: &Database,
    id: &str,
    json: bool,
    backend_override: Option<&str>,
) -> Result<()> {
    let app = db.get_application(id).await?;
    let jd_text = app
        .jd_text
        .as_deref()
        .ok_or_else(|| TenkiError::MissingJdText { id: id.to_string() })?;

    // Try agent CLI scoring first, fall back to keyword matching
    let (score, method, breakdown, notes) = match try_agent_scoring(
        id,
        &app.position,
        jd_text,
        app.skills.as_deref(),
        backend_override,
    )
    .await
    {
        Ok(result) => result,
        Err(e) => {
            eprintln!("Agent scoring failed, falling back to keywords: {e}");
            let (s, b) = keyword_scoring(jd_text, app.skills.as_deref());
            let n = format!("Keyword fallback (agent failed: {e})");
            (s, "keyword".to_string(), b, n)
        }
    };

    // Persist score to DB
    db.update_fitness(id, score, &notes).await?;

    let result = AnalysisResult {
        ok: true,
        action: "analyze",
        id: id.to_string(),
        score,
        method,
        breakdown: serde_json::to_value(&breakdown).context(crate::error::JsonSnafu)?,
        notes,
    };

    if json {
        println!(
            "{}",
            serde_json::to_string(&result).context(crate::error::JsonSnafu)?
        );
    } else {
        eprintln!("Fitness score: {score:.0}/100 (method: {})", result.method);
        eprintln!("Notes: {}", result.notes);
    }

    Ok(())
}

/// Run batch analysis on all unscored applications.
pub async fn run_batch(
    db: &Database,
    top_n: Option<usize>,
    json: bool,
    backend_override: Option<&str>,
) -> Result<()> {
    let apps = db.list_unscored().await?;
    let apps: Vec<_> = match top_n {
        Some(n) => apps.into_iter().take(n).collect(),
        None => apps,
    };

    if apps.is_empty() {
        if json {
            println!(r#"{{"ok":true,"action":"analyze_batch","scored":0}}"#);
        } else {
            eprintln!("No unscored applications found.");
        }
        return Ok(());
    }

    eprintln!("Scoring {} applications...", apps.len());
    let mut scored = 0usize;

    for app in &apps {
        eprintln!("  → {} @ {} ...", app.position, app.company);
        run(db, &app.id, false, backend_override).await?;
        scored += 1;
    }

    if json {
        println!(
            "{}",
            serde_json::to_string(&serde_json::json!({
                "ok": true, "action": "analyze_batch", "scored": scored
            }))
            .context(crate::error::JsonSnafu)?
        );
    } else {
        eprintln!("Batch complete: {scored} applications scored.");
    }

    Ok(())
}

/// Build the scoring prompt for the agent CLI.
fn build_prompt(position: &str, jd_text: &str, skills: Option<&str>) -> String {
    let skills_section = skills
        .map(|s| format!("\n\nCandidate skills: {s}"))
        .unwrap_or_default();

    format!(
        r#"You are a job fit scoring engine. Analyze the candidate's fit for this role and return ONLY a JSON object (no markdown, no explanation).

Position: {position}{skills_section}

Job Description:
{jd_text}

Score each criterion (integer):
- skills (0-30): How well candidate skills match required skills
- experience (0-25): Experience level alignment
- location (0-15): Location/remote compatibility
- domain (0-15): Industry/domain relevance
- growth (0-15): Career growth alignment

Return exactly this JSON structure:
{{"skills":0,"experience":0,"location":0,"domain":0,"growth":0,"total":0,"notes":"brief summary"}}"#
    )
}

/// Try scoring via agent CLI backend.
async fn try_agent_scoring(
    _id: &str,
    position: &str,
    jd_text: &str,
    skills: Option<&str>,
    backend_override: Option<&str>,
) -> std::result::Result<(f64, String, ScoreBreakdown, String), Box<dyn std::error::Error>> {
    let cfg = app_config::load();
    let backend_name = backend_override.unwrap_or(&cfg.agent.backend);

    let backend = CliBackend::from_name(backend_name)?;
    let executor = CliExecutor::new(backend);

    let prompt = build_prompt(position, jd_text, skills);
    let timeout = Duration::from_secs(u64::from(cfg.agent.idle_timeout_secs));
    let result = executor
        .execute_capture_with_timeout(&prompt, Some(timeout))
        .await?;

    if !result.success {
        return Err(format!("agent exited with code {:?}", result.exit_code).into());
    }

    let breakdown: ScoreBreakdown = parse_json_from_output(&result.output)?;
    let total = breakdown.total;
    let notes = breakdown.notes.clone();

    Ok((total, format!("agent:{backend_name}"), breakdown, notes))
}

/// Parse JSON from agent output, handling stream-json NDJSON, markdown fences,
/// and prefix text.
fn parse_json_from_output(
    output: &str,
) -> std::result::Result<ScoreBreakdown, Box<dyn std::error::Error>> {
    // When the backend emits stream-json (NDJSON), extract the result text first
    // so that system/hook event lines don't confuse downstream parsing.
    let effective = extract_result_from_stream_json(output).unwrap_or_else(|| output.to_string());
    let trimmed = effective.trim();

    // Try direct parse first
    if let Ok(v) = serde_json::from_str::<ScoreBreakdown>(trimmed) {
        return Ok(v);
    }

    // Try extracting from markdown fences
    if let Some(json_str) = extract_fenced_json(trimmed)
        && let Ok(v) = serde_json::from_str::<ScoreBreakdown>(json_str)
    {
        return Ok(v);
    }

    // Try finding JSON object in the output (prefix text before JSON)
    if let Some(start) = trimmed.find('{')
        && let Some(end) = trimmed.rfind('}')
    {
        let candidate = &trimmed[start..=end];
        if let Ok(v) = serde_json::from_str::<ScoreBreakdown>(candidate) {
            return Ok(v);
        }
    }

    Err(format!(
        "could not parse JSON from agent output: {}",
        &trimmed[..trimmed.len().min(200)]
    )
    .into())
}

/// Extract the final result text from Claude stream-json NDJSON output.
///
/// Stream-json output contains one JSON event per line. System/hook events
/// (e.g. `{"type":"system","subtype":"hook_started",...}`) appear before the
/// actual assistant response. The result lives in a line with
/// `{"type":"result","result":"<text>"}`. Returns `None` if the output does
/// not look like NDJSON stream-json.
fn extract_result_from_stream_json(output: &str) -> Option<String> {
    let mut found_stream_event = false;
    for line in output.lines() {
        let line = line.trim();
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
            if v.get("type").and_then(serde_json::Value::as_str).is_some() {
                found_stream_event = true;
            }
            if v.get("type").and_then(serde_json::Value::as_str) == Some("result") {
                return v
                    .get("result")
                    .and_then(serde_json::Value::as_str)
                    .map(String::from);
            }
        }
    }
    // If we saw stream events but no result line, return empty to avoid
    // misinterpreting hook JSON as scoring output.
    if found_stream_event {
        Some(String::new())
    } else {
        None
    }
}

/// Extract JSON content from markdown code fences.
fn extract_fenced_json(text: &str) -> Option<&str> {
    let start_markers = ["```json\n", "```json\r\n", "```\n", "```\r\n"];
    for marker in &start_markers {
        if let Some(start) = text.find(marker) {
            let json_start = start + marker.len();
            if let Some(end) = text[json_start..].find("```") {
                return Some(text[json_start..json_start + end].trim());
            }
        }
    }
    None
}

/// Keyword-based scoring fallback when agent is unavailable.
fn keyword_scoring(jd_text: &str, skills: Option<&str>) -> (f64, ScoreBreakdown) {
    let jd_lower = jd_text.to_lowercase();
    let skill_list: Vec<&str> = skills
        .map(|s| {
            s.split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default();

    if skill_list.is_empty() {
        let breakdown = ScoreBreakdown {
            skills: 0.0,
            experience: 12.0,
            location: 8.0,
            domain: 8.0,
            growth: 8.0,
            total: 36.0,
            notes: "No candidate skills provided — baseline score only".to_string(),
        };
        return (36.0, breakdown);
    }

    let matched: Vec<&str> = skill_list
        .iter()
        .filter(|skill| jd_lower.contains(&skill.to_lowercase()))
        .copied()
        .collect();

    #[allow(clippy::cast_precision_loss)] // skill counts are tiny, no precision concern
    let match_ratio = if skill_list.is_empty() {
        0.0
    } else {
        matched.len() as f64 / skill_list.len() as f64
    };

    // Skills score: up to 30 based on match ratio
    let skills_score = (match_ratio * 30.0).round();
    // Other dimensions get baseline scores
    let experience = 12.0;
    let location = 8.0;
    let domain = 8.0;
    let growth = 8.0;
    let total = skills_score + experience + location + domain + growth;

    let notes = format!(
        "Matched {}/{} skills: {}",
        matched.len(),
        skill_list.len(),
        matched.join(", ")
    );

    let breakdown = ScoreBreakdown {
        skills: skills_score,
        experience,
        location,
        domain,
        growth,
        total,
        notes,
    };

    (total, breakdown)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_clean_json() {
        let input = r#"{"skills":25,"experience":20,"location":10,"domain":12,"growth":10,"total":77,"notes":"good fit"}"#;
        let result = parse_json_from_output(input).unwrap();
        assert!((result.total - 77.0).abs() < f64::EPSILON);
        assert_eq!(result.notes, "good fit");
    }

    #[test]
    fn test_parse_fenced_json() {
        let input = "Here is the \
                     analysis:\n```json\n{\"skills\":20,\"experience\":15,\"location\":10,\"\
                     domain\":10,\"growth\":10,\"total\":65,\"notes\":\"decent\"}\n```\n";
        let result = parse_json_from_output(input).unwrap();
        assert!((result.total - 65.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_parse_prefix_text_before_json() {
        let input = "Based on my analysis, the score is: \
                     {\"skills\":15,\"experience\":10,\"location\":5,\"domain\":8,\"growth\":7,\"\
                     total\":45,\"notes\":\"ok\"}";
        let result = parse_json_from_output(input).unwrap();
        assert!((result.total - 45.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_parse_invalid_output() {
        let input = "I cannot parse this as JSON at all";
        assert!(parse_json_from_output(input).is_err());
    }

    #[test]
    fn test_keyword_scoring_with_matches() {
        let jd = "We need Rust, Python, and TypeScript experience with Docker and Kubernetes";
        let skills = Some("Rust, Python, Go, Docker");
        let (score, breakdown) = keyword_scoring(jd, skills);
        // 3/4 skills match -> 22.5 rounded to 23 + baselines (12+8+8+8) = 59
        assert!((breakdown.skills - 23.0).abs() < f64::EPSILON);
        assert!((score - 59.0).abs() < f64::EPSILON);
        assert!(breakdown.notes.contains("3/4"));
    }

    #[test]
    fn test_keyword_scoring_no_skills() {
        let jd = "Looking for a Rust developer";
        let (score, breakdown) = keyword_scoring(jd, None);
        assert!((score - 36.0).abs() < f64::EPSILON);
        assert!(breakdown.notes.contains("No candidate skills"));
    }

    #[test]
    fn test_keyword_scoring_no_matches() {
        let jd = "Looking for a Java developer with Spring Boot";
        let skills = Some("Rust, Python, Go");
        let (score, breakdown) = keyword_scoring(jd, skills);
        assert!((breakdown.skills - 0.0).abs() < f64::EPSILON);
        assert!((score - 36.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_parse_stream_json_with_hook_events() {
        let input = r#"{"type":"system","subtype":"hook_started","hook_id":"abc","hook_name":"SessionStart:startup","hook_event":"SessionStart","uuid":"def"}
{"type":"system","subtype":"hook_completed","hook_id":"abc","hook_name":"SessionStart:startup","hook_event":"SessionStart","uuid":"def"}
{"type":"assistant","message":{"content":[{"type":"text","text":"here is the score"}]}}
{"type":"result","result":"{\"skills\":25,\"experience\":20,\"location\":10,\"domain\":12,\"growth\":10,\"total\":77,\"notes\":\"good fit\"}","subtype":"success"}"#;
        let result = parse_json_from_output(input).unwrap();
        assert!((result.total - 77.0).abs() < f64::EPSILON);
        assert_eq!(result.notes, "good fit");
    }

    #[test]
    fn test_parse_stream_json_no_result_line() {
        // Stream events present but no result line — should not misparse hook JSON
        let input = r#"{"type":"system","subtype":"hook_started","hook_id":"abc"}
{"type":"system","subtype":"hook_completed","hook_id":"abc"}"#;
        assert!(parse_json_from_output(input).is_err());
    }

    #[test]
    fn test_extract_result_from_stream_json_returns_none_for_plain_text() {
        let input = "Just some plain text output with no NDJSON";
        assert!(extract_result_from_stream_json(input).is_none());
    }
}
