//! Parameter structs for database and CLI operations.

use bon::Builder;

use super::enums::{AppStatus, JobLevel, JobType, Outcome, Stage};

/// Parameters for creating a new job application.
#[derive(Debug, Builder)]
pub struct AddApplicationParams<'a> {
    /// Company name.
    pub company: &'a str,
    /// Job title or position.
    pub position: &'a str,
    /// URL to the job description.
    pub jd_url: Option<&'a str>,
    /// Full text of the job description.
    pub jd_text: Option<&'a str>,
    /// Job location (city, region, etc.).
    pub location: Option<&'a str>,
    /// Initial application status (defaults to Bookmarked).
    #[builder(default = AppStatus::Bookmarked)]
    pub status: AppStatus,
    /// Salary or compensation info.
    pub salary: Option<&'a str>,
    /// Employment type classification.
    pub job_type: Option<JobType>,
    /// Seniority level.
    pub job_level: Option<JobLevel>,
    /// Whether the position is remote.
    pub is_remote: Option<bool>,
    /// Where the listing was found.
    pub source: Option<&'a str>,
    /// Company website URL.
    pub company_url: Option<&'a str>,
    /// Free-form notes.
    pub notes: Option<&'a str>,
}

/// Parameters for updating application fields (all optional except id).
#[derive(Debug, Default, Builder)]
pub struct UpdateApplicationParams<'a> {
    /// Company name override.
    pub company: Option<&'a str>,
    /// Position override.
    pub position: Option<&'a str>,
    /// Location override.
    pub location: Option<&'a str>,
    /// JD URL override.
    pub jd_url: Option<&'a str>,
    /// JD text override.
    pub jd_text: Option<&'a str>,
    /// Salary override.
    pub salary: Option<&'a str>,
    /// Job type override (as string).
    pub job_type: Option<&'a str>,
    /// Job level override (as string).
    pub job_level: Option<&'a str>,
    /// Remote flag override.
    pub is_remote: Option<bool>,
    /// Skills override.
    pub skills: Option<&'a str>,
    /// Experience range override.
    pub experience_range: Option<&'a str>,
    /// Source override.
    pub source: Option<&'a str>,
    /// Company URL override.
    pub company_url: Option<&'a str>,
    /// Notes override.
    pub notes: Option<&'a str>,
    /// Tailored summary override.
    pub tailored_summary: Option<&'a str>,
    /// Tailored headline override.
    pub tailored_headline: Option<&'a str>,
    /// Tailored skills override.
    pub tailored_skills: Option<&'a str>,
    /// Applied-at date override.
    pub applied_at: Option<&'a str>,
}

/// Filter parameters for listing applications.
#[derive(Debug, Default, Builder)]
pub struct ListApplicationParams<'a> {
    /// Filter by application status.
    pub status: Option<AppStatus>,
    /// Filter by company name (substring match).
    pub company: Option<&'a str>,
    /// Filter by outcome.
    pub outcome: Option<Outcome>,
    /// Filter by pipeline stage.
    pub stage: Option<Stage>,
    /// Filter by source (substring match).
    pub source: Option<&'a str>,
}
