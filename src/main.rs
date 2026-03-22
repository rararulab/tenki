mod app_config;
mod cli;
mod db;
mod domain;
mod error;
mod paths;
mod store;

use clap::Parser;
use cli::{AppCommand, Cli, Command, InterviewCommand, StageCommand, TaskCommand};
use domain::{AddApplicationParams, ListApplicationParams, UpdateApplicationParams};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::WARN.into()),
        )
        .init();

    let json_mode = std::env::args().any(|a| a == "--json");
    if let Err(e) = run().await {
        if json_mode {
            println!(
                "{}",
                serde_json::json!({"ok": false, "error": e.to_string()})
            );
        } else {
            eprintln!("Error: {e}");
        }
        std::process::exit(1);
    }
}

#[allow(clippy::too_many_lines)]
async fn run() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let db = db::Database::open_default().await?;

    if !matches!(cli.command, Command::Init) {
        db.ensure_initialized().await?;
    }

    match cli.command {
        Command::Init => {
            db.init().await?;
            eprintln!("tenki initialized at {}", db.path().display());
        }
        Command::App(cmd) => match cmd {
            AppCommand::Add {
                company,
                position,
                jd_url,
                jd_text,
                location,
                status,
                salary,
                job_type,
                job_level,
                is_remote,
                source,
                company_url,
                notes,
                json,
            } => {
                let remote = if is_remote { Some(true) } else { None };
                let params = AddApplicationParams::builder()
                    .company(&company)
                    .position(&position)
                    .maybe_jd_url(jd_url.as_deref())
                    .maybe_jd_text(jd_text.as_deref())
                    .maybe_location(location.as_deref())
                    .status(status)
                    .maybe_salary(salary.as_deref())
                    .maybe_job_type(job_type)
                    .maybe_job_level(job_level)
                    .maybe_is_remote(remote)
                    .maybe_source(source.as_deref())
                    .maybe_company_url(company_url.as_deref())
                    .maybe_notes(notes.as_deref())
                    .build();
                cli::app::add(&db, &params, json).await?;
            }
            AppCommand::List {
                status,
                company,
                outcome,
                stage,
                source,
                json,
            } => {
                let params = ListApplicationParams::builder()
                    .maybe_status(status)
                    .maybe_company(company.as_deref())
                    .maybe_outcome(outcome)
                    .maybe_stage(stage)
                    .maybe_source(source.as_deref())
                    .build();
                cli::app::list(&db, &params, json).await?;
            }
            AppCommand::Show { id, json } => {
                cli::app::show(&db, &id, json).await?;
            }
            AppCommand::Update {
                id,
                status,
                outcome,
                stage,
                company,
                position,
                location,
                jd_url,
                jd_text,
                salary,
                job_type,
                job_level,
                is_remote,
                skills,
                source,
                notes,
                json,
            } => {
                let job_type_str = job_type.map(|v| v.as_str().to_string());
                let job_level_str = job_level.map(|v| v.as_str().to_string());
                let params = UpdateApplicationParams::builder()
                    .maybe_company(company.as_deref())
                    .maybe_position(position.as_deref())
                    .maybe_location(location.as_deref())
                    .maybe_jd_url(jd_url.as_deref())
                    .maybe_jd_text(jd_text.as_deref())
                    .maybe_salary(salary.as_deref())
                    .maybe_job_type(job_type_str.as_deref())
                    .maybe_job_level(job_level_str.as_deref())
                    .maybe_is_remote(is_remote)
                    .maybe_skills(skills.as_deref())
                    .maybe_source(source.as_deref())
                    .maybe_notes(notes.as_deref())
                    .build();
                cli::app::update(&db, &id, status, outcome, stage, &params, json).await?;
            }
            AppCommand::Delete { id, json } => {
                cli::app::delete(&db, &id, json).await?;
            }
        },
        Command::Interview(cmd) => match cmd {
            InterviewCommand::Add {
                app_id,
                round,
                r#type,
                interviewer,
                scheduled_at,
                duration_mins,
                json,
            } => {
                cli::interview::add(
                    &db,
                    &app_id,
                    round,
                    r#type,
                    interviewer.as_deref(),
                    scheduled_at.as_deref(),
                    duration_mins,
                    json,
                )
                .await?;
            }
            InterviewCommand::Update {
                id,
                status,
                outcome,
                interviewer,
                scheduled_at,
                duration_mins,
                json,
            } => {
                cli::interview::update(
                    &db,
                    &id,
                    status,
                    outcome,
                    interviewer.as_deref(),
                    scheduled_at.as_deref(),
                    duration_mins,
                    json,
                )
                .await?;
            }
            InterviewCommand::Note { id, note, json } => {
                cli::interview::note(&db, &id, &note, json).await?;
            }
            InterviewCommand::List { app_id, json } => {
                cli::interview::list(&db, &app_id, json).await?;
            }
        },
        Command::Task(cmd) => match cmd {
            TaskCommand::Add {
                app_id,
                r#type,
                title,
                due_date,
                notes,
                json,
            } => {
                cli::task::add(
                    &db,
                    &app_id,
                    r#type,
                    &title,
                    due_date.as_deref(),
                    notes.as_deref(),
                    json,
                )
                .await?;
            }
            TaskCommand::Update {
                id,
                title,
                due_date,
                notes,
                json,
            } => {
                cli::task::update(
                    &db,
                    &id,
                    title.as_deref(),
                    due_date.as_deref(),
                    notes.as_deref(),
                    json,
                )
                .await?;
            }
            TaskCommand::Done { id, json } => {
                cli::task::done(&db, &id, json).await?;
            }
            TaskCommand::Delete { id, json } => {
                cli::task::delete(&db, &id, json).await?;
            }
            TaskCommand::List { app_id, json } => {
                cli::task::list(&db, app_id.as_deref(), json).await?;
            }
        },
        Command::Stage(cmd) => match cmd {
            StageCommand::Set {
                app_id,
                stage,
                note,
                json,
            } => {
                cli::stage::set(&db, &app_id, stage, note.as_deref(), json).await?;
            }
            StageCommand::List { app_id, json } => {
                cli::stage::list(&db, &app_id, json).await?;
            }
        },
        Command::Analyze { id, json } => {
            let full_id = db.resolve_app_id(&id).await?;
            cli::analyze::run(&db, &full_id, json).await?;
        }
        Command::Tailor { id } => {
            eprintln!("tailor not yet implemented (app: {id})");
        }
        Command::Export {
            id,
            typ,
            pdf,
            output,
        } => {
            cli::export::export(&db, &id, typ, pdf, output.as_deref(), false).await?;
        }
        Command::Import { id, typ } => {
            cli::export::import(&db, &id, &typ, false).await?;
        }
        Command::Stats { json } => {
            cli::stats::stats(&db, json).await?;
        }
        Command::Timeline { id, json } => {
            cli::stats::timeline(&db, &id, json).await?;
        }
        Command::Config { action } => match action {
            cli::ConfigAction::Set { key, value } => {
                let mut cfg = app_config::load().clone();
                set_config_field(&mut cfg, &key, &value);
                app_config::save(&cfg)?;
                eprintln!("set {key} = {value}");
                println!(
                    "{}",
                    serde_json::json!({"ok": true, "action": "config_set", "key": key, "value": value})
                );
            }
            cli::ConfigAction::Get { key } => {
                let cfg = app_config::load();
                let value = get_config_field(cfg, &key);
                let display_value = value.as_deref().unwrap_or("(not set)");
                println!(
                    "{}",
                    serde_json::json!({"ok": true, "action": "config_get", "key": key, "value": display_value})
                );
            }
            cli::ConfigAction::List => {
                let cfg = app_config::load();
                let entries = config_as_map(cfg);
                let map: serde_json::Map<String, serde_json::Value> = entries
                    .into_iter()
                    .map(|(k, v)| (k, serde_json::Value::String(v)))
                    .collect();
                println!(
                    "{}",
                    serde_json::json!({"ok": true, "action": "config_list", "entries": map})
                );
            }
        },
    }

    Ok(())
}

/// Set a config field by dotted key path.
fn set_config_field(cfg: &mut app_config::AppConfig, key: &str, value: &str) {
    match key {
        "defaults.status" => cfg.defaults.status = value.to_string(),
        "defaults.source" => cfg.defaults.source = Some(value.to_string()),
        "display.date_format" => cfg.display.date_format = value.to_string(),
        _ => eprintln!("warning: unknown config key: {key}"),
    }
}

/// Get a config field by dotted key path.
fn get_config_field(cfg: &app_config::AppConfig, key: &str) -> Option<String> {
    match key {
        "defaults.status" => Some(cfg.defaults.status.clone()),
        "defaults.source" => cfg.defaults.source.clone(),
        "display.date_format" => Some(cfg.display.date_format.clone()),
        _ => None,
    }
}

/// Flatten config into key-value pairs for listing.
fn config_as_map(cfg: &app_config::AppConfig) -> Vec<(String, String)> {
    vec![
        ("defaults.status".to_string(), cfg.defaults.status.clone()),
        (
            "defaults.source".to_string(),
            cfg.defaults.source.clone().unwrap_or_default(),
        ),
        (
            "display.date_format".to_string(),
            cfg.display.date_format.clone(),
        ),
    ]
}
