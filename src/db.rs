//! Domain-level database operations.

use std::{fmt, path::PathBuf, str::FromStr};

use serde::Serialize;
use snafu::ResultExt as _;
use sqlx::SqlitePool;

use crate::{
    error::{self, Result, TenkiError},
    paths,
    store::{DBStore, DatabaseConfig},
};

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AppStatus {
    Bookmarked,
    Applied,
    Screening,
    Interview,
    Offer,
    Accepted,
    Rejected,
    Withdrawn,
}

impl AppStatus {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Bookmarked => "bookmarked",
            Self::Applied => "applied",
            Self::Screening => "screening",
            Self::Interview => "interview",
            Self::Offer => "offer",
            Self::Accepted => "accepted",
            Self::Rejected => "rejected",
            Self::Withdrawn => "withdrawn",
        }
    }
}

impl fmt::Display for AppStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for AppStatus {
    type Err = TenkiError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "bookmarked" => Ok(Self::Bookmarked),
            "applied" => Ok(Self::Applied),
            "screening" => Ok(Self::Screening),
            "interview" => Ok(Self::Interview),
            "offer" => Ok(Self::Offer),
            "accepted" => Ok(Self::Accepted),
            "rejected" => Ok(Self::Rejected),
            "withdrawn" => Ok(Self::Withdrawn),
            other => Err(TenkiError::InvalidStatus {
                status: other.to_string(),
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum InterviewType {
    Phone,
    Technical,
    Behavioral,
    #[clap(name = "system-design")]
    SystemDesign,
    Hr,
    Other,
}

impl InterviewType {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Phone => "phone",
            Self::Technical => "technical",
            Self::Behavioral => "behavioral",
            Self::SystemDesign => "system-design",
            Self::Hr => "hr",
            Self::Other => "other",
        }
    }
}

impl fmt::Display for InterviewType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum InterviewStatus {
    Scheduled,
    Completed,
    Cancelled,
}

impl InterviewStatus {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Scheduled => "scheduled",
            Self::Completed => "completed",
            Self::Cancelled => "cancelled",
        }
    }
}

impl fmt::Display for InterviewStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

