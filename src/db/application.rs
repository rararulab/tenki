//! Application CRUD operations.

use snafu::ResultExt as _;

use super::Database;
use crate::{
    domain::{
        AddApplicationParams, AppStatus, Application, ApplicationRow, ListApplicationParams,
        Outcome, Stage, UpdateApplicationParams,
    },
    error::{self, Result, TenkiError},
};

/// Column list shared by `get_application` and `list_applications`.
const APPLICATION_COLUMNS: &str =
    "\
    id, company, position, jd_url, jd_text, location, status, stage, outcome, fitness_score, \
     fitness_notes, resume_typ, (resume_pdf IS NOT NULL) AS has_resume_pdf, salary, salary_min, \
     salary_max, salary_currency, job_type, is_remote, job_level, skills, experience_range, \
     source, company_url, notes, tailored_summary, tailored_headline, tailored_skills, \
     applied_at, closed_at, created_at, updated_at";

impl Database {
    /// Add a new application and record the initial status change.
    pub async fn add_application(&self, params: &AddApplicationParams<'_>) -> Result<String> {
        if let Some(url) = params.jd_url {
            crate::domain::validation::validate_url(url)?;
        }
        if let Some(url) = params.company_url {
            crate::domain::validation::validate_url(url)?;
        }

        let id = uuid::Uuid::new_v4().to_string();
        let status_str = params.status.as_str();
        let jt = params.job_type.map(|v| v.as_str().to_string());
        let jl = params.job_level.map(|v| v.as_str().to_string());

        sqlx::query(
            "INSERT INTO applications (id, company, position, jd_url, jd_text, location, status, \
             salary, job_type, job_level, is_remote, source, company_url, notes)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
        )
        .bind(&id)
        .bind(params.company)
        .bind(params.position)
        .bind(params.jd_url)
        .bind(params.jd_text)
        .bind(params.location)
        .bind(status_str)
        .bind(params.salary)
        .bind(jt.as_deref())
        .bind(jl.as_deref())
        .bind(params.is_remote)
        .bind(params.source)
        .bind(params.company_url)
        .bind(params.notes)
        .execute(self.pool())
        .await
        .context(error::SqlxSnafu)?;

        self.record_status_change(&id, "", status_str, None).await?;

        Ok(id)
    }

    /// Fetch a single application by ID.
    pub async fn get_application(&self, id: &str) -> Result<Application> {
        let sql = format!("SELECT {APPLICATION_COLUMNS} FROM applications WHERE id = ?1");

        let row: Option<ApplicationRow> = sqlx::query_as(&sql)
            .bind(id)
            .fetch_optional(self.pool())
            .await
            .context(error::SqlxSnafu)?;

        row.map(Application::from)
            .ok_or_else(|| TenkiError::ApplicationNotFound { id: id.to_string() })
    }

    /// List applications with optional filters.
    pub async fn list_applications(
        &self,
        params: &ListApplicationParams<'_>,
    ) -> Result<Vec<Application>> {
        let mut where_clause = String::from(" WHERE 1=1");
        let mut binds: Vec<String> = Vec::new();

        if let Some(s) = params.status {
            where_clause.push_str(" AND status = ?");
            binds.push(s.as_str().to_string());
        }
        if let Some(c) = params.company {
            where_clause.push_str(" AND company LIKE ?");
            binds.push(format!("%{c}%"));
        }
        if let Some(o) = params.outcome {
            where_clause.push_str(" AND outcome = ?");
            binds.push(o.as_str().to_string());
        }
        if let Some(st) = params.stage {
            where_clause.push_str(" AND stage = ?");
            binds.push(st.as_str().to_string());
        }
        if let Some(src) = params.source {
            where_clause.push_str(" AND source LIKE ?");
            binds.push(format!("%{src}%"));
        }

        let sql = format!(
            "SELECT {APPLICATION_COLUMNS} FROM applications{where_clause} ORDER BY updated_at DESC"
        );

        let mut q = sqlx::query_as::<_, ApplicationRow>(&sql);
        for b in &binds {
            q = q.bind(b);
        }

        let rows = q.fetch_all(self.pool()).await.context(error::SqlxSnafu)?;

        Ok(rows.into_iter().map(Application::from).collect())
    }

    /// Update an application's status and record the change.
    pub async fn update_application_status(&self, id: &str, new_status: AppStatus) -> Result<()> {
        let app = self.get_application(id).await?;
        let new_str = new_status.as_str();

        sqlx::query(
            "UPDATE applications SET status = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
        )
        .bind(new_str)
        .bind(id)
        .execute(self.pool())
        .await
        .context(error::SqlxSnafu)?;

        self.record_status_change(id, &app.status, new_str, None)
            .await
    }

