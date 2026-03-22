//! Application-level error types.

use snafu::Snafu;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum AppError {
    #[snafu(display("IO error: {source}"))]
    Io { source: std::io::Error },

    #[snafu(display("HTTP error: {source}"))]
    Http { source: reqwest::Error },

    #[snafu(display("JSON error: {source}"))]
    Json { source: serde_json::Error },

    #[snafu(display("config error: {message}"))]
    Config { message: String },
}

pub type Result<T> = std::result::Result<T, AppError>;
