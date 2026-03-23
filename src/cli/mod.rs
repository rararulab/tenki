pub mod analyze;
pub mod app;
pub mod export;
pub mod interview;
pub mod stage;
pub mod stats;
pub mod tailor;
pub mod task;

use clap::{Parser, Subcommand};

use crate::domain::{
    AppStatus, InterviewOutcome, InterviewStatus, InterviewType, JobLevel, JobType, Outcome, Stage,
    TaskType,
};

/// Job application tracker — agent-native CLI.
#[derive(Parser)]
#[command(
    name = "tenki",
    version,
    about = "Job application tracker — agent-native"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

/// Top-level commands.
#[derive(Subcommand)]
pub enum Command {
    /// Initialize the tenki database
    Init,
    /// Manage job applications (add, list, show, update, delete)
    #[command(subcommand)]
    App(AppCommand),
    /// Track interviews (add, update, note, list)
    #[command(subcommand)]
    Interview(InterviewCommand),
    /// Manage tasks and reminders (add, update, done, delete, list)
    #[command(subcommand)]
    Task(TaskCommand),
    /// Track application stage transitions (set, list)
    #[command(subcommand)]
    Stage(StageCommand),
    /// Analyze job fit using AI agent
    Analyze {
        /// Application ID (8-char prefix or full UUID)
        id:      String,
        /// Output as JSON
        #[arg(long)]
        json:    bool,
        /// Override agent backend (e.g., "claude", "gemini")
        #[arg(long)]
        backend: Option<String>,
    },
    /// Tailor resume for a specific job
    Tailor {
        /// Application ID (8-char prefix or full UUID)
        id:      String,
        /// Output as JSON
        #[arg(long)]
        json:    bool,
        /// Override agent backend (e.g., "claude", "gemini")
        #[arg(long)]
        backend: Option<String>,
    },
    /// Export resume (typ or PDF)
    Export {
        /// Application ID (8-char prefix or full UUID)
        id:     String,
        /// Export as Typst source
        #[arg(long)]
        typ:    bool,
        /// Export as PDF
        #[arg(long)]
        pdf:    bool,
        /// Output file path
        #[arg(short, long)]
        output: Option<String>,
        /// Output as JSON
        #[arg(long)]
        json:   bool,
    },
    /// Import a resume typ file
    Import {
        /// Application ID (8-char prefix or full UUID)
        id:   String,
        /// Path to the Typst file to import
        #[arg(long)]
        typ:  String,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Show aggregate statistics
    Stats {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Show status change history for an application
    Timeline {
        /// Application ID (8-char prefix or full UUID)
        id:   String,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Manage configuration values
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
}

/// Application management subcommands.
#[derive(Subcommand)]
pub enum AppCommand {
    /// Add a new job application
    Add {
        /// Company name
        #[arg(long)]
        company:     String,
        /// Job position title
        #[arg(long)]
        position:    String,
        /// URL to the job description
        #[arg(long)]
        jd_url:      Option<String>,
        /// Raw job description text
        #[arg(long)]
        jd_text:     Option<String>,
        /// Job location (city, region, etc.)
        #[arg(long)]
        location:    Option<String>,
        /// Application status
        #[arg(long, value_enum, default_value_t = AppStatus::Bookmarked)]
        status:      AppStatus,
        /// Salary or compensation range
        #[arg(long)]
        salary:      Option<String>,
        /// Job type (full-time, contract, etc.)
        #[arg(long, value_enum)]
        job_type:    Option<JobType>,
        /// Seniority level
        #[arg(long, value_enum)]
        job_level:   Option<JobLevel>,
        /// Whether the position is remote
        #[arg(long)]
        is_remote:   bool,
        /// Where you found the listing
        #[arg(long)]
        source:      Option<String>,
        /// Company website URL
        #[arg(long)]
        company_url: Option<String>,
        /// Free-form notes
        #[arg(long)]
        notes:       Option<String>,
        /// Output as JSON
        #[arg(long)]
        json:        bool,
    },
    /// List applications with optional filters
    List {
        /// Filter by application status
        #[arg(long, value_enum)]
        status:  Option<AppStatus>,
        /// Filter by company name
        #[arg(long)]
        company: Option<String>,
        /// Filter by outcome
        #[arg(long, value_enum)]
        outcome: Option<Outcome>,
        /// Filter by current stage
        #[arg(long, value_enum)]
        stage:   Option<Stage>,
        /// Filter by source
        #[arg(long)]
        source:  Option<String>,
        /// Output as JSON
        #[arg(long)]
        json:    bool,
    },
    /// Show full details of an application
    Show {
        /// Application ID (8-char prefix or full UUID)
        id:   String,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Update application fields
    Update {
        /// Application ID (8-char prefix or full UUID)
        id:        String,
        /// New application status
        #[arg(long, value_enum)]
        status:    Option<AppStatus>,
        /// New outcome
        #[arg(long, value_enum)]
        outcome:   Option<Outcome>,
        /// New stage
        #[arg(long, value_enum)]
        stage:     Option<Stage>,
        /// New company name
        #[arg(long)]
        company:   Option<String>,
        /// New position title
        #[arg(long)]
        position:  Option<String>,
        /// New location
        #[arg(long)]
        location:  Option<String>,
        /// New job description URL
        #[arg(long)]
        jd_url:    Option<String>,
        /// New job description text
        #[arg(long)]
        jd_text:   Option<String>,
        /// New salary or compensation range
        #[arg(long)]
        salary:    Option<String>,
        /// New job type
        #[arg(long, value_enum)]
        job_type:  Option<JobType>,
        /// New seniority level
        #[arg(long, value_enum)]
        job_level: Option<JobLevel>,
        /// Whether the position is remote
        #[arg(long)]
        is_remote: Option<bool>,
        /// New source
        #[arg(long)]
        source:    Option<String>,
        /// Skills (comma-separated)
        #[arg(long)]
        skills:    Option<String>,
        /// New notes
        #[arg(long)]
        notes:     Option<String>,
        /// Output as JSON
        #[arg(long)]
        json:      bool,
    },
    /// Delete an application
    Delete {
        /// Application ID (8-char prefix or full UUID)
        id:   String,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

/// Interview management subcommands.
#[derive(Subcommand)]
pub enum InterviewCommand {
    /// Schedule a new interview
    Add {
        /// Application ID to associate with
        #[arg(long)]
        app_id:        String,
        /// Interview round number (1, 2, 3, ...)
        #[arg(long)]
        round:         i32,
        /// Interview format
        #[arg(long, value_enum, default_value_t = InterviewType::Other)]
        r#type:        InterviewType,
        /// Interviewer name or panel
        #[arg(long)]
        interviewer:   Option<String>,
        /// Scheduled date/time (ISO 8601)
        #[arg(long)]
        scheduled_at:  Option<String>,
        /// Expected duration in minutes
        #[arg(long)]
        duration_mins: Option<i64>,
        /// Output as JSON
        #[arg(long)]
        json:          bool,
    },
    /// Update interview status, outcome, or details
    Update {
        /// Interview ID (8-char prefix or full UUID)
        id:            String,
        /// New interview status
        #[arg(long, value_enum)]
        status:        Option<InterviewStatus>,
        /// Interview outcome
        #[arg(long, value_enum)]
        outcome:       Option<InterviewOutcome>,
        /// New interviewer name
        #[arg(long)]
        interviewer:   Option<String>,
        /// New scheduled date/time (ISO 8601)
        #[arg(long)]
        scheduled_at:  Option<String>,
        /// New duration in minutes
        #[arg(long)]
        duration_mins: Option<i64>,
        /// Output as JSON
        #[arg(long)]
        json:          bool,
    },
    /// Add a note to an interview
    Note {
        /// Interview ID (8-char prefix or full UUID)
        id:   String,
        /// Note text to append
        note: String,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// List interviews for an application
    List {
        /// Application ID to list interviews for
        app_id: String,
        /// Output as JSON
        #[arg(long)]
        json:   bool,
    },
}

/// Task management subcommands.
#[derive(Subcommand)]
pub enum TaskCommand {
    /// Create a new task or reminder
    Add {
        /// Application ID to associate with
        #[arg(long)]
        app_id:   String,
        /// Task type (todo, follow-up, etc.)
        #[arg(long, value_enum, default_value_t = TaskType::Todo)]
        r#type:   TaskType,
        /// Task title or description
        title:    String,
        /// Due date (YYYY-MM-DD)
        #[arg(long)]
        due_date: Option<String>,
        /// Additional notes
        #[arg(long)]
        notes:    Option<String>,
        /// Output as JSON
        #[arg(long)]
        json:     bool,
    },
    /// Update task details
    Update {
        /// Task ID (8-char prefix or full UUID)
        id:       String,
        /// New title
        #[arg(long)]
        title:    Option<String>,
        /// New due date (YYYY-MM-DD)
        #[arg(long)]
        due_date: Option<String>,
        /// New notes
        #[arg(long)]
        notes:    Option<String>,
        /// Output as JSON
        #[arg(long)]
        json:     bool,
    },
    /// Mark a task as completed
    Done {
        /// Task ID (8-char prefix or full UUID)
        id:   String,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Delete a task
    Delete {
        /// Task ID (8-char prefix or full UUID)
        id:   String,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// List tasks (by app or all pending)
    List {
        /// Application ID to filter by (omit for all)
        app_id: Option<String>,
        /// Output as JSON
        #[arg(long)]
        json:   bool,
    },
}

/// Stage transition subcommands.
#[derive(Subcommand)]
pub enum StageCommand {
    /// Transition an application to a new stage
    Set {
        /// Application ID to transition
        app_id: String,
        /// Target stage
        #[arg(value_enum)]
        stage:  Stage,
        /// Optional note about the transition
        #[arg(long)]
        note:   Option<String>,
        /// Output as JSON
        #[arg(long)]
        json:   bool,
    },
    /// List stage transition history
    List {
        /// Application ID to show history for
        app_id: String,
        /// Output as JSON
        #[arg(long)]
        json:   bool,
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
