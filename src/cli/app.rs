use comfy_table::{Table, presets::UTF8_FULL};
use snafu::ResultExt as _;

use crate::db::{AppStatus, Database};
use crate::error::{self, Result};

pub async fn add(
    db: &Database,
    company: &str,
    position: &str,
    jd_url: Option<&str>,
    jd_text: Option<&str>,
    location: Option<&str>,
    status: AppStatus,
    json: bool,
) -> Result<()> {
    let id = db.add_application(company, position, jd_url, jd_text, location, status).await?;
    if json {
        let out = serde_json::json!({ "id": id });
        println!("{}", serde_json::to_string(&out).context(error::JsonSnafu)?);
    } else {
        eprintln!("Created application {}", &id[..8]);
    }
    Ok(())
}

pub async fn list(
    db: &Database,
    status: Option<AppStatus>,
    company: Option<&str>,
    json: bool,
) -> Result<()> {
    let apps = db.list_applications(status, company).await?;
    if json {
        println!("{}", serde_json::to_string(&apps).context(error::JsonSnafu)?);
    } else {
        let mut table = Table::new();
        table.load_preset(UTF8_FULL);
        table.set_header(["ID", "Company", "Position", "Status", "Location", "Fitness", "Updated"]);
        for app in &apps {
            table.add_row([
                &app.id[..8],
                &app.company,
                &app.position,
                &app.status,
                app.location.as_deref().unwrap_or("—"),
                &app.fitness_score.map_or_else(|| "—".to_string(), |s| format!("{s:.1}")),
                &app.updated_at,
            ]);
        }
        println!("{table}");
    }
    Ok(())
}

pub async fn show(db: &Database, id: &str, json: bool) -> Result<()> {
    let full_id = db.resolve_app_id(id).await?;
    let app = db.get_application(&full_id).await?;
    if json {
        println!("{}", serde_json::to_string(&app).context(error::JsonSnafu)?);
    } else {
        eprintln!("Application {}", &app.id[..8]);
        println!("Company:    {}", app.company);
        println!("Position:   {}", app.position);
        println!("Status:     {}", app.status);
        println!("Location:   {}", app.location.as_deref().unwrap_or("—"));
        println!("JD URL:     {}", app.jd_url.as_deref().unwrap_or("—"));
        println!("Fitness:    {}", app.fitness_score.map_or_else(|| "—".to_string(), |s| format!("{s:.1}")));
        println!("Resume:     {}", if app.resume_typ.is_some() { "typ" } else { "—" });
        println!("PDF:        {}", if app.has_resume_pdf { "yes" } else { "no" });
        println!("Created:    {}", app.created_at);
        println!("Updated:    {}", app.updated_at);
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn update(
    db: &Database,
    id: &str,
    status: Option<AppStatus>,
    company: Option<&str>,
    position: Option<&str>,
    location: Option<&str>,
    jd_url: Option<&str>,
    jd_text: Option<&str>,
    json: bool,
) -> Result<()> {
    let full_id = db.resolve_app_id(id).await?;
    if let Some(s) = status {
        db.update_application_status(&full_id, s).await?;
    }
    db.update_application_fields(&full_id, company, position, location, jd_url, jd_text).await?;
    if json {
        let app = db.get_application(&full_id).await?;
        println!("{}", serde_json::to_string(&app).context(error::JsonSnafu)?);
    } else {
        eprintln!("Updated application {}", &full_id[..8]);
    }
    Ok(())
}

pub async fn delete(db: &Database, id: &str, json: bool) -> Result<()> {
    let full_id = db.resolve_app_id(id).await?;
    db.delete_application(&full_id).await?;
    if json {
        let out = serde_json::json!({ "deleted": full_id });
        println!("{}", serde_json::to_string(&out).context(error::JsonSnafu)?);
    } else {
        eprintln!("Deleted application {}", &full_id[..8]);
    }
    Ok(())
}