// ---------------------------------------------------------------------------
// Domain structs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct Application {
    pub id: String,
    pub company: String,
    pub position: String,
    pub jd_url: Option<String>,
    pub jd_text: Option<String>,
    pub location: Option<String>,
    pub status: String,
    pub fitness_score: Option<f64>,
    pub fitness_notes: Option<String>,
    pub resume_typ: Option<String>,
    pub has_resume_pdf: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct InterviewRow {
    pub id: String,
    pub application_id: String,
    pub round: i64,
    pub r#type: String,
    pub interviewer: Option<String>,
    pub scheduled_at: Option<String>,
    pub status: String,
    pub questions: Option<String>,
    pub notes: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct StatusChange {
    pub from_status: String,
    pub to_status: String,
    pub note: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Stats {
    pub total: usize,
    pub by_status: Vec<(String, usize)>,
}

// ---------------------------------------------------------------------------
// Database
// ---------------------------------------------------------------------------

pub struct Database {
    store: DBStore,
    path: PathBuf,
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
    pub fn path(&self) -> &std::path::Path {
        &self.path
    }

    /// Return a reference to the underlying `SqlitePool`.
    pub const fn pool(&self) -> &SqlitePool {
        self.store.pool()
    }

    /// Execute the schema DDL to create all tables.
    pub async fn init(&self) -> Result<()> {
        let schema = include_str!("schema.sql");
        sqlx::raw_sql(schema)
            .execute(self.pool())
            .await
            .context(error::SqlxSnafu)?;
        Ok(())
    }

    /// Check whether the database has been initialized (applications table exists).
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
        let rows: Vec<(String,)> =
            sqlx::query_as("SELECT id FROM applications WHERE id LIKE ?1")
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
        let rows: Vec<(String,)> =
            sqlx::query_as("SELECT id FROM interviews WHERE id LIKE ?1")
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

    // -----------------------------------------------------------------------
    // Applications
    // -----------------------------------------------------------------------

    /// Add a new application and record the initial status change.
    pub async fn add_application(
        &self,
        company: &str,
        position: &str,
        jd_url: Option<&str>,
        jd_text: Option<&str>,
        location: Option<&str>,
        status: AppStatus,
    ) -> Result<String> {
        let id = uuid::Uuid::new_v4().to_string();
        let status_str = status.as_str();

        sqlx::query(
            "INSERT INTO applications (id, company, position, jd_url, jd_text, location, status)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        )
        .bind(&id)
        .bind(company)
        .bind(position)
        .bind(jd_url)
        .bind(jd_text)
        .bind(location)
        .bind(status_str)
        .execute(self.pool())
        .await
        .context(error::SqlxSnafu)?;

        self.record_status_change(&id, "", status_str, None)
            .await?;

        Ok(id)
    }

    /// Fetch a single application by ID.
    pub async fn get_application(&self, id: &str) -> Result<Application> {
        let row: Option<(
            String,         // id
            String,         // company
            String,         // position
            Option<String>, // jd_url
            Option<String>, // jd_text
            Option<String>, // location
            String,         // status
            Option<f64>,    // fitness_score
            Option<String>, // fitness_notes
            Option<String>, // resume_typ
            bool,           // has_resume_pdf
            String,         // created_at
            String,         // updated_at
        )> = sqlx::query_as(
            "SELECT id, company, position, jd_url, jd_text, location, status,
                    fitness_score, fitness_notes, resume_typ,
                    (resume_pdf IS NOT NULL) AS has_resume_pdf,
                    created_at, updated_at
             FROM applications WHERE id = ?1",
        )
        .bind(id)
        .fetch_optional(self.pool())
        .await
        .context(error::SqlxSnafu)?;

        row.map(|r| Application {
            id: r.0,
            company: r.1,
            position: r.2,
            jd_url: r.3,
            jd_text: r.4,
            location: r.5,
            status: r.6,
            fitness_score: r.7,
            fitness_notes: r.8,
            resume_typ: r.9,
            has_resume_pdf: r.10,
            created_at: r.11,
            updated_at: r.12,
        })
        .ok_or_else(|| TenkiError::ApplicationNotFound { id: id.to_string() })
    }

    /// List applications with optional filters.
    pub async fn list_applications(
        &self,
        status: Option<AppStatus>,
        company: Option<&str>,
    ) -> Result<Vec<Application>> {
        let mut sql = String::from(
            "SELECT id, company, position, jd_url, jd_text, location, status,
                    fitness_score, fitness_notes, resume_typ,
                    (resume_pdf IS NOT NULL) AS has_resume_pdf,
                    created_at, updated_at
             FROM applications WHERE 1=1",
        );
        let mut binds: Vec<String> = Vec::new();

        if let Some(s) = status {
            sql.push_str(" AND status = ?");
            binds.push(s.as_str().to_string());
        }
        if let Some(c) = company {
            sql.push_str(" AND company LIKE ?");
            binds.push(format!("%{c}%"));
        }
        sql.push_str(" ORDER BY updated_at DESC");

        // We need to build the query dynamically. sqlx requires compile-time
        // knowledge of bind count when using `query_as`, so we use `query_as`
        // with a manual bind loop via `QueryAs`.
        let mut q = sqlx::query_as::<_, (
            String,
            String,
            String,
            Option<String>,
            Option<String>,
            Option<String>,
            String,
            Option<f64>,
            Option<String>,
            Option<String>,
            bool,
            String,
            String,
        )>(&sql);

        for b in &binds {
            q = q.bind(b);
        }

        let rows = q.fetch_all(self.pool()).await.context(error::SqlxSnafu)?;

        Ok(rows
            .into_iter()
            .map(|r| Application {
                id: r.0,
                company: r.1,
                position: r.2,
                jd_url: r.3,
                jd_text: r.4,
                location: r.5,
                status: r.6,
                fitness_score: r.7,
                fitness_notes: r.8,
                resume_typ: r.9,
                has_resume_pdf: r.10,
                created_at: r.11,
                updated_at: r.12,
            })
            .collect())
    }

    /// Update an application's status and record the change.
    pub async fn update_application_status(
        &self,
        id: &str,
        new_status: AppStatus,
    ) -> Result<()> {
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
        company: Option<&str>,
        position: Option<&str>,
        location: Option<&str>,
        jd_url: Option<&str>,
        jd_text: Option<&str>,
    ) -> Result<()> {
        // Ensure the application exists first.
        let _ = self.get_application(id).await?;

        let mut sets: Vec<String> = Vec::new();
        let mut binds: Vec<String> = Vec::new();

        if let Some(v) = company {
            sets.push("company = ?".to_string());
            binds.push(v.to_string());
        }
        if let Some(v) = position {
            sets.push("position = ?".to_string());
            binds.push(v.to_string());
        }
        if let Some(v) = location {
            sets.push("location = ?".to_string());
            binds.push(v.to_string());
        }
        if let Some(v) = jd_url {
            sets.push("jd_url = ?".to_string());
            binds.push(v.to_string());
        }
        if let Some(v) = jd_text {
            sets.push("jd_text = ?".to_string());
            binds.push(v.to_string());
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
    pub async fn update_fitness(
        &self,
        id: &str,
        score: f64,
        notes: &str,
    ) -> Result<()> {
        let result = sqlx::query(
            "UPDATE applications SET fitness_score = ?1, fitness_notes = ?2, updated_at = CURRENT_TIMESTAMP WHERE id = ?3",
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
    pub async fn update_resume(
        &self,
        id: &str,
        typ: &str,
        pdf_bytes: &[u8],
    ) -> Result<()> {
        let result = sqlx::query(
            "UPDATE applications SET resume_typ = ?1, resume_pdf = ?2, updated_at = CURRENT_TIMESTAMP WHERE id = ?3",
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
        let row: Option<(Vec<u8>,)> =
            sqlx::query_as("SELECT resume_pdf FROM applications WHERE id = ?1 AND resume_pdf IS NOT NULL")
                .bind(id)
                .fetch_optional(self.pool())
                .await
                .context(error::SqlxSnafu)?;

        Ok(row.map(|r| r.0))
    }

    /// Retrieve the resume typst source.
    pub async fn get_resume_typ(&self, id: &str) -> Result<Option<String>> {
        let row: Option<(String,)> =
            sqlx::query_as("SELECT resume_typ FROM applications WHERE id = ?1 AND resume_typ IS NOT NULL")
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
    ) -> Result<String> {
        // Verify the application exists.
        let _ = self.get_application(application_id).await?;

        let id = uuid::Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO interviews (id, application_id, round, type, interviewer, scheduled_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        )
        .bind(&id)
        .bind(application_id)
        .bind(round)
        .bind(interview_type.as_str())
        .bind(interviewer)
        .bind(scheduled_at)
        .execute(self.pool())
        .await
        .context(error::SqlxSnafu)?;

        Ok(id)
    }

    /// Update an interview's status.
    pub async fn update_interview_status(
        &self,
        id: &str,
        status: InterviewStatus,
    ) -> Result<()> {
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
    pub async fn list_interviews(
        &self,
        application_id: &str,
    ) -> Result<Vec<InterviewRow>> {
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
            String,         // created_at
        )> = sqlx::query_as(
            "SELECT id, application_id, round, type, interviewer, scheduled_at,
                    status, questions, notes, created_at
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
                id: r.0,
                application_id: r.1,
                round: r.2,
                r#type: r.3,
                interviewer: r.4,
                scheduled_at: r.5,
                status: r.6,
                questions: r.7,
                notes: r.8,
                created_at: r.9,
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
    pub async fn get_timeline(
        &self,
        application_id: &str,
    ) -> Result<Vec<StatusChange>> {
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
                to_status: r.1,
                note: r.2,
                created_at: r.3,
            })
            .collect())
    }

    // -----------------------------------------------------------------------
    // Stats
    // -----------------------------------------------------------------------

    /// Get aggregate statistics across all applications.
    pub async fn stats(&self) -> Result<Stats> {
        let (total,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM applications")
                .fetch_one(self.pool())
                .await
                .context(error::SqlxSnafu)?;

        let rows: Vec<(String, i64)> = sqlx::query_as(
            "SELECT status, COUNT(*) FROM applications GROUP BY status ORDER BY status",
        )
        .fetch_all(self.pool())
        .await
        .context(error::SqlxSnafu)?;

        #[allow(clippy::cast_sign_loss)]
        let by_status = rows
            .into_iter()
            .map(|(s, c)| (s, c as usize))
            .collect();

        #[allow(clippy::cast_sign_loss)]
        Ok(Stats {
            total: total as usize,
            by_status,
        })
    }
}
