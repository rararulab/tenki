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

#[allow(unused_imports)]
pub use backend::{CliBackend, CommandSpec, OutputFormat, PromptMode};
pub use config::AgentConfig;
#[allow(unused_imports)]
pub use config::ConfigPromptMode;
pub use executor::CliExecutor;
#[allow(unused_imports)]
pub use executor::ExecutionResult;
