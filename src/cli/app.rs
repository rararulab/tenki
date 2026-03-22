use comfy_table::{Table, presets::UTF8_FULL};
use snafu::ResultExt as _;

use crate::{
    db::Database,
    domain::{
        AddApplicationParams, AppStatus, ListApplicationParams, Outcome, Stage,
        UpdateApplicationParams,
    },
    error::{self, Result},
};

/// Create a new job application from the given parameters.
pub async fn add(db: &Database, params: &AddApplicationParams<'_>, json: bool) -> Result<()> {
    let id = db.add_application(params).await?;
    if json {
        let out = serde_json::json!({ "id": id });
        println!("{}", serde_json::to_string(&out).context(error::JsonSnafu)?);
    } else {
        eprintln!("Created application {}", &id[..8]);
    }
    Ok(())
}

/// List applications matching the given filters.
pub async fn list(db: &Database, params: &ListApplicationParams<'_>, json: bool) -> Result<()> {
    let apps = db.list_applications(params).await?;
    if json {
        println!(
            "{}",
            serde_json::to_string(&apps).context(error::JsonSnafu)?
        );
    } else {
        let mut table = Table::new();
        table.load_preset(UTF8_FULL);
        table.set_header([
            "ID", "Company", "Position", "Status", "Stage", "Outcome", "Source", "Updated",
        ]);
        for app in &apps {
            table.add_row([
                &app.id[..8],
                &app.company,
                &app.position,
                &app.status,
                app.stage.as_deref().unwrap_or("—"),
                app.outcome.as_deref().unwrap_or("—"),
                app.source.as_deref().unwrap_or("—"),
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
        println!("Stage:      {}", app.stage.as_deref().unwrap_or("—"));
        println!("Outcome:    {}", app.outcome.as_deref().unwrap_or("—"));
        println!("Location:   {}", app.location.as_deref().unwrap_or("—"));
        println!("JD URL:     {}", app.jd_url.as_deref().unwrap_or("—"));
        println!("Salary:     {}", app.salary.as_deref().unwrap_or("—"));
        println!("Job Type:   {}", app.job_type.as_deref().unwrap_or("—"));
        println!("Job Level:  {}", app.job_level.as_deref().unwrap_or("—"));
        println!(
            "Remote:     {}",
            match app.is_remote {
                Some(true) => "yes",
                Some(false) => "no",
                None => "—",
            }
        );
        println!("Skills:     {}", app.skills.as_deref().unwrap_or("—"));
        println!("Source:     {}", app.source.as_deref().unwrap_or("—"));
        println!("Company URL:{}", app.company_url.as_deref().unwrap_or("—"));
        println!("Notes:      {}", app.notes.as_deref().unwrap_or("—"));
        println!(
            "Fitness:    {}",
            app.fitness_score
                .map_or_else(|| "—".to_string(), |s| format!("{s:.1}"))
        );
        println!(
            "Resume:     {}",
            if app.resume_typ.is_some() {
                "typ"
            } else {
                "—"
            }
        );
        println!(
            "PDF:        {}",
            if app.has_resume_pdf { "yes" } else { "no" }
        );
        println!(
            "Tailored Summary:  {}",
            app.tailored_summary.as_deref().unwrap_or("—")
        );
        println!(
            "Tailored Headline: {}",
            app.tailored_headline.as_deref().unwrap_or("—")
        );
        println!(
            "Tailored Skills:   {}",
            app.tailored_skills.as_deref().unwrap_or("—")
        );
        println!("Applied At: {}", app.applied_at.as_deref().unwrap_or("—"));
        println!("Closed At:  {}", app.closed_at.as_deref().unwrap_or("—"));
        println!("Created:    {}", app.created_at);
        println!("Updated:    {}", app.updated_at);
    }
    Ok(())
}

/// Update an existing application's status, outcome, stage, and/or fields.
pub async fn update(
    db: &Database,
    id: &str,
    status: Option<AppStatus>,
    outcome: Option<Outcome>,
    stage: Option<Stage>,
    params: &UpdateApplicationParams<'_>,
    json: bool,
) -> Result<()> {
    let full_id = db.resolve_app_id(id).await?;
    if let Some(s) = status {
        db.update_application_status(&full_id, s).await?;
    }
    if let Some(o) = outcome {
        db.update_application_outcome(&full_id, o).await?;
    }
    if let Some(st) = stage {
        db.update_application_stage(&full_id, st, None).await?;
    }
    db.update_application_fields(&full_id, params).await?;
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