    /// Update only the provided fields on an application.
    pub async fn update_application_fields(
        &self,
        id: &str,
        params: &UpdateApplicationParams<'_>,
    ) -> Result<()> {
        if let Some(url) = params.jd_url {
            crate::domain::validation::validate_url(url)?;
        }
        if let Some(url) = params.company_url {
            crate::domain::validation::validate_url(url)?;
        }
        if let Some(date) = params.applied_at {
            crate::domain::validation::validate_date(date)?;
        }

        // Ensure the application exists first.
        let _ = self.get_application(id).await?;

        let mut sets: Vec<String> = Vec::new();
        let mut binds: Vec<String> = Vec::new();

        macro_rules! push_field {
            ($name:expr, $val:expr) => {
                if let Some(v) = $val {
                    sets.push(format!("{} = ?", $name));
                    binds.push(v.to_string());
                }
            };
        }

        push_field!("company", params.company);
        push_field!("position", params.position);
        push_field!("location", params.location);
        push_field!("jd_url", params.jd_url);
        push_field!("jd_text", params.jd_text);
        push_field!("salary", params.salary);
        push_field!("job_type", params.job_type);
        push_field!("job_level", params.job_level);
        push_field!("skills", params.skills);
        push_field!("experience_range", params.experience_range);
        push_field!("source", params.source);
        push_field!("company_url", params.company_url);
        push_field!("notes", params.notes);
        push_field!("tailored_summary", params.tailored_summary);
        push_field!("tailored_headline", params.tailored_headline);
        push_field!("tailored_skills", params.tailored_skills);
        push_field!("applied_at", params.applied_at);

        // is_remote needs special handling (bool -> integer)
        if let Some(v) = params.is_remote {
            sets.push("is_remote = ?".to_string());
            binds.push(if v { "1".to_string() } else { "0".to_string() });
        }

        if sets.is_empty() {
            return Ok(());
        }

        sets.push("updated_at = CURRENT_TIMESTAMP".to_string());
        let sql = format!("UPDATE applications SET {} WHERE id = ?", sets.join(", "));
        binds.push(id.to_string());

        let mut q = sqlx::query(&sql);
        for b in &binds {
            q = q.bind(b);
        }
        q.execute(self.pool()).await.context(error::SqlxSnafu)?;

        Ok(())
    }

    /// Set outcome and close an application.
    pub async fn update_application_outcome(&self, id: &str, outcome: Outcome) -> Result<()> {
        let result = sqlx::query(
            "UPDATE applications SET outcome = ?1, closed_at = CURRENT_TIMESTAMP, updated_at = \
             CURRENT_TIMESTAMP WHERE id = ?2",
        )
        .bind(outcome.as_str())
        .bind(id)
        .execute(self.pool())
        .await
        .context(error::SqlxSnafu)?;

        if result.rows_affected() == 0 {
            return Err(TenkiError::ApplicationNotFound { id: id.to_string() });
        }
        Ok(())
    }

    /// Update the pipeline stage and record a stage event.
    pub async fn update_application_stage(
        &self,
        id: &str,
        stage: Stage,
        note: Option<&str>,
    ) -> Result<()> {
        let app = self.get_application(id).await?;
        let new_stage = stage.as_str();

        sqlx::query(
            "UPDATE applications SET stage = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
        )
        .bind(new_stage)
        .bind(id)
        .execute(self.pool())
        .await
        .context(error::SqlxSnafu)?;

        self.record_stage_event(id, app.stage.as_deref(), new_stage, note)
            .await?;

        Ok(())
    }

    /// Delete an application and cascade-delete interviews and status history.
    pub async fn delete_application(&self, id: &str) -> Result<()> {
        // Ensure the application exists first.
        let _ = self.get_application(id).await?;

        // With foreign_keys=ON and ON DELETE CASCADE, deleting the parent suffices.
        sqlx::query("DELETE FROM applications WHERE id = ?1")
            .bind(id)
            .execute(self.pool())
            .await
            .context(error::SqlxSnafu)?;

        Ok(())
    }

    /// Update fitness score and notes for an application.
    pub async fn update_fitness(&self, id: &str, score: f64, notes: &str) -> Result<()> {
        let result = sqlx::query(
            "UPDATE applications SET fitness_score = ?1, fitness_notes = ?2, updated_at = \
             CURRENT_TIMESTAMP WHERE id = ?3",
        )
        .bind(score)
        .bind(notes)
        .bind(id)
        .execute(self.pool())
        .await
        .context(error::SqlxSnafu)?;

        if result.rows_affected() == 0 {
            return Err(TenkiError::ApplicationNotFound { id: id.to_string() });
        }
        Ok(())
    }

