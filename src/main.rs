mod agent;
mod app_config;
mod cli;
mod db;
mod domain;
mod error;
mod extractor;
mod paths;
mod pipeline;
mod store;

use clap::Parser;
use cli::{
    AppCommand, Cli, Command, InterviewCommand, PipelineCommand, StageCommand, TaskCommand,
    interview::{AddInterviewParams, UpdateInterviewParams},
};
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

async fn run() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let db = db::Database::open_default().await?;

    if !matches!(cli.command, Command::Init | Command::Doctor { .. }) {
        db.ensure_initialized().await?;
        db.migrate_sqlx().await?;
    }

    match cli.command {
        Command::Init => {
            db.init().await?;
            eprintln!("tenki initialized at {}", db.path().display());
        }
        Command::Discover {
            source,
            query,
            location,
            limit,
            json,
        } => {
            cli::discover::run(
                &db,
                source.as_deref(),
                &query,
                location.as_deref(),
                limit,
                json,
            )
            .await?;
        }
        Command::App(cmd) => handle_app(&db, cmd).await?,
        Command::Interview(cmd) => handle_interview(&db, cmd).await?,
        Command::Task(cmd) => handle_task(&db, cmd).await?,
        Command::Stage(cmd) => handle_stage(&db, cmd).await?,
        Command::Analyze {
            id,
            json,
            backend,
            unscored,
            top_n,
        } => {
            if unscored {
                cli::analyze::run_batch(&db, top_n, json, backend.as_deref()).await?;
            } else {
                let id = id.ok_or("application ID required when not using --unscored")?;
                let full_id = db.resolve_app_id(&id).await?;
                cli::analyze::run(&db, &full_id, json, backend.as_deref()).await?;
            }
        }
        Command::Tailor {
            id,
            json,
            backend,
            untailored,
            top_n,
        } => {
            if untailored {
                cli::tailor::run_batch(&db, top_n, json, backend.as_deref()).await?;
            } else {
                let id = id.ok_or("application ID required when not using --untailored")?;
                let full_id = db.resolve_app_id(&id).await?;
                cli::tailor::run(&db, &full_id, json, backend.as_deref()).await?;
            }
        }
        Command::Export {
            id,
            typ,
            pdf,
            output,
            json,
        } => {
            cli::export::export(&db, &id, typ, pdf, output.as_deref(), json).await?;
        }
        Command::Import { id, typ, json } => {
            cli::export::import(&db, &id, &typ, json).await?;
        }
        Command::Stats { json } => {
            cli::stats::stats(&db, json).await?;
        }
        Command::Timeline { id, json } => {
            cli::stats::timeline(&db, &id, json).await?;
        }
        Command::Pipeline(cmd) => handle_pipeline(&db, cmd).await?,
        Command::Doctor { json } => {
            if !cli::doctor::run(json) {
                std::process::exit(1);
            }
        }
        Command::Config { action } => handle_config(action)?,
    }

    Ok(())
}

/// Dispatch application subcommands.
async fn handle_app(
    db: &db::Database,
    cmd: AppCommand,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    match cmd {
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
            cli::app::add(db, &params, json).await?;
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
            cli::app::list(db, &params, json).await?;
        }
        AppCommand::Show { id, json } => {
            cli::app::show(db, &id, json).await?;
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
            skills,
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
                .maybe_source(source.as_deref())
                .maybe_skills(skills.as_deref())
                .maybe_notes(notes.as_deref())
                .build();
            cli::app::update(db, &id, status, outcome, stage, &params, json).await?;
        }
        AppCommand::Delete { id, json } => {
            cli::app::delete(db, &id, json).await?;
        }
    }
    Ok(())
}

