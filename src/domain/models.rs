//! Domain model structs used across tenki.

use serde::Serialize;

/// A job application with all tracked metadata.
#[derive(Debug, Clone, Serialize)]
pub struct Application {
    pub id:                String,
    pub company:           String,
    pub position:          String,
    pub jd_url:            Option<String>,
    pub jd_text:           Option<String>,
    pub location:          Option<String>,
    pub status:            String,
    pub stage:             Option<String>,
    pub outcome:           Option<String>,
    pub fitness_score:     Option<f64>,
    pub fitness_notes:     Option<String>,
    pub resume_typ:        Option<String>,
    pub has_resume_pdf:    bool,
    pub salary:            Option<String>,
    pub salary_min:        Option<f64>,
    pub salary_max:        Option<f64>,
    pub salary_currency:   Option<String>,
    pub job_type:          Option<String>,
    pub is_remote:         Option<bool>,
    pub job_level:         Option<String>,
    pub skills:            Option<String>,
    pub experience_range:  Option<String>,
    pub source:            Option<String>,
    pub company_url:       Option<String>,
    pub notes:             Option<String>,
    pub posted_at:         Option<String>,
    pub tailored_summary:  Option<String>,
    pub tailored_headline: Option<String>,
    pub tailored_skills:   Option<String>,
    pub applied_at:        Option<String>,
    pub closed_at:         Option<String>,
    pub created_at:        String,
    pub updated_at:        String,
}

/// Raw database row for applications — matches SQL column types exactly.
/// Used internally by query layer; converted to `Application` after fetch.
#[derive(Debug, Clone, sqlx::FromRow)]
pub(crate) struct ApplicationRow {
    pub id:                String,
    pub company:           String,
    pub position:          String,
    pub jd_url:            Option<String>,
    pub jd_text:           Option<String>,
    pub location:          Option<String>,
    pub status:            String,
    pub stage:             Option<String>,
    pub outcome:           Option<String>,
    pub fitness_score:     Option<f64>,
    pub fitness_notes:     Option<String>,
    pub resume_typ:        Option<String>,
    #[sqlx(rename = "has_resume_pdf")]
    pub has_resume_pdf:    bool,
    pub salary:            Option<String>,
    pub salary_min:        Option<f64>,
    pub salary_max:        Option<f64>,
    pub salary_currency:   Option<String>,
    pub job_type:          Option<String>,
    pub is_remote:         Option<i64>,
    pub job_level:         Option<String>,
    pub skills:            Option<String>,
    pub experience_range:  Option<String>,
    pub source:            Option<String>,
    pub company_url:       Option<String>,
    pub notes:             Option<String>,
    pub posted_at:         Option<String>,
    pub tailored_summary:  Option<String>,
    pub tailored_headline: Option<String>,
    pub tailored_skills:   Option<String>,
    pub applied_at:        Option<String>,
    pub closed_at:         Option<String>,
    pub created_at:        String,
    pub updated_at:        String,
}

impl From<ApplicationRow> for Application {
    fn from(r: ApplicationRow) -> Self {
        Self {
            id:                r.id,
            company:           r.company,
            position:          r.position,
            jd_url:            r.jd_url,
            jd_text:           r.jd_text,
            location:          r.location,
            status:            r.status,
            stage:             r.stage,
            outcome:           r.outcome,
            fitness_score:     r.fitness_score,
            fitness_notes:     r.fitness_notes,
            resume_typ:        r.resume_typ,
            has_resume_pdf:    r.has_resume_pdf,
            salary:            r.salary,
            salary_min:        r.salary_min,
            salary_max:        r.salary_max,
            salary_currency:   r.salary_currency,
            job_type:          r.job_type,
            is_remote:         r.is_remote.map(|v| v != 0),
            job_level:         r.job_level,
            skills:            r.skills,
            experience_range:  r.experience_range,
            source:            r.source,
            company_url:       r.company_url,
            notes:             r.notes,
            posted_at:         r.posted_at,
            tailored_summary:  r.tailored_summary,
            tailored_headline: r.tailored_headline,
            tailored_skills:   r.tailored_skills,
            applied_at:        r.applied_at,
            closed_at:         r.closed_at,
            created_at:        r.created_at,
            updated_at:        r.updated_at,
        }
    }
}

/// A single interview round linked to an application.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct InterviewRow {
    pub id:             String,
    pub application_id: String,
    pub round:          i64,
    #[sqlx(rename = "type")]
    pub r#type:         String,
    pub interviewer:    Option<String>,
    pub scheduled_at:   Option<String>,
    pub status:         String,
    pub questions:      Option<String>,
    pub notes:          Option<String>,
    pub outcome:        Option<String>,
    pub duration_mins:  Option<i64>,
    pub created_at:     String,
}

/// A task (e.g. follow-up, prep) linked to an application.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct TaskRow {
    pub id:             String,
    pub application_id: String,
    #[sqlx(rename = "type")]
    pub r#type:         String,
    pub title:          String,
    pub due_date:       Option<String>,
    pub is_completed:   bool,
    pub notes:          Option<String>,
    pub created_at:     String,
}

/// A stage transition event for an application.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct StageEvent {
    pub id:             String,
    pub application_id: String,
    pub from_stage:     Option<String>,
    pub to_stage:       String,
    pub occurred_at:    String,
    pub metadata:       Option<String>,
    pub created_at:     String,
}

/// A record of an application's status change.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct StatusChange {
    pub from_status: String,
    pub to_status:   String,
    pub note:        Option<String>,
    pub created_at:  String,
}

/// Aggregate statistics across all applications.
#[derive(Debug, Clone, Serialize)]
pub struct Stats {
    pub total:               i64,
    pub by_status:           Vec<(String, i64)>,
    pub by_outcome:          Vec<(String, i64)>,
    pub by_stage:            Vec<(String, i64)>,
    pub by_source:           Vec<(String, i64)>,
    pub pending_tasks:       i64,
    pub upcoming_interviews: i64,
}
