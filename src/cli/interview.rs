use comfy_table::{Table, presets::UTF8_FULL};
use snafu::ResultExt as _;

use crate::{
    db::Database,
    domain::{InterviewOutcome, InterviewStatus, InterviewType},
    error::{self, Result},
};

/// Parameters for adding an interview.
#[derive(bon::Builder)]
pub struct AddInterviewParams<'a> {
    /// Short or full application ID.
    pub app_id:         &'a str,
    /// Interview round number.
    pub round:          i32,
    /// Type of interview (e.g. phone, onsite).
    pub interview_type: InterviewType,
    /// Name of the interviewer.
    pub interviewer:    Option<&'a str>,
    /// Scheduled date/time string.
    pub scheduled_at:   Option<&'a str>,
    /// Duration in minutes.
    pub duration_mins:  Option<i64>,
    /// Whether to output JSON.
    pub json:           bool,
}

/// Add a new interview record for an application.
pub async fn add(db: &Database, params: &AddInterviewParams<'_>) -> Result<()> {
    let full_app_id = db.resolve_app_id(params.app_id).await?;
    let id = db
        .add_interview(
            &full_app_id,
            i64::from(params.round),
            params.interview_type,
            params.interviewer,
            params.scheduled_at,
            params.duration_mins,
        )
        .await?;
    if params.json {
        let out = serde_json::json!({ "id": id });
        println!("{}", serde_json::to_string(&out).context(error::JsonSnafu)?);
    } else {
        eprintln!("Created interview {}", &id[..8]);
    }
    Ok(())
}

/// Parameters for updating an interview.
#[derive(bon::Builder)]
pub struct UpdateInterviewParams<'a> {
    /// Short or full interview ID.
    pub id:            &'a str,
    /// New interview status.
    pub status:        Option<InterviewStatus>,
    /// New interview outcome.
    pub outcome:       Option<InterviewOutcome>,
    /// Updated interviewer name.
    pub interviewer:   Option<&'a str>,
    /// Updated scheduled date/time.
    pub scheduled_at:  Option<&'a str>,
    /// Updated duration in minutes.
    pub duration_mins: Option<i64>,
    /// Whether to output JSON.
    pub json:          bool,
}

/// Update an existing interview record.
pub async fn update(db: &Database, params: &UpdateInterviewParams<'_>) -> Result<()> {
    let full_id = db.resolve_interview_id(params.id).await?;
    db.update_interview(
        &full_id,
        params.status,
        params.outcome,
        params.interviewer,
        params.scheduled_at,
        params.duration_mins,
    )
    .await?;
    if params.json {
        let out = serde_json::json!({ "updated": full_id });
        println!("{}", serde_json::to_string(&out).context(error::JsonSnafu)?);
    } else {
        eprintln!("Updated interview {}", &full_id[..8]);
    }
    Ok(())
}

pub async fn note(db: &Database, id: &str, note: &str, json: bool) -> Result<()> {
    let full_id = db.resolve_interview_id(id).await?;
    db.add_interview_note(&full_id, note).await?;
    if json {
        let out = serde_json::json!({ "noted": full_id });
        println!("{}", serde_json::to_string(&out).context(error::JsonSnafu)?);
    } else {
        eprintln!("Added note to interview {}", &full_id[..8]);
    }
    Ok(())
}

pub async fn list(db: &Database, app_id: &str, json: bool) -> Result<()> {
    let full_app_id = db.resolve_app_id(app_id).await?;
    let interviews = db.list_interviews(&full_app_id).await?;
    if json {
        println!(
            "{}",
            serde_json::to_string(&interviews).context(error::JsonSnafu)?
        );
    } else {
        let mut table = Table::new();
        table.load_preset(UTF8_FULL);
        table.set_header([
            "ID",
            "Round",
            "Type",
            "Status",
            "Outcome",
            "Interviewer",
            "Scheduled",
        ]);
        for iv in &interviews {
            table.add_row([
                &iv.id[..8],
                &iv.round.to_string(),
                &iv.r#type,
                &iv.status,
                iv.outcome.as_deref().unwrap_or("—"),
                iv.interviewer.as_deref().unwrap_or("—"),
                iv.scheduled_at.as_deref().unwrap_or("—"),
            ]);
        }
        println!("{table}");
    }
    Ok(())
}
