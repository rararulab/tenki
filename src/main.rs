mod app_config;
mod cli;
mod error;
mod http;
mod paths;

use clap::Parser;
use cli::{Cli, Command};

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

async fn run() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
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
        Command::Hello { name } => {
            let greeting = format!("Hello, {name}!");
            eprintln!("{greeting}");
            println!(
                "{}",
                serde_json::json!({"ok": true, "action": "hello", "greeting": greeting})
            );
        }
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
