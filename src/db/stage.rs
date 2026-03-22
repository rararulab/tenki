//! Stage event operations.

use snafu::ResultExt as _;

use super::Database;
use crate::{
    domain::StageEvent,
    error::{self, Result},
};

impl Database {
    /// Record a stage transition event.
    pub async fn record_stage_event(
        &self,
        application_id: &str,
        from_stage: Option<&str>,
        to_stage: &str,
        metadata: Option<&str>,
    ) -> Result<String> {
        let id = uuid::Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO stage_events (id, application_id, from_stage, to_stage, metadata)
             VALUES (?1, ?2, ?3, ?4, ?5)",
        )
        .bind(&id)
        .bind(application_id)
        .bind(from_stage)
        .bind(to_stage)
        .bind(metadata)
        .execute(self.pool())
        .await
        .context(error::SqlxSnafu)?;

        Ok(id)
    }

    /// List stage events for an application.
    pub async fn list_stage_events(&self, application_id: &str) -> Result<Vec<StageEvent>> {
        let rows: Vec<StageEvent> = sqlx::query_as(
            "SELECT id, application_id, from_stage, to_stage, occurred_at, metadata, created_at
             FROM stage_events WHERE application_id = ?1
             ORDER BY occurred_at ASC",
        )
        .bind(application_id)
        .fetch_all(self.pool())
        .await
        .context(error::SqlxSnafu)?;

        Ok(rows)
    }
}
