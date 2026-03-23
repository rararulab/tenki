//! Types for the job discovery extractor system.

use bon::Builder;
use serde::{Deserialize, Serialize};

/// Parameters for a job discovery search.
#[derive(Debug, Builder)]
pub struct DiscoverParams {
    /// Search query (e.g. "rust developer").
    pub query: String,
    /// Geographic filter (e.g. "shanghai").
    pub location: Option<String>,
    /// Maximum results to return.
    pub limit: Option<u32>,
}

/// A job discovered from an external source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredJob {
    /// Job title.
    pub title: String,
    /// Company name.
    pub company: String,
    /// URL to the job description page.
    pub jd_url: Option<String>,
    /// Raw job description text.
    pub jd_text: Option<String>,
    /// Job location.
    pub location: Option<String>,
    /// Salary or compensation info.
    pub salary: Option<String>,
    /// Source platform (e.g. "boss", "linkedin").
    pub source: String,
}
