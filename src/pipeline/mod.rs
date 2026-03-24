//! Pipeline orchestrator — chains discover → score → tailor → export.

pub mod steps;

use bon::Builder;
use serde::Serialize;

/// Configuration for a pipeline run.
#[derive(Debug, Builder)]
pub struct PipelineConfig {
    /// Search query for job discovery.
    pub query:       String,
    /// Source platforms to search (empty = all).
    pub sources:     Vec<String>,
    /// Location filter.
    pub location:    Option<String>,
    /// Maximum results to keep after scoring.
    pub top_n:       u32,
    /// Minimum fitness score threshold.
    pub min_score:   f64,
    /// Skip tailoring step.
    pub skip_tailor: bool,
    /// Skip export step.
    pub skip_export: bool,
}

/// Per-application detail included in the pipeline response.
#[derive(Debug, Serialize)]
pub struct ApplicationSummary {
    /// Application ID.
    pub id:            String,
    /// Company name.
    pub company:       String,
    /// Job position title.
    pub position:      String,
    /// Fitness score assigned during scoring (None if unscored).
    pub fitness_score: Option<f64>,
    /// Whether a tailored resume was generated.
    pub tailored:      bool,
    /// Whether a PDF export exists.
    pub has_pdf:       bool,
}

/// A non-fatal error captured during a pipeline step.
#[derive(Debug, Serialize)]
pub struct PipelineError {
    /// Application ID that caused the error.
    pub id:      String,
    /// Pipeline step where the error occurred (e.g. "score", "tailor",
    /// "export").
    pub step:    String,
    /// Human-readable error message.
    pub message: String,
}

/// Summary of a pipeline run.
#[derive(Debug, Serialize)]
pub struct PipelineSummary {
    /// Whether the pipeline completed successfully.
    pub ok:              bool,
    /// Action name for JSON output.
    pub action:          &'static str,
    /// Number of jobs found from external sources.
    pub discovered:      usize,
    /// Number of new jobs imported (after dedup).
    pub imported:        usize,
    /// Number of jobs scored in this run.
    pub scored:          usize,
    /// Number of jobs above the score threshold.
    pub above_threshold: usize,
    /// Number of jobs tailored in this run.
    pub tailored:        usize,
    /// Number of resumes exported in this run.
    pub exported:        usize,
    /// Per-application details for qualified applications.
    pub applications:    Vec<ApplicationSummary>,
    /// Non-fatal errors encountered during pipeline steps.
    pub errors:          Vec<PipelineError>,
}
