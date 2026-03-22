use comfy_table::{Table, presets::UTF8_FULL};
use snafu::ResultExt as _;

use crate::db::{AppStatus, Database, JobType, JobLevel, Outcome, Stage};
use crate::error::{self, Result};

#[allow(clippy::too_many_arguments)]
pub async fn add(
    db: &Database,
    company: &str,
    position: &str,
    jd_url: Option<&str>,
    jd_text: Option<&str>,
    location: Option<&str>,
    status: AppStatus,
    salary: Option<&str>,
    job_type: Option<JobType>,
    job_level: Option<JobLevel>,
    is_remote: bool,
    source: Option<&str>,
    company_url: Option<&str>,
    notes: Option<&str>,
    json: bool,
) -> Result<()> {
    let remote = if is_remote { Some(true) } else { None };
    let id = db.add_application(company, position, jd_url, jd_text, location, status, salary, job_type, job_level, remote, source, company_url, notes).await?;
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
    outcome: Option<Outcome>,
    stage: Option<Stage>,
    source: Option<&str>,
    json: bool,
) -> Result<()> {
    let apps = db.list_applications(status, company, outcome, stage, source).await?;
    if json {
        println!("{}", serde_json::to_string(&apps).context(error::JsonSnafu)?);
    } else {
        let mut table = Table::new();
        table.load_preset(UTF8_FULL);
        table.set_header(["ID", "Company", "Position", "Status", "Stage", "Outcome", "Source", "Updated"]);
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
        println!("Remote:     {}", app.is_remote.map_or("—".to_string(), |v| if v { "yes" } else { "no" }.to_string()));
        println!("Skills:     {}", app.skills.as_deref().unwrap_or("—"));
        println!("Source:     {}", app.source.as_deref().unwrap_or("—"));
        println!("Company URL:{}", app.company_url.as_deref().unwrap_or("—"));
        println!("Notes:      {}", app.notes.as_deref().unwrap_or("—"));
        println!("Fitness:    {}", app.fitness_score.map_or_else(|| "—".to_string(), |s| format!("{s:.1}")));
        println!("Resume:     {}", if app.resume_typ.is_some() { "typ" } else { "—" });
        println!("PDF:        {}", if app.has_resume_pdf { "yes" } else { "no" });
        println!("Tailored Summary:  {}", app.tailored_summary.as_deref().unwrap_or("—"));
        println!("Tailored Headline: {}", app.tailored_headline.as_deref().unwrap_or("—"));
        println!("Tailored Skills:   {}", app.tailored_skills.as_deref().unwrap_or("—"));
        println!("Applied At: {}", app.applied_at.as_deref().unwrap_or("—"));
        println!("Closed At:  {}", app.closed_at.as_deref().unwrap_or("—"));
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
    outcome: Option<Outcome>,
    stage: Option<Stage>,
    company: Option<&str>,
    position: Option<&str>,
    location: Option<&str>,
    jd_url: Option<&str>,
    jd_text: Option<&str>,
    salary: Option<&str>,
    job_type: Option<JobType>,
    job_level: Option<JobLevel>,
    is_remote: Option<bool>,
    source: Option<&str>,
    notes: Option<&str>,
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
    let jt_str = job_type.map(|v| v.as_str().to_string());
    let jl_str = job_level.map(|v| v.as_str().to_string());
    db.update_application_fields(
        &full_id,
        company,
        position,
        location,
        jd_url,
        jd_text,
        salary,
        jt_str.as_deref(),
        jl_str.as_deref(),
        is_remote,
        None, // skills
        None, // experience_range
        source,
        None, // company_url
        notes,
        None, // tailored_summary
        None, // tailored_headline
        None, // tailored_skills
        None, // applied_at
    ).await?;
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
