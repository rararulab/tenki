use comfy_table::{Table, presets::UTF8_FULL};
use snafu::ResultExt as _;

use crate::{
    db::{Database, Stage},
    error::{self, Result},
};

pub async fn set(
    db: &Database,
    app_id: &str,
    stage: Stage,
    note: Option<&str>,
    json: bool,
) -> Result<()> {
    let full_id = db.resolve_app_id(app_id).await?;
    db.update_application_stage(&full_id, stage, note).await?;
    if json {
        let app = db.get_application(&full_id).await?;
        println!("{}", serde_json::to_string(&app).context(error::JsonSnafu)?);
    } else {
        eprintln!("Set stage to {} for application {}", stage, &full_id[..8]);
    }
    Ok(())
}

pub async fn list(db: &Database, app_id: &str, json: bool) -> Result<()> {
    let full_id = db.resolve_app_id(app_id).await?;
    let events = db.list_stage_events(&full_id).await?;
    if json {
        println!(
            "{}",
            serde_json::to_string(&events).context(error::JsonSnafu)?
        );
    } else {
        let mut table = Table::new();
        table.load_preset(UTF8_FULL);
        table.set_header(["From", "To", "Metadata", "Time"]);
        for e in &events {
            table.add_row([
                e.from_stage.as_deref().unwrap_or("—"),
                e.to_stage.as_str(),
                e.metadata.as_deref().unwrap_or("—"),
                e.occurred_at.as_str(),
            ]);
        }
        println!("{table}");
    }
    Ok(())
}
