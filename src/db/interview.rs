//! Interview CRUD operations.

use snafu::ResultExt as _;

use super::Database;
use crate::{
    domain::{InterviewOutcome, InterviewRow, InterviewStatus, InterviewType},
    error::{self, Result, TenkiError},
};

impl Database {
    /// Add an interview for an application.
    pub async fn add_interview(
        &self,
        application_id: &str,
        round: i64,
        interview_type: InterviewType,
        interviewer: Option<&str>,
        scheduled_at: Option<&str>,
        duration_mins: Option<i64>,
    ) -> Result<String> {
        if let Some(date) = scheduled_at {
            crate::domain::validation::validate_date(date)?;
        }

        // Verify the application exists.
        let _ = self.get_application(application_id).await?;

        let id = uuid::Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO interviews (id, application_id, round, type, interviewer, scheduled_at, \
             duration_mins)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        )
        .bind(&id)
        .bind(application_id)
        .bind(round)
        .bind(interview_type.as_str())
        .bind(interviewer)
        .bind(scheduled_at)
        .bind(duration_mins)
        .execute(self.pool())
        .await
        .context(error::SqlxSnafu)?;

        Ok(id)
    }

    /// Update an interview's status.
    #[allow(dead_code)]
    pub async fn update_interview_status(&self, id: &str, status: InterviewStatus) -> Result<()> {
        let result = sqlx::query("UPDATE interviews SET status = ?1 WHERE id = ?2")
            .bind(status.as_str())
            .bind(id)
            .execute(self.pool())
            .await
            .context(error::SqlxSnafu)?;

        if result.rows_affected() == 0 {
            return Err(TenkiError::InterviewNotFound { id: id.to_string() });
        }
        Ok(())
    }

    /// Update interview fields (all optional).
    pub async fn update_interview(
        &self,
        id: &str,
        status: Option<InterviewStatus>,
        outcome: Option<InterviewOutcome>,
        interviewer: Option<&str>,
        scheduled_at: Option<&str>,
        duration_mins: Option<i64>,
    ) -> Result<()> {
        let mut sets: Vec<String> = Vec::new();
        let mut binds: Vec<String> = Vec::new();

        if let Some(s) = status {
            sets.push("status = ?".to_string());
            binds.push(s.as_str().to_string());
        }
        if let Some(o) = outcome {
            sets.push("outcome = ?".to_string());
            binds.push(o.as_str().to_string());
        }
        if let Some(v) = interviewer {
            sets.push("interviewer = ?".to_string());
            binds.push(v.to_string());
        }
        if let Some(v) = scheduled_at {
            sets.push("scheduled_at = ?".to_string());
            binds.push(v.to_string());
        }
        if let Some(v) = duration_mins {
            sets.push("duration_mins = ?".to_string());
            binds.push(v.to_string());
        }

        if sets.is_empty() {
            return Ok(());
        }

        let sql = format!("UPDATE interviews SET {} WHERE id = ?", sets.join(", "));
        binds.push(id.to_string());

        let mut q = sqlx::query(&sql);
        for b in &binds {
            q = q.bind(b);
        }

        let result = q.execute(self.pool()).await.context(error::SqlxSnafu)?;

        if result.rows_affected() == 0 {
            return Err(TenkiError::InterviewNotFound { id: id.to_string() });
        }
        Ok(())
    }

    /// Append a note to an interview (separated by newlines).
    pub async fn add_interview_note(&self, id: &str, note: &str) -> Result<()> {
        let result = sqlx::query(
            "UPDATE interviews SET notes = CASE
                WHEN notes IS NULL THEN ?1
                ELSE notes || char(10) || ?1
             END
             WHERE id = ?2",
        )
        .bind(note)
        .bind(id)
        .execute(self.pool())
        .await
        .context(error::SqlxSnafu)?;

        if result.rows_affected() == 0 {
            return Err(TenkiError::InterviewNotFound { id: id.to_string() });
        }
        Ok(())
    }

    /// List interviews for an application ordered by round.
    pub async fn list_interviews(&self, application_id: &str) -> Result<Vec<InterviewRow>> {
        let rows: Vec<InterviewRow> = sqlx::query_as(
            "SELECT id, application_id, round, type, interviewer, scheduled_at,
                    status, questions, notes, outcome, duration_mins, created_at
             FROM interviews WHERE application_id = ?1
             ORDER BY round ASC",
        )
        .bind(application_id)
        .fetch_all(self.pool())
        .await
        .context(error::SqlxSnafu)?;

        Ok(rows)
    }
}
