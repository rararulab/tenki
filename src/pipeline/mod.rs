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
}