/// Dispatch interview subcommands.
async fn handle_interview(
    db: &db::Database,
    cmd: InterviewCommand,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    match cmd {
        InterviewCommand::Add {
            app_id,
            round,
            r#type,
            interviewer,
            scheduled_at,
            duration_mins,
            json,
        } => {
            let params = AddInterviewParams::builder()
                .app_id(&app_id)
                .round(round)
                .interview_type(r#type)
                .maybe_interviewer(interviewer.as_deref())
                .maybe_scheduled_at(scheduled_at.as_deref())
                .maybe_duration_mins(duration_mins)
                .json(json)
                .build();
            cli::interview::add(db, &params).await?;
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
            let params = UpdateInterviewParams::builder()
                .id(&id)
                .maybe_status(status)
                .maybe_outcome(outcome)
                .maybe_interviewer(interviewer.as_deref())
                .maybe_scheduled_at(scheduled_at.as_deref())
                .maybe_duration_mins(duration_mins)
                .json(json)
                .build();
            cli::interview::update(db, &params).await?;
        }
        InterviewCommand::Note { id, note, json } => {
            cli::interview::note(db, &id, &note, json).await?;
        }
        InterviewCommand::List { app_id, json } => {
            cli::interview::list(db, &app_id, json).await?;
        }
    }
    Ok(())
}

/// Dispatch task subcommands.
async fn handle_task(
    db: &db::Database,
    cmd: TaskCommand,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    match cmd {
        TaskCommand::Add {
            app_id,
            r#type,
            title,
            due_date,
            notes,
            json,
        } => {
            cli::task::add(
                db,
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
                db,
                &id,
                title.as_deref(),
                due_date.as_deref(),
                notes.as_deref(),
                json,
            )
            .await?;
        }
        TaskCommand::Done { id, json } => {
            cli::task::done(db, &id, json).await?;
        }
        TaskCommand::Delete { id, json } => {
            cli::task::delete(db, &id, json).await?;
        }
        TaskCommand::List { app_id, json } => {
            cli::task::list(db, app_id.as_deref(), json).await?;
        }
    }
    Ok(())
}

/// Dispatch stage subcommands.
async fn handle_stage(
    db: &db::Database,
    cmd: StageCommand,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    match cmd {
        StageCommand::Set {
            app_id,
            stage,
            note,
            json,
        } => {
            cli::stage::set(db, &app_id, stage, note.as_deref(), json).await?;
        }
        StageCommand::List { app_id, json } => {
            cli::stage::list(db, &app_id, json).await?;
        }
    }
    Ok(())
}

/// Dispatch pipeline subcommands.
async fn handle_pipeline(
    db: &db::Database,
    cmd: PipelineCommand,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    match cmd {
        PipelineCommand::Run {
            query,
            sources,
            location,
            top_n,
            min_score,
            skip_tailor,
            skip_export,
            json,
        } => {
            let cfg = app_config::load();
            let resolved = resolve_pipeline_inputs(query, sources, location, cfg)?;
            let config = pipeline::PipelineConfig::builder()
                .query(resolved.query)
                .sources(resolved.sources)
                .maybe_location(resolved.location)
                .top_n(top_n)
                .min_score(min_score)
                .skip_tailor(skip_tailor)
                .skip_export(skip_export)
                .build();
            cli::pipeline::run(db, &config, json).await?;
        }
    }
    Ok(())
}

#[derive(Debug)]
struct ResolvedPipelineInputs {
    query:    String,
    sources:  Vec<String>,
    location: Option<String>,
}

fn resolve_pipeline_inputs(
    query: Option<String>,
    sources: Vec<String>,
    location: Option<String>,
    cfg: &app_config::AppConfig,
) -> error::Result<ResolvedPipelineInputs> {
    let resolved_query = query
        .or_else(|| cfg.preferences.query.clone())
        .ok_or_else(|| error::TenkiError::Config {
            message: "pipeline query missing — use --query or set preferences.query via `tenki \
                      config set`"
                .to_string(),
        })?;

    let resolved_sources = if sources.is_empty() {
        if cfg.preferences.sources.is_empty() {
            vec!["linkedin".to_string()]
        } else {
            cfg.preferences.sources.clone()
        }
    } else {
        sources
    };

    let resolved_location = location.or_else(|| cfg.preferences.location.clone());

    Ok(ResolvedPipelineInputs {
        query:    resolved_query,
        sources:  resolved_sources,
        location: resolved_location,
    })
}

/// Dispatch config subcommands.
fn handle_config(action: cli::ConfigAction) -> std::result::Result<(), Box<dyn std::error::Error>> {
    match action {
        cli::ConfigAction::Set { key, value } => {
            let mut cfg = app_config::load().clone();
            set_config_field(&mut cfg, &key, &value)?;
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
    }
    Ok(())
}

/// Set a config field by dotted key path.
fn set_config_field(
    cfg: &mut app_config::AppConfig,
    key: &str,
    value: &str,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    match key {
        "defaults.status" => cfg.defaults.status = value.to_string(),
        "defaults.source" => cfg.defaults.source = Some(value.to_string()),
        "display.date_format" => cfg.display.date_format = value.to_string(),
        "agent.backend" => cfg.agent.backend = value.to_string(),
        "agent.idle_timeout_secs" => {
            cfg.agent.idle_timeout_secs = value
                .parse()
                .map_err(|_| format!("invalid integer for idle_timeout_secs: {value}"))?;
        }
        "resume.repo_path" => cfg.resume.repo_path = Some(value.to_string()),
        "resume.build_command" => cfg.resume.build_command = Some(value.to_string()),
        "resume.output_path" => cfg.resume.output_path = Some(value.to_string()),
        "preferences.query" => cfg.preferences.query = Some(value.to_string()),
        "preferences.location" => cfg.preferences.location = Some(value.to_string()),
        "preferences.sources" => cfg.preferences.sources = parse_csv_values(value),
        _ => return Err(format!("unknown config key: {key}").into()),
    }
    Ok(())
}

/// Get a config field by dotted key path.
fn get_config_field(cfg: &app_config::AppConfig, key: &str) -> Option<String> {
    match key {
        "defaults.status" => Some(cfg.defaults.status.clone()),
        "defaults.source" => cfg.defaults.source.clone(),
        "display.date_format" => Some(cfg.display.date_format.clone()),
        "agent.backend" => Some(cfg.agent.backend.clone()),
        "agent.idle_timeout_secs" => Some(cfg.agent.idle_timeout_secs.to_string()),
        "resume.repo_path" => cfg.resume.repo_path.clone(),
        "resume.build_command" => cfg.resume.build_command.clone(),
        "resume.output_path" => cfg.resume.output_path.clone(),
        "preferences.query" => cfg.preferences.query.clone(),
        "preferences.location" => cfg.preferences.location.clone(),
        "preferences.sources" => Some(cfg.preferences.sources.join(",")),
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
        ("agent.backend".to_string(), cfg.agent.backend.clone()),
        (
            "agent.idle_timeout_secs".to_string(),
            cfg.agent.idle_timeout_secs.to_string(),
        ),
        (
            "resume.repo_path".to_string(),
            cfg.resume.repo_path.clone().unwrap_or_default(),
        ),
        (
            "resume.build_command".to_string(),
            cfg.resume.build_command.clone().unwrap_or_default(),
        ),
        (
            "resume.output_path".to_string(),
            cfg.resume.output_path.clone().unwrap_or_default(),
        ),
        (
            "preferences.query".to_string(),
            cfg.preferences.query.clone().unwrap_or_default(),
        ),
        (
            "preferences.location".to_string(),
            cfg.preferences.location.clone().unwrap_or_default(),
        ),
        (
            "preferences.sources".to_string(),
            cfg.preferences.sources.join(","),
        ),
    ]
}

fn parse_csv_values(input: &str) -> Vec<String> {
    input
        .split(',')
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_pipeline_inputs_uses_preferences_with_linkedin_fallback_source() {
        let mut cfg = app_config::AppConfig::default();
        cfg.preferences.query = Some("rust backend".to_string());
        cfg.preferences.location = Some("tokyo".to_string());

        let resolved = resolve_pipeline_inputs(None, vec![], None, &cfg).expect("resolved");
        assert_eq!(resolved.query, "rust backend");
        assert_eq!(resolved.location.as_deref(), Some("tokyo"));
        assert_eq!(resolved.sources, vec!["linkedin"]);
    }

    #[test]
    fn resolve_pipeline_inputs_pref_sources_are_used_when_cli_sources_empty() {
        let mut cfg = app_config::AppConfig::default();
        cfg.preferences.query = Some("platform engineer".to_string());
        cfg.preferences.sources = vec!["linkedin".to_string(), "boss".to_string()];

        let resolved = resolve_pipeline_inputs(None, vec![], None, &cfg).expect("resolved");
        assert_eq!(resolved.sources, vec!["linkedin", "boss"]);
    }

    #[test]
    fn resolve_pipeline_inputs_cli_overrides_preferences() {
        let mut cfg = app_config::AppConfig::default();
        cfg.preferences.query = Some("old query".to_string());
        cfg.preferences.location = Some("tokyo".to_string());
        cfg.preferences.sources = vec!["linkedin".to_string()];

        let resolved = resolve_pipeline_inputs(
            Some("new query".to_string()),
            vec!["boss".to_string()],
            Some("osaka".to_string()),
            &cfg,
        )
        .expect("resolved");

        assert_eq!(resolved.query, "new query");
        assert_eq!(resolved.location.as_deref(), Some("osaka"));
        assert_eq!(resolved.sources, vec!["boss"]);
    }

    #[test]
    fn resolve_pipeline_inputs_requires_query_from_cli_or_preferences() {
        let cfg = app_config::AppConfig::default();
        let err = resolve_pipeline_inputs(None, vec![], None, &cfg).expect_err("must fail");
        assert!(err.to_string().contains("preferences.query"));
    }

    #[test]
    fn preferences_sources_config_roundtrip() {
        let mut cfg = app_config::AppConfig::default();
        set_config_field(&mut cfg, "preferences.sources", "linkedin, boss , ,xing")
            .expect("set config");
        assert_eq!(cfg.preferences.sources, vec!["linkedin", "boss", "xing"]);
        assert_eq!(
            get_config_field(&cfg, "preferences.sources").as_deref(),
            Some("linkedin,boss,xing")
        );
    }
}
