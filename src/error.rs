//! Application-level error types.

use snafu::Snafu;

/// Top-level error type for the tenki application.
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

    #[snafu(display("invalid date format: {input} — expected YYYY-MM-DD or YYYY-MM-DDTHH:MM:SS"))]
    InvalidDate { input: String },

    #[snafu(display("invalid URL: {input} — expected http:// or https://"))]
    InvalidUrl { input: String },

    #[snafu(display("LLM analysis failed: {message}"))]
    LlmAnalysis { message: String },

    #[snafu(display(
        "missing JD text for application {id} — cannot analyze without job description"
    ))]
    MissingJdText { id: String },
}

/// Convenience result type for tenki operations.
pub type Result<T> = std::result::Result<T, TenkiError>;