    /// Update tailored resume fields for an application.
    pub async fn update_tailored(
        &self,
        id: &str,
        headline: &str,
        summary: &str,
        skills: &str,
    ) -> Result<()> {
        let result = sqlx::query(
            "UPDATE applications SET tailored_headline = ?1, tailored_summary = ?2, \
             tailored_skills = ?3, updated_at = CURRENT_TIMESTAMP WHERE id = ?4",
        )
        .bind(headline)
        .bind(summary)
        .bind(skills)
        .bind(id)
        .execute(self.pool())
        .await
        .context(error::SqlxSnafu)?;

        if result.rows_affected() == 0 {
            return Err(TenkiError::ApplicationNotFound { id: id.to_string() });
        }
        Ok(())
    }

    /// Retrieve the compiled resume PDF bytes.
    pub async fn get_resume_pdf(&self, id: &str) -> Result<Option<Vec<u8>>> {
        let row: Option<(Vec<u8>,)> = sqlx::query_as(
            "SELECT resume_pdf FROM applications WHERE id = ?1 AND resume_pdf IS NOT NULL",
        )
        .bind(id)
        .fetch_optional(self.pool())
        .await
        .context(error::SqlxSnafu)?;

        Ok(row.map(|r| r.0))
    }

    /// Retrieve the resume typst source.
    pub async fn get_resume_typ(&self, id: &str) -> Result<Option<String>> {
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT resume_typ FROM applications WHERE id = ?1 AND resume_typ IS NOT NULL",
        )
        .bind(id)
        .fetch_optional(self.pool())
        .await
        .context(error::SqlxSnafu)?;

        Ok(row.map(|r| r.0))
    }

    /// Import a discovered job as a new application if its `jd_url` is not
    /// already in the DB. Returns `Some(id)` if imported, `None` if
    /// duplicate.
    pub async fn import_discovered_job(
        &self,
        job: &crate::extractor::DiscoveredJob,
    ) -> Result<Option<String>> {
        // Dedup by jd_url if present
        if let Some(url) = &job.jd_url {
            let existing: Option<(String,)> =
                sqlx::query_as("SELECT id FROM applications WHERE jd_url = ?1")
                    .bind(url)
                    .fetch_optional(self.pool())
                    .await
                    .context(error::SqlxSnafu)?;
            if existing.is_some() {
                return Ok(None);
            }
        }

        let id = uuid::Uuid::new_v4().to_string();
        let status = "discovered";

        sqlx::query(
            "INSERT INTO applications (id, company, position, jd_url, jd_text, location, salary, \
             source, status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        )
        .bind(&id)
        .bind(&job.company)
        .bind(&job.title)
        .bind(&job.jd_url)
        .bind(&job.jd_text)
        .bind(&job.location)
        .bind(&job.salary)
        .bind(&job.source)
        .bind(status)
        .execute(self.pool())
        .await
        .context(error::SqlxSnafu)?;

        self.record_status_change(&id, "", status, None).await?;
        Ok(Some(id))
    }

    /// Store a compiled resume PDF for an application.
    pub async fn store_resume_pdf(&self, id: &str, pdf_bytes: &[u8]) -> Result<()> {
        let result = sqlx::query(
            "UPDATE applications SET resume_pdf = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
        )
        .bind(pdf_bytes)
        .bind(id)
        .execute(self.pool())
        .await
        .context(error::SqlxSnafu)?;

        if result.rows_affected() == 0 {
            return Err(TenkiError::ApplicationNotFound { id: id.to_string() });
        }
        Ok(())
    }

    /// List applications that have no `fitness_score`.
    pub async fn list_unscored(&self) -> Result<Vec<Application>> {
        let sql = format!(
            "SELECT {APPLICATION_COLUMNS} FROM applications WHERE fitness_score IS NULL AND \
             jd_text IS NOT NULL ORDER BY created_at DESC"
        );
        let rows = sqlx::query_as::<_, ApplicationRow>(&sql)
            .fetch_all(self.pool())
            .await
            .context(error::SqlxSnafu)?;
        Ok(rows.into_iter().map(Application::from).collect())
    }

    /// List applications that are scored but have no `tailored_summary`.
    pub async fn list_untailored(&self) -> Result<Vec<Application>> {
        let sql = format!(
            "SELECT {APPLICATION_COLUMNS} FROM applications WHERE fitness_score IS NOT NULL AND \
             tailored_summary IS NULL AND jd_text IS NOT NULL ORDER BY fitness_score DESC"
        );
        let rows = sqlx::query_as::<_, ApplicationRow>(&sql)
            .fetch_all(self.pool())
            .await
            .context(error::SqlxSnafu)?;
        Ok(rows.into_iter().map(Application::from).collect())
    }
}
