//! Job discovery extractor system.
//!
//! Provides the [`Extractor`] trait and platform adapters (e.g. OpenCLI)
//! for discovering jobs from external sources.

pub mod opencli;
pub mod types;

pub use types::{DiscoverParams, DiscoveredJob};

use crate::error::Result;

/// Trait for job discovery backends.
pub trait Extractor: Send + Sync {
    /// Human-readable name of this extractor.
    fn name(&self) -> &str;
    /// Supported source identifiers (e.g. ["boss", "linkedin"]).
    fn sources(&self) -> &[&str];
    /// Discover jobs matching the given parameters.
    fn discover(
        &self,
        params: &DiscoverParams,
    ) -> impl std::future::Future<Output = Result<Vec<DiscoveredJob>>> + Send;
}
