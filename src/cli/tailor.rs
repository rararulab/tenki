//! Tailor command — AI-powered resume tailoring via agent CLI with keyword
//! fallback.

use std::time::Duration;

use serde::{Deserialize, Serialize};
use snafu::ResultExt;

use crate::{
    agent::{CliBackend, CliExecutor},
    app_config,
    db::Database,
    error::{Result, TenkiError},
};

/// Tailoring result returned to the caller.
#[derive(Debug, Serialize)]
struct TailoringResult {
    ok:             bool,
    action:         &'static str,
    application_id: String,
    headline:       String,
    summary:        String,
    skills:         String,
    method:         String,
}

/// Parsed tailoring response from the agent.
#[derive(Debug, Deserialize, Serialize, Clone)]
struct TailoringResponse {
    #[serde(default)]
    headline: String,
    #[serde(default)]
    summary:  String,
    #[serde(default)]
    skills:   String,
}

/// Run batch tailoring on all untailored applications.
pub async fn run_batch(
    db: &Database,
    top_n: Option<usize>,
    json: bool,
    backend_override: Option<&str>,
) -> Result<()> {
    let apps = db.list_untailored().await?;
    let apps: Vec<_> = match top_n {
        Some(n) => apps.into_iter().take(n).collect(),
        None => apps,
    };

    if apps.is_empty() {
        if json {
            println!(r#"{{"ok":true,"action":"tailor_batch","tailored":0}}"#);
        } else {
            eprintln!("No untailored applications found.");
        }
        return Ok(());
    }

    eprintln!("Tailoring {} applications...", apps.len());
    let mut tailored = 0usize;

    for app in &apps {
        eprintln!("  → {} @ {} ...", app.position, app.company);
        run(db, &app.id, false, backend_override).await?;
        tailored += 1;
    }

    if json {
        println!(
            "{}",
            serde_json::to_string(&serde_json::json!({
                "ok": true, "action": "tailor_batch", "tailored": tailored
            }))
            .context(crate::error::JsonSnafu)?
        );
    } else {
        eprintln!("Batch complete: {tailored} applications tailored.");
    }

    Ok(())
}

/// Run the tailor command for a given application.
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

    // Try agent CLI tailoring first, fall back to keyword extraction
    let (headline, summary, skills, method) = match try_agent_tailoring(
        &app.position,
        jd_text,
        app.skills.as_deref(),
        app.notes.as_deref(),
        Some(&app.company),
        app.location.as_deref(),
        backend_override,
    )
    .await
    {
        Ok(result) => result,
        Err(e) => {
            eprintln!("Agent tailoring failed, falling back to keywords: {e}");
            let resp =
                keyword_tailoring(&app.position, jd_text, app.skills.as_deref(), &app.company);
            (
                resp.headline,
                resp.summary,
                resp.skills,
                "keyword".to_string(),
            )
        }
    };

    // Persist tailored content to DB
    db.update_tailored(id, &headline, &summary, &skills).await?;

    let result = TailoringResult {
        ok: true,
        action: "tailor",
        application_id: id.to_string(),
        headline,
        summary,
        skills,
        method,
    };

    if json {
        println!(
            "{}",
            serde_json::to_string(&result).context(crate::error::JsonSnafu)?
        );
    } else {
        eprintln!("Tailored resume content (method: {}):", result.method);
        eprintln!("  Headline: {}", result.headline);
        eprintln!("  Summary:  {}", result.summary);
        eprintln!("  Skills:   {}", result.skills);
    }

    Ok(())
}

/// Build the tailoring prompt for the agent CLI.
fn build_prompt(
    position: &str,
    jd_text: &str,
    skills: Option<&str>,
    notes: Option<&str>,
    company: Option<&str>,
    location: Option<&str>,
) -> String {
    let skills_line = skills.map(|s| format!("\nSkills: {s}")).unwrap_or_default();
    let notes_line = notes.map(|n| format!("\nNotes: {n}")).unwrap_or_default();
    let company_line = company
        .map(|c| format!("\nCompany: {c}"))
        .unwrap_or_default();
    let location_line = location
        .map(|l| format!("\nLocation: {l}"))
        .unwrap_or_default();

    format!(
        r#"You are tailoring a candidate's resume for a specific job. Based on the job description and candidate info, generate tailored resume content.

CANDIDATE PROFILE:
Current Position: {position}{skills_line}{notes_line}

JOB LISTING:{company_line}
Position: {position}{location_line}
Job Description:
{jd_text}

Generate tailored resume content. Respond with ONLY a valid JSON object:
{{"headline": "<professional headline tailored for this role>", "summary": "<2-3 sentence professional summary>", "skills": "<comma-separated relevant skills, prioritized for this role>"}}"#
    )
}

/// Try tailoring via agent CLI backend.
async fn try_agent_tailoring(
    position: &str,
    jd_text: &str,
    skills: Option<&str>,
    notes: Option<&str>,
    company: Option<&str>,
    location: Option<&str>,
    backend_override: Option<&str>,
) -> std::result::Result<(String, String, String, String), Box<dyn std::error::Error>> {
    let cfg = app_config::load();
    let backend_name = backend_override.unwrap_or(&cfg.agent.backend);

    let backend = CliBackend::from_name(backend_name)?;
    let executor = CliExecutor::new(backend);

    let prompt = build_prompt(position, jd_text, skills, notes, company, location);
    let timeout = Duration::from_secs(u64::from(cfg.agent.idle_timeout_secs));
    let result = executor
        .execute_capture_with_timeout(&prompt, Some(timeout))
        .await?;

    if !result.success {
        return Err(format!("agent exited with code {:?}", result.exit_code).into());
    }

    let response: TailoringResponse = parse_json_from_output(&result.output)?;

    Ok((
        response.headline,
        response.summary,
        response.skills,
        format!("agent:{backend_name}"),
    ))
}

