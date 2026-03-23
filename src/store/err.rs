//! Store-level error types.

use snafu::Snafu;

/// Convenience result type for store operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors originating from the database store layer.
#[derive(Snafu, Debug)]
#[snafu(visibility(pub))]
pub enum Error {
    #[snafu(transparent)]
    Sqlx {
        source: sqlx::Error,
        #[snafu(implicit)]
        loc:    snafu::Location,
    },

    #[snafu(transparent)]
    Migration {
        source: sqlx::migrate::MigrateError,
        #[snafu(implicit)]
        loc:    snafu::Location,
    },
}
