//! Domain enums shared across tenki modules.

use std::{fmt, str::FromStr};

use serde::Serialize;

use crate::error::TenkiError;

/// Application lifecycle status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AppStatus {
    Discovered,
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
    /// Returns the lowercase string representation of this status.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Discovered => "discovered",
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
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.write_str(self.as_str()) }
}

impl FromStr for AppStatus {
    type Err = TenkiError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "discovered" => Ok(Self::Discovered),
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

/// Interview format or category.
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
    /// Returns the kebab-case string representation of this interview type.
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
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.write_str(self.as_str()) }
}

/// Interview scheduling status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum InterviewStatus {
    Scheduled,
    Completed,
    Cancelled,
}

impl InterviewStatus {
    /// Returns the lowercase string representation of this interview status.
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
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.write_str(self.as_str()) }
}

/// Final outcome of an application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    #[clap(name = "offer-accepted")]
    OfferAccepted,
    #[clap(name = "offer-declined")]
    OfferDeclined,
    Rejected,
    Withdrawn,
    #[clap(name = "no-response")]
    NoResponse,
    Ghosted,
}

impl Outcome {
    /// Returns the `snake_case` string representation of this outcome.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::OfferAccepted => "offer_accepted",
            Self::OfferDeclined => "offer_declined",
            Self::Rejected => "rejected",
            Self::Withdrawn => "withdrawn",
            Self::NoResponse => "no_response",
            Self::Ghosted => "ghosted",
        }
    }
}

impl fmt::Display for Outcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.write_str(self.as_str()) }
}

impl FromStr for Outcome {
    type Err = TenkiError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "offer_accepted" => Ok(Self::OfferAccepted),
            "offer_declined" => Ok(Self::OfferDeclined),
            "rejected" => Ok(Self::Rejected),
            "withdrawn" => Ok(Self::Withdrawn),
            "no_response" => Ok(Self::NoResponse),
            "ghosted" => Ok(Self::Ghosted),
            other => Err(TenkiError::InvalidStatus {
                status: other.to_string(),
            }),
        }
    }
}

/// Pipeline stage within an application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    Applied,
    #[clap(name = "recruiter-screen")]
    RecruiterScreen,
    Assessment,
    #[clap(name = "hiring-manager")]
    HiringManager,
    Technical,
    Onsite,
    Offer,
    Closed,
}

impl Stage {
    /// Returns the `snake_case` string representation of this stage.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Applied => "applied",
            Self::RecruiterScreen => "recruiter_screen",
            Self::Assessment => "assessment",
            Self::HiringManager => "hiring_manager",
            Self::Technical => "technical",
            Self::Onsite => "onsite",
            Self::Offer => "offer",
            Self::Closed => "closed",
        }
    }
}

impl fmt::Display for Stage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.write_str(self.as_str()) }
}

impl FromStr for Stage {
    type Err = TenkiError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "applied" => Ok(Self::Applied),
            "recruiter_screen" => Ok(Self::RecruiterScreen),
            "assessment" => Ok(Self::Assessment),
            "hiring_manager" => Ok(Self::HiringManager),
            "technical" => Ok(Self::Technical),
            "onsite" => Ok(Self::Onsite),
            "offer" => Ok(Self::Offer),
            "closed" => Ok(Self::Closed),
            other => Err(TenkiError::InvalidStatus {
                status: other.to_string(),
            }),
        }
    }
}

/// Employment type classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum JobType {
    #[clap(name = "full-time")]
    FullTime,
    #[clap(name = "part-time")]
    PartTime,
    Contract,
    Internship,
}

impl JobType {
    /// Returns the `snake_case` string representation of this job type.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::FullTime => "full_time",
            Self::PartTime => "part_time",
            Self::Contract => "contract",
            Self::Internship => "internship",
        }
    }
}

impl fmt::Display for JobType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.write_str(self.as_str()) }
}

impl FromStr for JobType {
    type Err = TenkiError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "full_time" => Ok(Self::FullTime),
            "part_time" => Ok(Self::PartTime),
            "contract" => Ok(Self::Contract),
            "internship" => Ok(Self::Internship),
            other => Err(TenkiError::InvalidStatus {
                status: other.to_string(),
            }),
        }
    }
}

/// Seniority level of a job position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum JobLevel {
    Junior,
    Mid,
    Senior,
    Lead,
    Staff,
    Principal,
}

impl JobLevel {
    /// Returns the lowercase string representation of this job level.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Junior => "junior",
            Self::Mid => "mid",
            Self::Senior => "senior",
            Self::Lead => "lead",
            Self::Staff => "staff",
            Self::Principal => "principal",
        }
    }
}

impl fmt::Display for JobLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.write_str(self.as_str()) }
}

impl FromStr for JobLevel {
    type Err = TenkiError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "junior" => Ok(Self::Junior),
            "mid" => Ok(Self::Mid),
            "senior" => Ok(Self::Senior),
            "lead" => Ok(Self::Lead),
            "staff" => Ok(Self::Staff),
            "principal" => Ok(Self::Principal),
            other => Err(TenkiError::InvalidStatus {
                status: other.to_string(),
            }),
        }
    }
}

/// Category of a follow-up task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskType {
    Prep,
    Todo,
    #[clap(name = "follow-up")]
    FollowUp,
    #[clap(name = "check-status")]
    CheckStatus,
}

impl TaskType {
    /// Returns the `snake_case` string representation of this task type.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Prep => "prep",
            Self::Todo => "todo",
            Self::FollowUp => "follow_up",
            Self::CheckStatus => "check_status",
        }
    }
}

impl fmt::Display for TaskType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.write_str(self.as_str()) }
}

impl FromStr for TaskType {
    type Err = TenkiError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "prep" => Ok(Self::Prep),
            "todo" => Ok(Self::Todo),
            "follow_up" => Ok(Self::FollowUp),
            "check_status" => Ok(Self::CheckStatus),
            other => Err(TenkiError::InvalidStatus {
                status: other.to_string(),
            }),
        }
    }
}

/// Result of a single interview round.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InterviewOutcome {
    Pass,
    Fail,
    Pending,
    Cancelled,
}

impl InterviewOutcome {
    /// Returns the lowercase string representation of this interview outcome.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Fail => "fail",
            Self::Pending => "pending",
            Self::Cancelled => "cancelled",
        }
    }
}

impl fmt::Display for InterviewOutcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.write_str(self.as_str()) }
}

impl FromStr for InterviewOutcome {
    type Err = TenkiError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "pass" => Ok(Self::Pass),
            "fail" => Ok(Self::Fail),
            "pending" => Ok(Self::Pending),
            "cancelled" => Ok(Self::Cancelled),
            other => Err(TenkiError::InvalidStatus {
                status: other.to_string(),
            }),
        }
    }
}
