use comfy_table::{Table, presets::UTF8_FULL};
use snafu::ResultExt as _;

use crate::db::{Database, InterviewStatus, InterviewType};
use crate::error::{self, Result};

pub async fn add(
    db: &Database,
    app_id: &str,
    round: i32,
    interview_type: InterviewType,
    interviewer: Option<&str>,
    scheduled_at: Option<&str>,
    json: bool,
) -> Result<()> {
    let full_app_id = db.resolve_app_id(app_id).await?;
    let id = db.add_interview(&full_app_id, i64::from(round), interview_type, interviewer, scheduled_at).await?;
    if json {
        let out = serde_json::json!({ "id": id });
        println!("{}", serde_json::to_string(&out).context(error::JsonSnafu)?);
    } else {
        eprintln!("Created interview {}", &id[..8]);
    }
    Ok(())
}

pub async fn update(
    db: &Database,
    id: &str,
    status: Option<InterviewStatus>,
    json: bool,
) -> Result<()> {
    let full_id = db.resolve_interview_id(id).await?;
    if let Some(s) = status {
        db.update_interview_status(&full_id, s).await?;
    }
    if json {
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
        println!("{}", serde_json::to_string(&interviews).context(error::JsonSnafu)?);
    } else {
        let mut table = Table::new();
        table.load_preset(UTF8_FULL);
        table.set_header(["ID", "Round", "Type", "Status", "Interviewer", "Scheduled"]);
        for iv in &interviews {
            table.add_row([
                &iv.id[..8],
                &iv.round.to_string(),
                &iv.r#type,
                &iv.status,
                iv.interviewer.as_deref().unwrap_or("—"),
                iv.scheduled_at.as_deref().unwrap_or("—"),
            ]);
        }
        println!("{table}");
    }
    Ok(())
}
