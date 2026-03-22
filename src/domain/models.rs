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
    pub tailored_summary:  Option<String>,
    pub tailored_headline: Option<String>,
    pub tailored_skills:   Option<String>,
    pub applied_at:        Option<String>,
    pub closed_at:         Option<String>,
    pub created_at:        String,
    pub updated_at:        String,
}

/// A single interview round linked to an application.
#[derive(Debug, Clone, Serialize)]
pub struct InterviewRow {
    pub id:             String,
    pub application_id: String,
    pub round:          i64,
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
#[derive(Debug, Clone, Serialize)]
pub struct TaskRow {
    pub id:             String,
    pub application_id: String,
    pub r#type:         String,
    pub title:          String,
    pub due_date:       Option<String>,
    pub is_completed:   bool,
    pub notes:          Option<String>,
    pub created_at:     String,
}

/// A stage transition event for an application.
#[derive(Debug, Clone, Serialize)]
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
#[derive(Debug, Clone, Serialize)]
pub struct StatusChange {
    pub from_status: String,
    pub to_status:   String,
    pub note:        Option<String>,
    pub created_at:  String,
}

/// Aggregate statistics across all applications.
#[derive(Debug, Clone, Serialize)]
pub struct Stats {
    pub total:               usize,
    pub by_status:           Vec<(String, usize)>,
    pub by_outcome:          Vec<(String, usize)>,
    pub by_stage:            Vec<(String, usize)>,
    pub by_source:           Vec<(String, usize)>,
    pub pending_tasks:       usize,
    pub upcoming_interviews: usize,
}
