//! Domain-level database operations.

use std::path::PathBuf;

use snafu::ResultExt as _;
use sqlx::SqlitePool;

use crate::{
    domain::{
        AddApplicationParams, AppStatus, Application, InterviewOutcome, InterviewRow,
        InterviewStatus, InterviewType, ListApplicationParams, Outcome, Stage, StageEvent, Stats,
        StatusChange, TaskRow, TaskType, UpdateApplicationParams,
    },
    error::{self, Result, TenkiError},
    paths,
    store::{DBStore, DatabaseConfig},
};

// ---------------------------------------------------------------------------
// Database
// ---------------------------------------------------------------------------

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
    pub const fn pool(&self) -> &SqlitePool { self.store.pool() }

    /// Execute the schema DDL to create all tables, then run pending
    /// migrations.
    pub async fn init(&self) -> Result<()> {
        let schema = include_str!("schema.sql");
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
            let migration = include_str!("migrations/v2.sql");
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
            1 => Ok(rows.into_iter().next().expect("checked len").0),
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
            1 => Ok(rows.into_iter().next().expect("checked len").0),
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
            1 => Ok(rows.into_iter().next().expect("checked len").0),
            _ => Err(TenkiError::AmbiguousId {
                prefix: prefix.to_string(),
            }),
        }
    }

    // -----------------------------------------------------------------------
    // Applications
    // -----------------------------------------------------------------------

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
    #[allow(clippy::type_complexity, clippy::too_many_lines)]
    pub async fn get_application(&self, id: &str) -> Result<Application> {
        let row: Option<(
            String,         // 0  id
            String,         // 1  company
            String,         // 2  position
            Option<String>, // 3  jd_url
            Option<String>, // 4  jd_text
            Option<String>, // 5  location
            String,         // 6  status
            Option<String>, // 7  stage
            Option<String>, // 8  outcome
            Option<f64>,    // 9  fitness_score
            Option<String>, // 10 fitness_notes
            Option<String>, // 11 resume_typ
            bool,           // 12 has_resume_pdf
            Option<String>, // 13 salary
            Option<f64>,    // 14 salary_min
            Option<f64>,    // 15 salary_max
        )> = sqlx::query_as(
            "SELECT id, company, position, jd_url, jd_text, location, status,
                    stage, outcome,
                    fitness_score, fitness_notes, resume_typ,
                    (resume_pdf IS NOT NULL) AS has_resume_pdf,
                    salary, salary_min, salary_max
             FROM applications WHERE id = ?1",
        )
        .bind(id)
        .fetch_optional(self.pool())
        .await
        .context(error::SqlxSnafu)?;

        let r = row.ok_or_else(|| TenkiError::ApplicationNotFound { id: id.to_string() })?;

        // Fetch remaining columns in a second query to avoid giant tuple
        let row2: Option<(
            Option<String>, // 0  salary_currency
            Option<String>, // 1  job_type
            Option<i64>,    // 2  is_remote (INTEGER)
            Option<String>, // 3  job_level
            Option<String>, // 4  skills
            Option<String>, // 5  experience_range
            Option<String>, // 6  source
            Option<String>, // 7  company_url
            Option<String>, // 8  notes
            Option<String>, // 9  tailored_summary
            Option<String>, // 10 tailored_headline
            Option<String>, // 11 tailored_skills
            Option<String>, // 12 applied_at
            Option<String>, // 13 closed_at
            String,         // 14 created_at
            String,         // 15 updated_at
        )> = sqlx::query_as(
            "SELECT salary_currency, job_type, is_remote, job_level, skills,
                    experience_range, source, company_url, notes,
                    tailored_summary, tailored_headline, tailored_skills,
                    applied_at, closed_at, created_at, updated_at
             FROM applications WHERE id = ?1",
        )
        .bind(id)
        .fetch_optional(self.pool())
        .await
        .context(error::SqlxSnafu)?;

        let r2 = row2.expect("row must exist since first query succeeded");

        Ok(Application {
            id:                r.0,
            company:           r.1,
            position:          r.2,
            jd_url:            r.3,
            jd_text:           r.4,
            location:          r.5,
            status:            r.6,
            stage:             r.7,
            outcome:           r.8,
            fitness_score:     r.9,
            fitness_notes:     r.10,
            resume_typ:        r.11,
            has_resume_pdf:    r.12,
            salary:            r.13,
            salary_min:        r.14,
            salary_max:        r.15,
            salary_currency:   r2.0,
            job_type:          r2.1,
            is_remote:         r2.2.map(|v| v != 0),
            job_level:         r2.3,
            skills:            r2.4,
            experience_range:  r2.5,
            source:            r2.6,
            company_url:       r2.7,
            notes:             r2.8,
            tailored_summary:  r2.9,
            tailored_headline: r2.10,
            tailored_skills:   r2.11,
            applied_at:        r2.12,
            closed_at:         r2.13,
            created_at:        r2.14,
            updated_at:        r2.15,
        })
    }

    /// List applications with optional filters.
    #[allow(clippy::too_many_lines)]
    pub async fn list_applications(
        &self,
        params: &ListApplicationParams<'_>,
    ) -> Result<Vec<Application>> {
        // Build first query (first 16 columns)
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

        let sql1 = format!(
            "SELECT id, company, position, jd_url, jd_text, location, status,
                    stage, outcome,
                    fitness_score, fitness_notes, resume_typ,
                    (resume_pdf IS NOT NULL) AS has_resume_pdf,
                    salary, salary_min, salary_max
             FROM applications{where_clause} ORDER BY updated_at DESC"
        );

        let mut q1 = sqlx::query_as::<
            _,
            (
                String,
                String,
                String,
                Option<String>,
                Option<String>,
                Option<String>,
                String,
                Option<String>,
                Option<String>,
                Option<f64>,
                Option<String>,
                Option<String>,
                bool,
                Option<String>,
                Option<f64>,
                Option<f64>,
            ),
        >(&sql1);

        for b in &binds {
            q1 = q1.bind(b);
        }

        let rows1 = q1.fetch_all(self.pool()).await.context(error::SqlxSnafu)?;

        // Collect IDs for second query
        if rows1.is_empty() {
            return Ok(Vec::new());
        }

        let sql2 = format!(
            "SELECT salary_currency, job_type, is_remote, job_level, skills,
                    experience_range, source, company_url, notes,
                    tailored_summary, tailored_headline, tailored_skills,
                    applied_at, closed_at, created_at, updated_at
             FROM applications{where_clause} ORDER BY updated_at DESC"
        );

        let mut q2 = sqlx::query_as::<
            _,
            (
                Option<String>,
                Option<String>,
                Option<i64>,
                Option<String>,
                Option<String>,
                Option<String>,
                Option<String>,
                Option<String>,
                Option<String>,
                Option<String>,
                Option<String>,
                Option<String>,
                Option<String>,
                Option<String>,
                String,
                String,
            ),
        >(&sql2);

        for b in &binds {
            q2 = q2.bind(b);
        }

        let rows2 = q2.fetch_all(self.pool()).await.context(error::SqlxSnafu)?;

        Ok(rows1
            .into_iter()
            .zip(rows2)
            .map(|(r, r2)| Application {
                id:                r.0,
                company:           r.1,
                position:          r.2,
                jd_url:            r.3,
                jd_text:           r.4,
                location:          r.5,
                status:            r.6,
                stage:             r.7,
                outcome:           r.8,
                fitness_score:     r.9,
                fitness_notes:     r.10,
                resume_typ:        r.11,
                has_resume_pdf:    r.12,
                salary:            r.13,
                salary_min:        r.14,
                salary_max:        r.15,
                salary_currency:   r2.0,
                job_type:          r2.1,
                is_remote:         r2.2.map(|v| v != 0),
                job_level:         r2.3,
                skills:            r2.4,
                experience_range:  r2.5,
                source:            r2.6,
                company_url:       r2.7,
                notes:             r2.8,
                tailored_summary:  r2.9,
                tailored_headline: r2.10,
                tailored_skills:   r2.11,
                applied_at:        r2.12,
                closed_at:         r2.13,
                created_at:        r2.14,
                updated_at:        r2.15,
            })
            .collect())
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
    #[allow(dead_code)]
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

    /// Update resume typst source and compiled PDF.
    #[allow(dead_code)]
    pub async fn update_resume(&self, id: &str, typ: &str, pdf_bytes: &[u8]) -> Result<()> {
        let result = sqlx::query(
            "UPDATE applications SET resume_typ = ?1, resume_pdf = ?2, updated_at = \
             CURRENT_TIMESTAMP WHERE id = ?3",
        )
        .bind(typ)
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

    // -----------------------------------------------------------------------
    // Interviews
    // -----------------------------------------------------------------------

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
    #[allow(clippy::type_complexity)]
    pub async fn list_interviews(&self, application_id: &str) -> Result<Vec<InterviewRow>> {
        let rows: Vec<(
            String,         // id
            String,         // application_id
            i64,            // round
            String,         // type
            Option<String>, // interviewer
            Option<String>, // scheduled_at
            String,         // status
            Option<String>, // questions
            Option<String>, // notes
            Option<String>, // outcome
            Option<i64>,    // duration_mins
            String,         // created_at
        )> = sqlx::query_as(
            "SELECT id, application_id, round, type, interviewer, scheduled_at,
                    status, questions, notes, outcome, duration_mins, created_at
             FROM interviews WHERE application_id = ?1
             ORDER BY round ASC",
        )
        .bind(application_id)
        .fetch_all(self.pool())
        .await
        .context(error::SqlxSnafu)?;

        Ok(rows
            .into_iter()
            .map(|r| InterviewRow {
                id:             r.0,
                application_id: r.1,
                round:          r.2,
                r#type:         r.3,
                interviewer:    r.4,
                scheduled_at:   r.5,
                status:         r.6,
                questions:      r.7,
                notes:          r.8,
                outcome:        r.9,
                duration_mins:  r.10,
                created_at:     r.11,
            })
            .collect())
    }

    // -----------------------------------------------------------------------
    // Tasks
    // -----------------------------------------------------------------------

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
    #[allow(clippy::type_complexity)]
    pub async fn list_tasks(&self, application_id: &str) -> Result<Vec<TaskRow>> {
        let rows: Vec<(
            String,         // id
            String,         // application_id
            String,         // type
            String,         // title
            Option<String>, // due_date
            bool,           // is_completed
            Option<String>, // notes
            String,         // created_at
        )> = sqlx::query_as(
            "SELECT id, application_id, type, title, due_date, is_completed, notes, created_at
             FROM tasks WHERE application_id = ?1
             ORDER BY due_date ASC NULLS LAST, created_at ASC",
        )
        .bind(application_id)
        .fetch_all(self.pool())
        .await
        .context(error::SqlxSnafu)?;

        Ok(rows
            .into_iter()
            .map(|r| TaskRow {
                id:             r.0,
                application_id: r.1,
                r#type:         r.2,
                title:          r.3,
                due_date:       r.4,
                is_completed:   r.5,
                notes:          r.6,
                created_at:     r.7,
            })
            .collect())
    }

    /// List all pending (incomplete) tasks across all applications, ordered by
    /// due date.
    #[allow(clippy::type_complexity)]
    pub async fn list_all_pending_tasks(&self) -> Result<Vec<TaskRow>> {
        let rows: Vec<(
            String,         // id
            String,         // application_id
            String,         // type
            String,         // title
            Option<String>, // due_date
            bool,           // is_completed
            Option<String>, // notes
            String,         // created_at
        )> = sqlx::query_as(
            "SELECT id, application_id, type, title, due_date, is_completed, notes, created_at
             FROM tasks WHERE is_completed = 0
             ORDER BY due_date ASC NULLS LAST, created_at ASC",
        )
        .fetch_all(self.pool())
        .await
        .context(error::SqlxSnafu)?;

        Ok(rows
            .into_iter()
            .map(|r| TaskRow {
                id:             r.0,
                application_id: r.1,
                r#type:         r.2,
                title:          r.3,
                due_date:       r.4,
                is_completed:   r.5,
                notes:          r.6,
                created_at:     r.7,
            })
            .collect())
    }

    // -----------------------------------------------------------------------
    // Stage events
    // -----------------------------------------------------------------------

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
    #[allow(clippy::type_complexity)]
    pub async fn list_stage_events(&self, application_id: &str) -> Result<Vec<StageEvent>> {
        let rows: Vec<(
            String,         // id
            String,         // application_id
            Option<String>, // from_stage
            String,         // to_stage
            String,         // occurred_at
            Option<String>, // metadata
            String,         // created_at
        )> = sqlx::query_as(
            "SELECT id, application_id, from_stage, to_stage, occurred_at, metadata, created_at
             FROM stage_events WHERE application_id = ?1
             ORDER BY occurred_at ASC",
        )
        .bind(application_id)
        .fetch_all(self.pool())
        .await
        .context(error::SqlxSnafu)?;

        Ok(rows
            .into_iter()
            .map(|r| StageEvent {
                id:             r.0,
                application_id: r.1,
                from_stage:     r.2,
                to_stage:       r.3,
                occurred_at:    r.4,
                metadata:       r.5,
                created_at:     r.6,
            })
            .collect())
    }

    // -----------------------------------------------------------------------
    // Status history
    // -----------------------------------------------------------------------

    /// Record a status change in the history table.
    async fn record_status_change(
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
        let rows: Vec<(String, String, Option<String>, String)> = sqlx::query_as(
            "SELECT from_status, to_status, note, created_at
             FROM status_history WHERE application_id = ?1
             ORDER BY created_at ASC",
        )
        .bind(application_id)
        .fetch_all(self.pool())
        .await
        .context(error::SqlxSnafu)?;

        Ok(rows
            .into_iter()
            .map(|r| StatusChange {
                from_status: r.0,
                to_status:   r.1,
                note:        r.2,
                created_at:  r.3,
            })
            .collect())
    }

    // -----------------------------------------------------------------------
    // Stats
    // -----------------------------------------------------------------------

    /// Get aggregate statistics across all applications.
    pub async fn stats(&self) -> Result<Stats> {
        let (total,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM applications")
            .fetch_one(self.pool())
            .await
            .context(error::SqlxSnafu)?;

        let status_rows: Vec<(String, i64)> = sqlx::query_as(
            "SELECT status, COUNT(*) FROM applications GROUP BY status ORDER BY status",
        )
        .fetch_all(self.pool())
        .await
        .context(error::SqlxSnafu)?;

        let outcome_rows: Vec<(String, i64)> = sqlx::query_as(
            "SELECT outcome, COUNT(*) FROM applications WHERE outcome IS NOT NULL GROUP BY \
             outcome ORDER BY outcome",
        )
        .fetch_all(self.pool())
        .await
        .context(error::SqlxSnafu)?;

        let stage_rows: Vec<(String, i64)> = sqlx::query_as(
            "SELECT stage, COUNT(*) FROM applications WHERE stage IS NOT NULL GROUP BY stage \
             ORDER BY stage",
        )
        .fetch_all(self.pool())
        .await
        .context(error::SqlxSnafu)?;

        let source_rows: Vec<(String, i64)> = sqlx::query_as(
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

        #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
        let by_status = status_rows
            .into_iter()
            .map(|(s, c)| (s, c as usize))
            .collect();

        #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
        let by_outcome = outcome_rows
            .into_iter()
            .map(|(s, c)| (s, c as usize))
            .collect();

        #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
        let by_stage = stage_rows
            .into_iter()
            .map(|(s, c)| (s, c as usize))
            .collect();

        #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
        let by_source = source_rows
            .into_iter()
            .map(|(s, c)| (s, c as usize))
            .collect();

        #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
        Ok(Stats {
            total: total as usize,
            by_status,
            by_outcome,
            by_stage,
            by_source,
            pending_tasks: pending_tasks as usize,
            upcoming_interviews: upcoming_interviews as usize,
        })
    }
}
