//! Job fit analysis using AI agent CLI or keyword fallback.

use serde::{Deserialize, Serialize};

use crate::{
    agent::{CliBackend, CliExecutor},
    db::Database,
    domain::Application,
    error::{Result, TenkiError},
};

/// Result of a job fit analysis.
#[derive(Debug, Serialize)]
pub struct AnalysisResult {
    /// The application ID that was analyzed.
    pub application_id: String,
    /// Fitness score (0-100).
    pub fitness_score:  f64,
    /// Explanation of the score.
    pub reason:         String,
    /// Scoring method used ("agent" or "keyword").
    pub method:         String,
}

/// Run job fit analysis for an application.
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

    let cfg = crate::app_config::load();
    let mut agent_cfg = cfg.agent.clone();
    if let Some(b) = backend_override {
        agent_cfg.backend = b.to_string();
    }

    let result = match score_with_agent(&agent_cfg, &app, jd_text).await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Agent scoring failed, falling back to keywords: {e}");
            score_with_keywords(&app, jd_text)
        }
    };

    // Persist to DB
    db.update_fitness(id, result.fitness_score, &result.reason)
        .await?;

    if json {
        let json_str =
            serde_json::to_string_pretty(&result).expect("serialization should not fail");
        println!("{json_str}");
    } else {
        println!("Fitness Score: {:.0}/100", result.fitness_score);
        println!("Method: {}", result.method);
        println!("Reason: {}", result.reason);
    }

    Ok(())
}

/// Score using an AI agent CLI.
async fn score_with_agent(
    agent_cfg: &crate::agent::AgentConfig,
    app: &Application,
    jd_text: &str,
) -> Result<AnalysisResult> {
    let prompt = build_scoring_prompt(app, jd_text);

    let cli_backend =
        CliBackend::from_agent_config(agent_cfg).map_err(|e| TenkiError::LlmAnalysis {
            message: e.to_string(),
        })?;
    let executor = CliExecutor::new(cli_backend);

    let timeout = if agent_cfg.idle_timeout_secs > 0 {
        Some(std::time::Duration::from_secs(u64::from(
            agent_cfg.idle_timeout_secs,
        )))
    } else {
        None
    };

    let result = executor
        .execute_capture_with_timeout(&prompt, timeout)
        .await
        .map_err(|e| TenkiError::LlmAnalysis {
            message: e.to_string(),
        })?;

    if !result.success {
        return Err(TenkiError::LlmAnalysis {
            message: format!(
                "agent exited with code {:?}: {}",
                result.exit_code,
                result.stderr.lines().last().unwrap_or("unknown error")
            ),
        });
    }

    // Parse the agent output for JSON score
    let parsed = parse_score_from_output(&result.output)?;

    Ok(AnalysisResult {
        application_id: app.id.clone(),
        fitness_score:  f64::from(parsed.score.clamp(0, 100)),
        reason:         parsed.reason,
        method:         "agent".to_string(),
    })
}

/// Build the scoring prompt (inspired by job-ops scorer).
fn build_scoring_prompt(app: &Application, jd_text: &str) -> String {
    let skills = app.skills.as_deref().unwrap_or("Not specified");
    let location = app.location.as_deref().unwrap_or("Not specified");
    let salary = app.salary.as_deref().unwrap_or("Not specified");
    let job_type = app.job_type.as_deref().unwrap_or("Not specified");
    let job_level = app.job_level.as_deref().unwrap_or("Not specified");
    let is_remote = match app.is_remote {
        Some(true) => "Yes",
        Some(false) => "No",
        None => "Not specified",
    };
    let notes = app.notes.as_deref().unwrap_or("None");

    format!(
        r#"You are evaluating a job listing for a candidate. Score how suitable this job is for the candidate on a scale of 0-100.

SCORING CRITERIA:
- Skills match (technologies, frameworks, languages): 0-30 points
- Experience level match: 0-25 points
- Location/remote work alignment: 0-15 points
- Industry/domain fit: 0-15 points
- Career growth potential: 0-15 points

CANDIDATE PROFILE:
Skills: {skills}
Notes: {notes}

JOB LISTING:
Company: {company}
Position: {position}
Location: {location}
Salary: {salary}
Job Type: {job_type}
Job Level: {job_level}
Remote: {is_remote}

JOB DESCRIPTION:
{jd_text}

IMPORTANT: Respond with ONLY a valid JSON object. No markdown, no code fences, no explanation outside the JSON.

REQUIRED FORMAT (exactly this structure):
{{"score": <integer 0-100>, "reason": "<1-2 sentence explanation>"}}"#,
        company = app.company,
        position = app.position,
    )
}

#[derive(Debug, Deserialize)]
struct ScoringResponse {
    score:  i32,
    reason: String,
}

/// Parse score JSON from agent output, handling markdown fences and surrounding
/// text.
fn parse_score_from_output(output: &str) -> Result<ScoringResponse> {
    let trimmed = output.trim();

    // Try each line for JSON (agent may output progress text before the JSON)
    for line in trimmed.lines().rev() {
        let line = line.trim();
        // Skip markdown code fence lines
        if line.starts_with("```") {
            continue;
        }

        if let Some(start) = line.find('{')
            && let Some(end) = line.rfind('}')
        {
            let json_str = &line[start..=end];
            if let Ok(parsed) = serde_json::from_str::<ScoringResponse>(json_str) {
                return Ok(parsed);
            }
        }
    }

    // Try the whole output as one block
    let cleaned = trimmed
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    if let Some(start) = cleaned.find('{')
        && let Some(end) = cleaned.rfind('}')
    {
        let json_str = &cleaned[start..=end];
        if let Ok(parsed) = serde_json::from_str::<ScoringResponse>(json_str) {
            return Ok(parsed);
        }
    }

    Err(TenkiError::LlmAnalysis {
        message: format!(
            "could not parse score from agent output: {}",
            &output[..output.len().min(200)]
        ),
    })
}

/// Keyword-based scoring fallback.
fn score_with_keywords(app: &Application, jd_text: &str) -> AnalysisResult {
    let jd_lower = jd_text.to_lowercase();
    let mut score: i32 = 50;

    if let Some(skills) = &app.skills {
        for skill in skills.split(',').map(str::trim) {
            if !skill.is_empty() && jd_lower.contains(&skill.to_lowercase()) {
                score += 5;
            }
        }
    }

    let score = score.clamp(0, 100);
    AnalysisResult {
        application_id: app.id.clone(),
        fitness_score:  f64::from(score),
        reason:         "Scored using keyword matching (agent CLI not available)".to_string(),
        method:         "keyword".to_string(),
    }
}
