//! Job fit analysis using LLM or keyword fallback.

use serde::{Deserialize, Serialize};

use crate::{
    db::Database,
    domain::Application,
    error::{Result, TenkiError},
    llm::{ChatMessage, LlmClient},
};

/// Result of a job fit analysis.
#[derive(Debug, Serialize)]
pub struct AnalysisResult {
    /// The application ID that was analyzed.
    pub application_id: String,
    /// Fitness score from 0 to 100.
    pub fitness_score:  f64,
    /// Human-readable explanation of the score.
    pub reason:         String,
    /// Scoring method used: "llm" or "keyword".
    pub method:         String,
}

/// Run job fit analysis for an application.
pub async fn run(db: &Database, id: &str, json: bool) -> Result<()> {
    let full_id = db.resolve_app_id(id).await?;
    let app = db.get_application(&full_id).await?;

    let jd_text = app
        .jd_text
        .as_deref()
        .ok_or_else(|| TenkiError::MissingJdText {
            id: full_id.clone(),
        })?;

    let cfg = crate::app_config::load();
    let result = if let Some(client) = LlmClient::from_config(&cfg.llm) {
        score_with_llm(&client, &app, jd_text)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!("LLM scoring failed, falling back to keywords: {e}");
                score_with_keywords(&app, jd_text)
            })
    } else {
        tracing::info!("No LLM API key configured, using keyword scoring");
        score_with_keywords(&app, jd_text)
    };

    // Persist to DB
    db.update_fitness(&full_id, result.fitness_score, &result.reason)
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

/// Score a job application using the LLM API.
async fn score_with_llm(
    client: &LlmClient,
    app: &Application,
    jd_text: &str,
) -> Result<AnalysisResult> {
    let prompt = build_scoring_prompt(app, jd_text);
    let response = client
        .chat(vec![ChatMessage {
            role:    "user".to_string(),
            content: prompt,
        }])
        .await?;

    let parsed = parse_llm_json(&response)?;
    let score = parsed.score.clamp(0, 100);

    Ok(AnalysisResult {
        application_id: app.id.clone(),
        fitness_score:  f64::from(score),
        reason:         parsed.reason,
        method:         "llm".to_string(),
    })
}

/// Build the scoring prompt sent to the LLM.
fn build_scoring_prompt(app: &Application, jd_text: &str) -> String {
    let skills = app.skills.as_deref().unwrap_or("Not specified");
    let notes = app.notes.as_deref().unwrap_or("None");
    let location = app.location.as_deref().unwrap_or("Not specified");
    let salary = app.salary.as_deref().unwrap_or("Not specified");
    let job_type = app.job_type.as_deref().unwrap_or("Not specified");
    let job_level = app.job_level.as_deref().unwrap_or("Not specified");
    let is_remote = match app.is_remote {
        Some(true) => "Yes",
        Some(false) => "No",
        None => "Not specified",
    };

    format!(
        "You are evaluating a job listing for a candidate. Score how suitable this job is for the \
         candidate on a scale of 0-100.\n\nSCORING CRITERIA:\n- Skills match (technologies, \
         frameworks, languages): 0-30 points\n- Experience level match: 0-25 points\n- \
         Location/remote work alignment: 0-15 points\n- Industry/domain fit: 0-15 points\n- \
         Career growth potential: 0-15 points\n\nCANDIDATE PROFILE:\nPosition Applied: \
         {position}\nSkills: {skills}\nNotes: {notes}\n\nJOB LISTING:\nCompany: \
         {company}\nPosition: {position}\nLocation: {location}\nSalary: {salary}\nJob Type: \
         {job_type}\nJob Level: {job_level}\nRemote: {is_remote}\n\nJOB \
         DESCRIPTION:\n{jd_text}\n\nIMPORTANT: Respond with ONLY a valid JSON object. No \
         markdown, no code fences, no explanation outside the JSON.\n\nREQUIRED FORMAT (exactly \
         this structure):\n{{\"score\": <integer 0-100>, \"reason\": \"<1-2 sentence \
         explanation>\"}}",
        position = app.position,
        company = app.company,
    )
}

#[derive(Debug, Deserialize)]
struct ScoringResponse {
    score:  i32,
    reason: String,
}

