//! Agent backend for invoking local AI agent CLIs.
//!
//! This module provides a CLI-agnostic execution layer that can invoke
//! various agent CLIs (Claude, Kiro, Gemini, Codex, etc.) with prompts
//! and collect their output. Ported from cli-template.

pub mod backend;
pub mod config;
pub mod executor;

// Re-export the subset used by tenki's analyze command.
pub use backend::CliBackend;
pub use config::AgentConfig;
pub use executor::CliExecutor;
