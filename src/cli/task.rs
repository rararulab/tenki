use comfy_table::{Table, presets::UTF8_FULL};
use snafu::ResultExt as _;

use crate::{
    db::Database,
    domain::TaskType,
    error::{self, Result},
};

pub async fn add(
    db: &Database,
    app_id: &str,
    task_type: TaskType,
    title: &str,
    due_date: Option<&str>,
    notes: Option<&str>,
    json: bool,
) -> Result<()> {
    let full_app_id = db.resolve_app_id(app_id).await?;
    let id = db
        .add_task(&full_app_id, task_type, title, due_date, notes)
        .await?;
    if json {
        let out = serde_json::json!({ "id": id });
        println!("{}", serde_json::to_string(&out).context(error::JsonSnafu)?);
    } else {
        eprintln!("Created task {}", &id[..8]);
    }
    Ok(())
}

pub async fn update(
    db: &Database,
    id: &str,
    title: Option<&str>,
    due_date: Option<&str>,
    notes: Option<&str>,
    json: bool,
) -> Result<()> {
    let full_id = db.resolve_task_id(id).await?;
    db.update_task(&full_id, title, due_date, notes).await?;
    if json {
        let out = serde_json::json!({ "updated": full_id });
        println!("{}", serde_json::to_string(&out).context(error::JsonSnafu)?);
    } else {
        eprintln!("Updated task {}", &full_id[..8]);
    }
    Ok(())
}

pub async fn done(db: &Database, id: &str, json: bool) -> Result<()> {
    let full_id = db.resolve_task_id(id).await?;
    db.complete_task(&full_id).await?;
    if json {
        let out = serde_json::json!({ "completed": full_id });
        println!("{}", serde_json::to_string(&out).context(error::JsonSnafu)?);
    } else {
        eprintln!("Completed task {}", &full_id[..8]);
    }
    Ok(())
}

pub async fn delete(db: &Database, id: &str, json: bool) -> Result<()> {
    let full_id = db.resolve_task_id(id).await?;
    db.delete_task(&full_id).await?;
    if json {
        let out = serde_json::json!({ "deleted": full_id });
        println!("{}", serde_json::to_string(&out).context(error::JsonSnafu)?);
    } else {
        eprintln!("Deleted task {}", &full_id[..8]);
    }
    Ok(())
}

pub async fn list(db: &Database, app_id: Option<&str>, json: bool) -> Result<()> {
    let tasks = match app_id {
        Some(aid) => {
            let full_id = db.resolve_app_id(aid).await?;
            db.list_tasks(&full_id).await?
        }
        None => db.list_all_pending_tasks().await?,
    };
    if json {
        println!(
            "{}",
            serde_json::to_string(&tasks).context(error::JsonSnafu)?
        );
    } else {
        let mut table = Table::new();
        table.load_preset(UTF8_FULL);
        table.set_header(["ID", "App", "Type", "Title", "Due", "Done"]);
        for t in &tasks {
            table.add_row([
                &t.id[..8],
                &t.application_id[..8],
                &t.r#type,
                &t.title,
                t.due_date.as_deref().unwrap_or("—"),
                if t.is_completed { "yes" } else { "no" },
            ]);
        }
        println!("{table}");
    }
    Ok(())
}