/// Parse a JSON scoring response from LLM output, handling markdown fences
/// and surrounding text.
fn parse_llm_json(content: &str) -> Result<ScoringResponse> {
    let trimmed = content.trim();

    // Strip markdown code fences if present
    let cleaned = if trimmed.starts_with("```") {
        trimmed
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim()
    } else {
        trimmed
    };

    // Try direct parse first
    if let Ok(parsed) = serde_json::from_str::<ScoringResponse>(cleaned) {
        return Ok(parsed);
    }

    // Try to extract JSON object from surrounding text
    if let Some(start) = cleaned.find('{')
        && let Some(end) = cleaned.rfind('}')
    {
        let json_str = &cleaned[start..=end];
        if let Ok(parsed) = serde_json::from_str::<ScoringResponse>(json_str) {
            return Ok(parsed);
        }
    }

    Err(TenkiError::LlmApi {
        message: format!(
            "failed to parse LLM response as JSON: {}",
            &content[..content.len().min(200)]
        ),
    })
}

/// Score a job application using keyword matching (fallback when LLM is
/// unavailable).
fn score_with_keywords(app: &Application, jd_text: &str) -> AnalysisResult {
    let jd_lower = jd_text.to_lowercase();
    let mut score: i32 = 50;

    // Check skill overlap with JD text
    if let Some(skills) = &app.skills {
        let matched = skills
            .split(',')
            .map(str::trim)
            .filter(|skill| !skill.is_empty())
            .filter(|skill| jd_lower.contains(&skill.to_lowercase()))
            .count();

        score += i32::try_from(matched).unwrap_or(i32::MAX).saturating_mul(5);
        // Cap the total score at 80 for keyword-only scoring
        score = score.min(80);
    }

    let score = score.clamp(0, 100);
    AnalysisResult {
        application_id: app.id.clone(),
        fitness_score:  f64::from(score),
        reason:         "Scored using keyword matching (LLM API key not configured)".to_string(),
        method:         "keyword".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_clean_json() {
        let input = r#"{"score": 75, "reason": "Good match"}"#;
        let result = parse_llm_json(input).expect("should parse");
        assert_eq!(result.score, 75);
        assert_eq!(result.reason, "Good match");
    }

    #[test]
    fn parse_json_with_code_fences() {
        let input = "```json\n{\"score\": 60, \"reason\": \"Partial match\"}\n```";
        let result = parse_llm_json(input).expect("should parse");
        assert_eq!(result.score, 60);
    }

    #[test]
    fn parse_json_with_surrounding_text() {
        let input = "Here is the result: {\"score\": 80, \"reason\": \"Great fit\"} end";
        let result = parse_llm_json(input).expect("should parse");
        assert_eq!(result.score, 80);
    }

    #[test]
    fn parse_invalid_json_returns_error() {
        let input = "not json at all";
        assert!(parse_llm_json(input).is_err());
    }

    #[test]
    fn keyword_scoring_no_skills() {
        let app = make_test_app(None);
        let result = score_with_keywords(&app, "Rust developer needed");
        assert!((result.fitness_score - 50.0).abs() < f64::EPSILON);
        assert_eq!(result.method, "keyword");
    }

    #[test]
    fn keyword_scoring_with_matches() {
        let app = make_test_app(Some("Rust, Python, Go".to_string()));
        let result = score_with_keywords(&app, "We need a Rust and Python developer");
        // 50 base + 2 matches * 5 = 60
        assert!((result.fitness_score - 60.0).abs() < f64::EPSILON);
    }

    #[test]
    fn keyword_scoring_caps_at_80() {
        let app = make_test_app(Some(
            "Rust, Python, Go, Java, TypeScript, React, Node".to_string(),
        ));
        let result =
            score_with_keywords(&app, "Rust Python Go Java TypeScript React Node developer");
        assert!((result.fitness_score - 80.0).abs() < f64::EPSILON);
    }

    fn make_test_app(skills: Option<String>) -> Application {
        Application {
            id: "test-id".to_string(),
            company: "TestCo".to_string(),
            position: "Developer".to_string(),
            jd_url: None,
            jd_text: None,
            location: None,
            status: "bookmarked".to_string(),
            stage: None,
            outcome: None,
            fitness_score: None,
            fitness_notes: None,
            resume_typ: None,
            has_resume_pdf: false,
            salary: None,
            salary_min: None,
            salary_max: None,
            salary_currency: None,
            job_type: None,
            is_remote: None,
            job_level: None,
            skills,
            experience_range: None,
            source: None,
            company_url: None,
            notes: None,
            tailored_summary: None,
            tailored_headline: None,
            tailored_skills: None,
            applied_at: None,
            closed_at: None,
            created_at: "2024-01-01".to_string(),
            updated_at: "2024-01-01".to_string(),
        }
    }
}
