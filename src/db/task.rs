//! Task CRUD operations.

use snafu::ResultExt as _;

use super::Database;
use crate::{
    domain::{TaskRow, TaskType},
    error::{self, Result, TenkiError},
};

impl Database {
    /// Add a task for an application.
    pub async fn add_task(
        &self,
        application_id: &str,
        task_type: TaskType,
        title: &str,
        due_date: Option<&str>,
        notes: Option<&str>,
    ) -> Result<String> {
        if let Some(date) = due_date {
            crate::domain::validation::validate_date(date)?;
        }

        // Verify the application exists.
        let _ = self.get_application(application_id).await?;

        let id = uuid::Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO tasks (id, application_id, type, title, due_date, notes)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        )
        .bind(&id)
        .bind(application_id)
        .bind(task_type.as_str())
        .bind(title)
        .bind(due_date)
        .bind(notes)
        .execute(self.pool())
        .await
        .context(error::SqlxSnafu)?;

        Ok(id)
    }

    /// Update a task's fields.
    pub async fn update_task(
        &self,
        id: &str,
        title: Option<&str>,
        due_date: Option<&str>,
        notes: Option<&str>,
    ) -> Result<()> {
        let mut sets: Vec<String> = Vec::new();
        let mut binds: Vec<String> = Vec::new();

        if let Some(v) = title {
            sets.push("title = ?".to_string());
            binds.push(v.to_string());
        }
        if let Some(v) = due_date {
            sets.push("due_date = ?".to_string());
            binds.push(v.to_string());
        }
        if let Some(v) = notes {
            sets.push("notes = ?".to_string());
            binds.push(v.to_string());
        }

        if sets.is_empty() {
            return Ok(());
        }

        let sql = format!("UPDATE tasks SET {} WHERE id = ?", sets.join(", "));
        binds.push(id.to_string());

        let mut q = sqlx::query(&sql);
        for b in &binds {
            q = q.bind(b);
        }

        let result = q.execute(self.pool()).await.context(error::SqlxSnafu)?;

        if result.rows_affected() == 0 {
            return Err(TenkiError::TaskNotFound { id: id.to_string() });
        }
        Ok(())
    }

    /// Mark a task as completed.
    pub async fn complete_task(&self, id: &str) -> Result<()> {
        let result = sqlx::query("UPDATE tasks SET is_completed = 1 WHERE id = ?1")
            .bind(id)
            .execute(self.pool())
            .await
            .context(error::SqlxSnafu)?;

        if result.rows_affected() == 0 {
            return Err(TenkiError::TaskNotFound { id: id.to_string() });
        }
        Ok(())
    }

    /// Delete a task.
    pub async fn delete_task(&self, id: &str) -> Result<()> {
        let result = sqlx::query("DELETE FROM tasks WHERE id = ?1")
            .bind(id)
            .execute(self.pool())
            .await
            .context(error::SqlxSnafu)?;

        if result.rows_affected() == 0 {
            return Err(TenkiError::TaskNotFound { id: id.to_string() });
        }
        Ok(())
    }

    /// List tasks for a specific application.
    pub async fn list_tasks(&self, application_id: &str) -> Result<Vec<TaskRow>> {
        let rows: Vec<TaskRow> = sqlx::query_as(
            "SELECT id, application_id, type, title, due_date, is_completed, notes, created_at
             FROM tasks WHERE application_id = ?1
             ORDER BY due_date ASC NULLS LAST, created_at ASC",
        )
        .bind(application_id)
        .fetch_all(self.pool())
        .await
        .context(error::SqlxSnafu)?;

        Ok(rows)
    }

    /// List all pending (incomplete) tasks across all applications, ordered by
    /// due date.
    pub async fn list_all_pending_tasks(&self) -> Result<Vec<TaskRow>> {
        let rows: Vec<TaskRow> = sqlx::query_as(
            "SELECT id, application_id, type, title, due_date, is_completed, notes, created_at
             FROM tasks WHERE is_completed = 0
             ORDER BY due_date ASC NULLS LAST, created_at ASC",
        )
        .fetch_all(self.pool())
        .await
        .context(error::SqlxSnafu)?;

        Ok(rows)
    }
}
