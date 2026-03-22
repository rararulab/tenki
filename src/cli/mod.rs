pub mod app;
pub mod export;
pub mod interview;
pub mod stats;

use clap::{Parser, Subcommand};
use crate::db::{AppStatus, InterviewType, InterviewStatus};

#[derive(Parser)]
#[command(name = "tenki", about = "Job application tracker — agent-native")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    Init,
    #[command(subcommand)]
    App(AppCommand),
    #[command(subcommand)]
    Interview(InterviewCommand),
    Analyze { id: String },
    Tailor { id: String },
    Export {
        id: String,
        #[arg(long)] typ: bool,
        #[arg(long)] pdf: bool,
        #[arg(short, long)] output: Option<String>,
    },
    Import {
        id: String,
        #[arg(long)] typ: String,
    },
    Stats { #[arg(long)] json: bool },
    Timeline {
        id: String,
        #[arg(long)] json: bool,
    },

    /// Manage config values
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
}

#[derive(Subcommand)]
pub enum AppCommand {
    Add {
        #[arg(long)] company: String,
        #[arg(long)] position: String,
        #[arg(long)] jd_url: Option<String>,
        #[arg(long)] jd_text: Option<String>,
        #[arg(long)] location: Option<String>,
        #[arg(long, value_enum, default_value_t = AppStatus::Bookmarked)] status: AppStatus,
        #[arg(long)] json: bool,
    },
    List {
        #[arg(long, value_enum)] status: Option<AppStatus>,
        #[arg(long)] company: Option<String>,
        #[arg(long)] json: bool,
    },
    Show {
        id: String,
        #[arg(long)] json: bool,
    },
    Update {
        id: String,
        #[arg(long, value_enum)] status: Option<AppStatus>,
        #[arg(long)] company: Option<String>,
        #[arg(long)] position: Option<String>,
        #[arg(long)] location: Option<String>,
        #[arg(long)] jd_url: Option<String>,
        #[arg(long)] jd_text: Option<String>,
        #[arg(long)] json: bool,
    },
    Delete {
        id: String,
        #[arg(long)] json: bool,
    },
}

#[derive(Subcommand)]
pub enum InterviewCommand {
    Add {
        #[arg(long)] app_id: String,
        #[arg(long)] round: i32,
        #[arg(long, value_enum, default_value_t = InterviewType::Other)] r#type: InterviewType,
        #[arg(long)] interviewer: Option<String>,
        #[arg(long)] scheduled_at: Option<String>,
        #[arg(long)] json: bool,
    },
    Update {
        id: String,
        #[arg(long, value_enum)] status: Option<InterviewStatus>,
        #[arg(long)] interviewer: Option<String>,
        #[arg(long)] scheduled_at: Option<String>,
        #[arg(long)] json: bool,
    },
    Note {
        id: String,
        note: String,
        #[arg(long)] json: bool,
    },
    List {
        app_id: String,
        #[arg(long)] json: bool,
    },
}

/// Config management subcommands.
#[derive(Subcommand)]
pub enum ConfigAction {
    /// Set a config value
    Set {
        /// Config key (e.g. example.setting)
        key:   String,
        /// Config value
        value: String,
    },
    /// Get a config value
    Get {
        /// Config key to look up
        key: String,
    },
    /// List all config values
    List,
}
