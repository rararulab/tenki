//! Application-level error types.

use snafu::Snafu;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum TenkiError {
    #[snafu(display("store error: {source}"))]
    Store { source: crate::store::StoreError },

    #[snafu(display("sqlx error: {source}"))]
    Sqlx { source: sqlx::Error },

    #[snafu(display("IO error: {source}"))]
    Io { source: std::io::Error },

    #[snafu(display("JSON error: {source}"))]
    Json { source: serde_json::Error },

    #[snafu(display("config error: {message}"))]
    Config { message: String },

    #[snafu(display("database not initialized — run `tenki init` first"))]
    DatabaseNotInitialized,

    #[snafu(display("application not found: {id}"))]
    ApplicationNotFound { id: String },

    #[snafu(display("interview not found: {id}"))]
    InterviewNotFound { id: String },

    #[snafu(display("task not found: {id}"))]
    TaskNotFound { id: String },

    #[snafu(display("invalid status: {status}"))]
    InvalidStatus { status: String },

    #[snafu(display("ambiguous id prefix: {prefix} matches multiple records"))]
    AmbiguousId { prefix: String },
}

pub type Result<T> = std::result::Result<T, TenkiError>;
