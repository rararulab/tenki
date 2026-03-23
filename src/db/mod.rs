//! Domain-level database operations.

mod application;
mod interview;
mod stage;
mod stats;
mod status;
mod task;

use std::path::PathBuf;

use snafu::ResultExt as _;

use crate::{
    error::{self, Result, TenkiError},
    paths,
    store::{DBStore, DatabaseConfig},
};

// ---------------------------------------------------------------------------
// Database
// ---------------------------------------------------------------------------

/// Core handle wrapping a `SQLite` connection pool and its file path.
pub struct Database {
    store: DBStore,
    path:  PathBuf,
}

impl Database {
    /// Open the database at the default location (`~/.tenki/tenki.db`).
    pub async fn open_default() -> Result<Self> {
        let path = paths::db_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).context(error::IoSnafu)?;
        }
        Self::open_at(&path).await
    }

    /// Open the database at a specific path.
    pub async fn open_at(path: &std::path::Path) -> Result<Self> {
        let url = format!("sqlite://{}?mode=rwc", path.display());
        let config = DatabaseConfig::builder().build();
        let store = config.open(&url).await.context(error::StoreSnafu)?;
        Ok(Self {
            store,
            path: path.to_path_buf(),
        })
    }

    /// Return the database file path.
    pub fn path(&self) -> &std::path::Path { &self.path }

    /// Return a reference to the underlying `SqlitePool`.
    pub const fn pool(&self) -> &sqlx::SqlitePool { self.store.pool() }

    /// Execute the schema DDL to create all tables, then run pending
    /// migrations.
    pub async fn init(&self) -> Result<()> {
        let schema = include_str!("../schema.sql");
        sqlx::raw_sql(schema)
            .execute(self.pool())
            .await
            .context(error::SqlxSnafu)?;

        // Run migrations for existing databases
        self.run_migrations().await?;
        Ok(())
    }

    /// Apply any pending migrations to bring existing databases up to date.
    async fn run_migrations(&self) -> Result<()> {
        // Ensure migrations table exists (already in schema.sql, but
        // belt-and-suspenders)
        sqlx::raw_sql(
            "CREATE TABLE IF NOT EXISTS migrations (version INTEGER PRIMARY KEY, applied_at \
             DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP)",
        )
        .execute(self.pool())
        .await
        .context(error::SqlxSnafu)?;

        // Check which migrations have been applied
        let applied: Vec<(i64,)> =
            sqlx::query_as("SELECT version FROM migrations ORDER BY version")
                .fetch_all(self.pool())
                .await
                .context(error::SqlxSnafu)?;
        let applied_set: std::collections::HashSet<i64> =
            applied.into_iter().map(|r| r.0).collect();

        // Migration v2: enriched fields + new tables
        if !applied_set.contains(&2) {
            let migration = include_str!("../migrations/v2.sql");
            // Run each statement individually; ALTER TABLE fails if column exists, which is
            // OK
            for stmt in migration.split(';') {
                let stmt = stmt.trim();
                if stmt.is_empty() || stmt.starts_with("--") {
                    continue;
                }
                // Ignore "duplicate column" errors from ALTER TABLE
                let _ = sqlx::raw_sql(&format!("{stmt};"))
                    .execute(self.pool())
                    .await;
            }
            // Record migration
            sqlx::query("INSERT OR IGNORE INTO migrations (version) VALUES (?1)")
                .bind(2i64)
                .execute(self.pool())
                .await
                .context(error::SqlxSnafu)?;
        }

        // Migration v3: clear misleading "applied" stage on discovered/bookmarked rows
        if !applied_set.contains(&3) {
            let migration = include_str!("../migrations/v3.sql");
            sqlx::raw_sql(migration)
                .execute(self.pool())
                .await
                .context(error::SqlxSnafu)?;

            sqlx::query("INSERT OR IGNORE INTO migrations (version) VALUES (?1)")
                .bind(3i64)
                .execute(self.pool())
                .await
                .context(error::SqlxSnafu)?;
        }

        Ok(())
    }

    /// Check whether the database has been initialized (applications table
    /// exists).
    pub async fn ensure_initialized(&self) -> Result<()> {
        let row: Option<(i64,)> = sqlx::query_as(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='applications'",
        )
        .fetch_optional(self.pool())
        .await
        .context(error::SqlxSnafu)?;

        match row {
            Some((count,)) if count > 0 => Ok(()),
            _ => Err(TenkiError::DatabaseNotInitialized),
        }
    }

    // -----------------------------------------------------------------------
    // ID resolution
    // -----------------------------------------------------------------------

    /// Resolve a short ID prefix to a full application ID.
    pub async fn resolve_app_id(&self, prefix: &str) -> Result<String> {
        let pattern = format!("{prefix}%");
        let rows: Vec<(String,)> = sqlx::query_as("SELECT id FROM applications WHERE id LIKE ?1")
            .bind(&pattern)
            .fetch_all(self.pool())
            .await
            .context(error::SqlxSnafu)?;

        match rows.len() {
            0 => Err(TenkiError::ApplicationNotFound {
                id: prefix.to_string(),
            }),
            1 => rows.into_iter().next().map(|r| r.0).ok_or_else(|| {
                TenkiError::ApplicationNotFound {
                    id: prefix.to_string(),
                }
            }),
            _ => Err(TenkiError::AmbiguousId {
                prefix: prefix.to_string(),
            }),
        }
    }

    /// Resolve a short ID prefix to a full interview ID.
    pub async fn resolve_interview_id(&self, prefix: &str) -> Result<String> {
        let pattern = format!("{prefix}%");
        let rows: Vec<(String,)> = sqlx::query_as("SELECT id FROM interviews WHERE id LIKE ?1")
            .bind(&pattern)
            .fetch_all(self.pool())
            .await
            .context(error::SqlxSnafu)?;

        match rows.len() {
            0 => Err(TenkiError::InterviewNotFound {
                id: prefix.to_string(),
            }),
            1 => {
                rows.into_iter()
                    .next()
                    .map(|r| r.0)
                    .ok_or_else(|| TenkiError::InterviewNotFound {
                        id: prefix.to_string(),
                    })
            }
            _ => Err(TenkiError::AmbiguousId {
                prefix: prefix.to_string(),
            }),
        }
    }

    /// Resolve a short ID prefix to a full task ID.
    pub async fn resolve_task_id(&self, prefix: &str) -> Result<String> {
        let pattern = format!("{prefix}%");
        let rows: Vec<(String,)> = sqlx::query_as("SELECT id FROM tasks WHERE id LIKE ?1")
            .bind(&pattern)
            .fetch_all(self.pool())
            .await
            .context(error::SqlxSnafu)?;

        match rows.len() {
            0 => Err(TenkiError::TaskNotFound {
                id: prefix.to_string(),
            }),
            1 => rows
                .into_iter()
                .next()
                .map(|r| r.0)
                .ok_or_else(|| TenkiError::TaskNotFound {
                    id: prefix.to_string(),
                }),
            _ => Err(TenkiError::AmbiguousId {
                prefix: prefix.to_string(),
            }),
        }
    }
}
