//! Aggregate statistics queries.

use snafu::ResultExt as _;

use super::Database;
use crate::{
    domain::Stats,
    error::{self, Result},
};

impl Database {
    /// Get aggregate statistics across all applications.
    pub async fn stats(&self) -> Result<Stats> {
        let (total,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM applications")
            .fetch_one(self.pool())
            .await
            .context(error::SqlxSnafu)?;

        let by_status: Vec<(String, i64)> = sqlx::query_as(
            "SELECT status, COUNT(*) FROM applications GROUP BY status ORDER BY status",
        )
        .fetch_all(self.pool())
        .await
        .context(error::SqlxSnafu)?;

        let by_outcome: Vec<(String, i64)> = sqlx::query_as(
            "SELECT outcome, COUNT(*) FROM applications WHERE outcome IS NOT NULL GROUP BY \
             outcome ORDER BY outcome",
        )
        .fetch_all(self.pool())
        .await
        .context(error::SqlxSnafu)?;

        let by_stage: Vec<(String, i64)> = sqlx::query_as(
            "SELECT stage, COUNT(*) FROM applications WHERE stage IS NOT NULL GROUP BY stage \
             ORDER BY stage",
        )
        .fetch_all(self.pool())
        .await
        .context(error::SqlxSnafu)?;

        let by_source: Vec<(String, i64)> = sqlx::query_as(
            "SELECT source, COUNT(*) FROM applications WHERE source IS NOT NULL GROUP BY source \
             ORDER BY source",
        )
        .fetch_all(self.pool())
        .await
        .context(error::SqlxSnafu)?;

        let (pending_tasks,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM tasks WHERE is_completed = 0")
                .fetch_one(self.pool())
                .await
                .context(error::SqlxSnafu)?;

        let (upcoming_interviews,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM interviews WHERE status = 'scheduled' AND scheduled_at > \
             datetime('now')",
        )
        .fetch_one(self.pool())
        .await
        .context(error::SqlxSnafu)?;

        Ok(Stats {
            total,
            by_status,
            by_outcome,
            by_stage,
            by_source,
            pending_tasks,
            upcoming_interviews,
        })
    }
}
