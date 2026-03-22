mod app_config;
mod cli;
mod db;
mod domain;
mod error;
mod http;
mod paths;
mod store;

use clap::Parser;
use cli::{AppCommand, Cli, Command, InterviewCommand, StageCommand, TaskCommand};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::WARN.into()),
        )
        .init();

    if let Err(e) = run().await {
        eprintln!("Error: {e}");
        println!(
            "{}",
            serde_json::json!({"ok": false, "error": e.to_string()})
        );
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
                cli::app::add(
                    &db,
                    &company,
                    &position,
                    jd_url.as_deref(),
                    jd_text.as_deref(),
                    location.as_deref(),
                    status,
                    salary.as_deref(),
                    job_type,
                    job_level,
                    is_remote,
                    source.as_deref(),
                    company_url.as_deref(),
                    notes.as_deref(),
                    json,
                )
                .await?;
            }
            AppCommand::List {
                status,
                company,
                outcome,
                stage,
                source,
                json,
            } => {
                cli::app::list(
                    &db,
                    status,
                    company.as_deref(),
                    outcome,
                    stage,
                    source.as_deref(),
                    json,
                )
                .await?;
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
                source,
                notes,
                json,
            } => {
                cli::app::update(
                    &db,
                    &id,
                    status,
                    outcome,
                    stage,
                    company.as_deref(),
                    position.as_deref(),
                    location.as_deref(),
                    jd_url.as_deref(),
                    jd_text.as_deref(),
                    salary.as_deref(),
                    job_type,
                    job_level,
                    is_remote,
                    source.as_deref(),
                    notes.as_deref(),
                    json,
                )
                .await?;
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
        Command::Analyze { id } => {
            eprintln!("analyze not yet implemented (app: {id})");
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
        "example.setting" => cfg.example.setting = value.to_string(),
        _ => eprintln!("warning: unknown config key: {key}"),
    }
}

/// Get a config field by dotted key path.
fn get_config_field(cfg: &app_config::AppConfig, key: &str) -> Option<String> {
    match key {
        "example.setting" => Some(cfg.example.setting.clone()),
        _ => None,
    }
}

/// Flatten config into key-value pairs for listing.
fn config_as_map(cfg: &app_config::AppConfig) -> Vec<(String, String)> {
    vec![("example.setting".to_string(), cfg.example.setting.clone())]
}
