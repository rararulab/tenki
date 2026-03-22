//! Status change history operations.

use snafu::ResultExt as _;

use super::Database;
use crate::{
    domain::StatusChange,
    error::{self, Result},
};

impl Database {
    /// Record a status change in the history table.
    pub(super) async fn record_status_change(
        &self,
        application_id: &str,
        from: &str,
        to: &str,
        note: Option<&str>,
    ) -> Result<()> {
        let id = uuid::Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO status_history (id, application_id, from_status, to_status, note)
             VALUES (?1, ?2, ?3, ?4, ?5)",
        )
        .bind(&id)
        .bind(application_id)
        .bind(from)
        .bind(to)
        .bind(note)
        .execute(self.pool())
        .await
        .context(error::SqlxSnafu)?;

        Ok(())
    }

    /// Get the status change timeline for an application.
    pub async fn get_timeline(&self, application_id: &str) -> Result<Vec<StatusChange>> {
        let rows: Vec<StatusChange> = sqlx::query_as(
            "SELECT from_status, to_status, note, created_at
             FROM status_history WHERE application_id = ?1
             ORDER BY created_at ASC",
        )
        .bind(application_id)
        .fetch_all(self.pool())
        .await
        .context(error::SqlxSnafu)?;

        Ok(rows)
    }
}
