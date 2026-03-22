use comfy_table::{Table, presets::UTF8_FULL};
use snafu::ResultExt as _;

use crate::db::Database;
use crate::error::{self, Result};

pub async fn stats(db: &Database, json: bool) -> Result<()> {
    let s = db.stats().await?;
    if json {
        println!("{}", serde_json::to_string(&s).context(error::JsonSnafu)?);
    } else {
        eprintln!("Total applications: {}", s.total);
        let mut table = Table::new();
        table.load_preset(UTF8_FULL);
        table.set_header(["Status", "Count"]);
        for (status, count) in &s.by_status {
            table.add_row([status.as_str(), &count.to_string()]);
        }
        println!("{table}");
    }
    Ok(())
}

pub async fn timeline(db: &Database, id: &str, json: bool) -> Result<()> {
    let full_id = db.resolve_app_id(id).await?;
    let changes = db.get_timeline(&full_id).await?;
    if json {
        println!("{}", serde_json::to_string(&changes).context(error::JsonSnafu)?);
    } else {
        let mut table = Table::new();
        table.load_preset(UTF8_FULL);
        table.set_header(["From", "To", "Note", "Time"]);
        for c in &changes {
            table.add_row([
                c.from_status.as_str(),
                c.to_status.as_str(),
                c.note.as_deref().unwrap_or("—"),
                c.created_at.as_str(),
            ]);
        }
        println!("{table}");
    }
    Ok(())
}
