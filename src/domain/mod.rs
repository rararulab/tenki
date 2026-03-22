//! Domain types for tenki.

mod enums;
mod models;
mod params;
pub mod validation;

pub use enums::{
    AppStatus, InterviewOutcome, InterviewStatus, InterviewType, JobLevel, JobType, Outcome, Stage,
    TaskType,
};
pub use models::{Application, InterviewRow, StageEvent, Stats, StatusChange, TaskRow};
pub use params::{AddApplicationParams, ListApplicationParams, UpdateApplicationParams};
