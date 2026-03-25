//! Agent backend for invoking local AI agent CLIs.
//!
//! This module provides a CLI-agnostic execution layer that can invoke
//! various agent CLIs (Claude, Kiro, Gemini, Codex, etc.) with prompts
//! and collect their output. Ported from ralph-orchestrator.
//!
//! Many items are kept for forward compatibility even if not yet used
//! by tenki's current command set.

pub mod backend;
pub mod config;
pub mod executor;
pub mod output;

pub use backend::CliBackend;
pub use config::AgentConfig;
pub use executor::CliExecutor;
pub use output::{extract_fenced_json, extract_result_from_stream_json};