/// Parse JSON from agent output, handling markdown fences and prefix text.
fn parse_json_from_output(
    output: &str,
) -> std::result::Result<TailoringResponse, Box<dyn std::error::Error>> {
    let trimmed = output.trim();

    // Try direct parse first
    if let Ok(v) = serde_json::from_str::<TailoringResponse>(trimmed) {
        return Ok(v);
    }

    // Try extracting from markdown fences
    if let Some(json_str) = extract_fenced_json(trimmed)
        && let Ok(v) = serde_json::from_str::<TailoringResponse>(json_str)
    {
        return Ok(v);
    }

    // Try finding JSON object in the output (prefix text before JSON)
    if let Some(start) = trimmed.find('{')
        && let Some(end) = trimmed.rfind('}')
    {
        let candidate = &trimmed[start..=end];
        if let Ok(v) = serde_json::from_str::<TailoringResponse>(candidate) {
            return Ok(v);
        }
    }

    Err(format!(
        "could not parse JSON from agent output: {}",
        &trimmed[..trimmed.len().min(200)]
    )
    .into())
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

/// Keyword-based tailoring fallback when the agent is unavailable.
fn keyword_tailoring(
    position: &str,
    jd_text: &str,
    skills: Option<&str>,
    company: &str,
) -> TailoringResponse {
    let jd_lower = jd_text.to_lowercase();

    let skill_list: Vec<&str> = skills
        .map(|s| {
            s.split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default();

    // Extract skills that appear in the JD, preserving order
    let matched_skills: Vec<&str> = skill_list
        .iter()
        .filter(|skill| jd_lower.contains(&skill.to_lowercase()))
        .copied()
        .collect();

    let skills_str = if matched_skills.is_empty() {
        skill_list.join(", ")
    } else {
        matched_skills.join(", ")
    };

    let headline = format!("{position} | {company}");

    let summary = if matched_skills.is_empty() {
        format!(
            "Experienced professional seeking {position} role at {company}. Bringing a strong \
             background and relevant expertise to drive results."
        )
    } else {
        format!(
            "Experienced professional with expertise in {} seeking {position} role at {company}. \
             Ready to leverage proven skills to deliver impact.",
            matched_skills
                .iter()
                .take(3)
                .copied()
                .collect::<Vec<_>>()
                .join(", ")
        )
    };

    TailoringResponse {
        headline,
        summary,
        skills: skills_str,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_clean_json() {
        let input = r#"{"headline":"Senior Rust Dev","summary":"Expert Rust developer.","skills":"Rust, Go, Python"}"#;
        let result = parse_json_from_output(input).unwrap();
        assert_eq!(result.headline, "Senior Rust Dev");
        assert_eq!(result.summary, "Expert Rust developer.");
        assert_eq!(result.skills, "Rust, Go, Python");
    }

    #[test]
    fn test_parse_fenced_json() {
        let input = "Here is the tailored content:\n```json\n{\"headline\":\"Backend \
                     Engineer\",\"summary\":\"Skilled backend engineer.\",\"skills\":\"Rust, \
                     Python\"}\n```\n";
        let result = parse_json_from_output(input).unwrap();
        assert_eq!(result.headline, "Backend Engineer");
    }

    #[test]
    fn test_parse_prefix_text_before_json() {
        let input = "Based on the JD: {\"headline\":\"SRE Lead\",\"summary\":\"Experienced \
                     SRE.\",\"skills\":\"Kubernetes, Docker\"}";
        let result = parse_json_from_output(input).unwrap();
        assert_eq!(result.headline, "SRE Lead");
    }

    #[test]
    fn test_parse_invalid_output() {
        let input = "I cannot parse this as JSON at all";
        assert!(parse_json_from_output(input).is_err());
    }

    #[test]
    fn test_keyword_tailoring_with_matches() {
        let jd = "We need Rust, Python, and TypeScript experience with Docker and Kubernetes";
        let skills = Some("Rust, Python, Go, Docker");
        let result = keyword_tailoring("Backend Engineer", jd, skills, "Acme Corp");
        assert_eq!(result.headline, "Backend Engineer | Acme Corp");
        assert!(result.skills.contains("Rust"));
        assert!(result.skills.contains("Python"));
        assert!(result.skills.contains("Docker"));
        // Go is not in JD, should be excluded from matched set
        assert!(!result.skills.contains("Go"));
        assert!(result.summary.contains("Acme Corp"));
    }

    #[test]
    fn test_keyword_tailoring_no_skills() {
        let jd = "Looking for a Rust developer";
        let result = keyword_tailoring("Rust Developer", jd, None, "FooCorp");
        assert_eq!(result.headline, "Rust Developer | FooCorp");
        assert!(result.skills.is_empty());
        assert!(result.summary.contains("FooCorp"));
    }

    #[test]
    fn test_keyword_tailoring_no_matches() {
        let jd = "Looking for a Java developer with Spring Boot";
        let skills = Some("Rust, Python, Go");
        let result = keyword_tailoring("Java Dev", jd, skills, "BarInc");
        // Falls back to all candidate skills when none match
        assert!(result.skills.contains("Rust"));
        assert!(result.skills.contains("Python"));
        assert!(result.skills.contains("Go"));
    }
}
