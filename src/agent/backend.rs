//! CLI backend definitions for different AI tools.
//!
//! Provides factory methods for configuring various agent CLIs
//! (Claude, Kiro, Gemini, Codex, etc.) with the correct flags and
//! prompt-passing conventions. Ported from ralph-orchestrator.

use std::io::Write;

use snafu::Snafu;
use tempfile::NamedTempFile;

/// Module-level result type.
pub type Result<T> = std::result::Result<T, BackendError>;

/// Prompts longer than this are written to a temp file and the agent is
/// asked to read from that file instead. This avoids OS `ARG_MAX` limits.
const LARGE_PROMPT_THRESHOLD: usize = 7000;

/// Output format supported by a CLI backend.
///
/// This allows adapters to declare whether they emit structured JSON
/// for real-time streaming or plain text output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputFormat {
    /// Plain text output (default for most adapters).
    #[default]
    Text,
    /// Newline-delimited JSON stream (Claude with `--output-format
    /// stream-json`).
    StreamJson,
    /// Newline-delimited JSON stream (Pi with `--mode json`).
    PiStreamJson,
    /// Agent Client Protocol over stdio (Kiro v2).
    Acp,
}

/// Errors that can occur when constructing a backend.
#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum BackendError {
    /// Unknown backend name.
    #[snafu(display("unknown backend: {name}"))]
    UnknownBackend {
        /// The unrecognized backend name.
        name: String,
    },
}

/// How to pass prompts to the CLI tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptMode {
    /// Pass prompt as a command-line argument.
    Arg,
    /// Write prompt to stdin.
    Stdin,
}

/// Prepared command ready for execution.
///
/// Returned by [`CliBackend::build_command`]. The `_temp_file` handle must
/// be kept alive for the duration of command execution — dropping it deletes
/// the underlying file.
pub struct CommandSpec {
    /// The command binary to execute.
    pub command:     String,
    /// Fully resolved argument list (includes prompt).
    pub args:        Vec<String>,
    /// If set, this string should be written to the child's stdin.
    pub stdin_input: Option<String>,
    /// Temp file handle — kept alive so the agent can read from it.
    _temp_file:      Option<NamedTempFile>,
}

/// A CLI backend configuration for executing prompts.
///
/// Factory methods construct backends with the correct flags for each
/// supported agent CLI. All prompts are passed to CLI processes directly
/// via [`std::process::Command`] (no shell involved), so there is no
/// shell injection risk. However, callers should treat this as a local
/// execution boundary — only pass prompts from trusted, local sources.
#[derive(Debug, Clone)]
pub struct CliBackend {
    /// The command to execute.
    pub command:       String,
    /// Additional arguments before the prompt.
    pub args:          Vec<String>,
    /// How to pass the prompt.
    pub prompt_mode:   PromptMode,
    /// Argument flag for prompt (if `prompt_mode` is `Arg`).
    pub prompt_flag:   Option<String>,
    /// Output format emitted by this backend (used by downstream stream
    /// parsers to select the correct decoder).
    #[allow(dead_code)]
    pub output_format: OutputFormat,
    /// Environment variables to set when spawning the process.
    pub env_vars:      Vec<(String, String)>,
}

impl CliBackend {
    /// Creates the Claude backend.
    ///
    /// Uses `-p` flag for headless/print mode execution. This runs Claude
    /// in non-interactive mode where it executes the prompt and exits.
    ///
    /// Emits `--output-format stream-json` for NDJSON streaming output.
    /// Note: `--verbose` is required when using `--output-format stream-json`
    /// with `-p`.
    fn claude() -> Self {
        Self {
            command:       "claude".to_string(),
            args:          vec![
                "--dangerously-skip-permissions".to_string(),
                "--verbose".to_string(),
                "--output-format".to_string(),
                "stream-json".to_string(),
            ],
            prompt_mode:   PromptMode::Arg,
            prompt_flag:   Some("-p".to_string()),
            output_format: OutputFormat::StreamJson,
            env_vars:      vec![],
        }
    }

    /// Creates the Kiro backend.
    ///
    /// Uses `kiro-cli` in headless mode with all tools trusted.
    fn kiro() -> Self {
        Self {
            command:       "kiro-cli".to_string(),
            args:          vec![
                "chat".to_string(),
                "--no-interactive".to_string(),
                "--trust-all-tools".to_string(),
            ],
            prompt_mode:   PromptMode::Arg,
            prompt_flag:   None,
            output_format: OutputFormat::Text,
            env_vars:      vec![],
        }
    }

    /// Creates the Kiro ACP backend.
    ///
    /// Uses `kiro-cli` with the ACP subcommand for structured JSON-RPC
    /// communication over stdio instead of PTY text scraping.
    fn kiro_acp() -> Self {
        Self {
            command:       "kiro-cli".to_string(),
            args:          vec!["acp".to_string()],
            prompt_mode:   PromptMode::Stdin,
            prompt_flag:   None,
            output_format: OutputFormat::Acp,
            env_vars:      vec![],
        }
    }

    /// Creates the Gemini backend.
    fn gemini() -> Self {
        Self {
            command:       "gemini".to_string(),
            args:          vec!["--yolo".to_string()],
            prompt_mode:   PromptMode::Arg,
            prompt_flag:   Some("-p".to_string()),
            output_format: OutputFormat::Text,
            env_vars:      vec![],
        }
    }

    /// Creates the Codex backend.
    fn codex() -> Self {
        Self {
            command:       "codex".to_string(),
            args:          vec!["exec".to_string(), "--full-auto".to_string()],
            prompt_mode:   PromptMode::Arg,
            prompt_flag:   None,
            output_format: OutputFormat::Text,
            env_vars:      vec![],
        }
    }

    /// Creates the Amp backend.
    fn amp() -> Self {
        Self {
            command:       "amp".to_string(),
            args:          vec!["--dangerously-allow-all".to_string()],
            prompt_mode:   PromptMode::Arg,
            prompt_flag:   Some("-x".to_string()),
            output_format: OutputFormat::Text,
            env_vars:      vec![],
        }
    }

    /// Creates the Copilot backend for autonomous mode.
    ///
    /// Uses GitHub Copilot CLI with `--allow-all-tools` for automated tool
    /// approval.
    fn copilot() -> Self {
        Self {
            command:       "copilot".to_string(),
            args:          vec!["--allow-all-tools".to_string()],
            prompt_mode:   PromptMode::Arg,
            prompt_flag:   Some("-p".to_string()),
            output_format: OutputFormat::Text,
            env_vars:      vec![],
        }
    }

    /// Creates the `OpenCode` backend for autonomous/headless mode.
    ///
    /// Uses `OpenCode` CLI with `run` subcommand. The prompt is passed as a
    /// positional argument after the subcommand.
    fn opencode() -> Self {
        Self {
            command:       "opencode".to_string(),
            args:          vec!["run".to_string()],
            prompt_mode:   PromptMode::Arg,
            prompt_flag:   None,
            output_format: OutputFormat::Text,
            env_vars:      vec![],
        }
    }

    /// Creates the Pi backend for headless execution.
    ///
    /// Uses `-p` for print mode with `--mode json` for NDJSON streaming output.
    /// Emits `PiStreamJson` output format for structured event parsing.
    fn pi() -> Self {
        Self {
            command:       "pi".to_string(),
            args:          vec![
                "-p".to_string(),
                "--mode".to_string(),
                "json".to_string(),
                "--no-session".to_string(),
            ],
            prompt_mode:   PromptMode::Arg,
            prompt_flag:   None,
            output_format: OutputFormat::PiStreamJson,
            env_vars:      vec![],
        }
    }

    /// Creates the Roo backend for headless execution.
    ///
    /// Uses `--print` for non-interactive output and `--ephemeral` for clean
    /// disk state. Prompts are always passed via `--prompt-file` (handled in
    /// `build_command()`).
    fn roo() -> Self {
        Self {
            command:       "roo".to_string(),
            args:          vec!["--print".to_string(), "--ephemeral".to_string()],
            prompt_mode:   PromptMode::Arg,
            prompt_flag:   None,
            output_format: OutputFormat::Text,
            env_vars:      vec![],
        }
    }

    /// Creates a backend from a named backend string.
    ///
    /// # Errors
    /// Returns [`BackendError::UnknownBackend`] if the name is not recognized.
    pub fn from_name(name: &str) -> Result<Self> {
        match name {
            "claude" => Ok(Self::claude()),
            "kiro" => Ok(Self::kiro()),
            "kiro-acp" => Ok(Self::kiro_acp()),
            "gemini" => Ok(Self::gemini()),
            "codex" => Ok(Self::codex()),
            "amp" => Ok(Self::amp()),
            "copilot" => Ok(Self::copilot()),
            "opencode" => Ok(Self::opencode()),
            "pi" => Ok(Self::pi()),
            "roo" => Ok(Self::roo()),
            _ => UnknownBackendSnafu {
                name: name.to_string(),
            }
            .fail(),
        }
    }

    /// Builds roo prompt-file args: writes prompt to a temp file and
    /// appends `--prompt-file <path>` to args. Falls back to positional
    /// arg if temp file creation fails.
    fn build_roo_prompt_file(
        args: &mut Vec<String>,
        prompt: &str,
    ) -> (Option<String>, Option<NamedTempFile>) {
        match NamedTempFile::new() {
            Ok(mut file) => {
                if let Err(e) = file.write_all(prompt.as_bytes()) {
                    tracing::warn!("Failed to write roo prompt to temp file: {e}");
                    args.push(prompt.to_string());
                    (None, None)
                } else {
                    args.push("--prompt-file".to_string());
                    args.push(file.path().display().to_string());
                    (None, Some(file))
                }
            }
            Err(e) => {
                tracing::warn!("Failed to create temp file for roo: {e}");
                args.push(prompt.to_string());
                (None, None)
            }
        }
    }

    /// Builds the full command with arguments for execution.
    ///
    /// # Safety assumptions
    ///
    /// The prompt is passed directly to the child process via
    /// [`std::process::Command`] — no shell is involved, so there is no
    /// shell-injection risk. This function is intended for local, trusted
    /// prompts only. Do not pass untrusted external input without
    /// validation.
    pub fn build_command(&self, prompt: &str, interactive: bool) -> CommandSpec {
        let mut args = self.args.clone();

        // Filter args based on execution mode
        if interactive {
            args = self.filter_args_for_interactive(args);
        }

        // Handle prompt passing: Roo uses --prompt-file, all others use temp file for
        // large prompts
        let (stdin_input, temp_file) = match self.prompt_mode {
            PromptMode::Arg => {
                // Roo headless: always use --prompt-file for all prompts
                if self.command == "roo" && args.contains(&"--print".to_string()) {
                    Self::build_roo_prompt_file(&mut args, prompt)
                } else {
                    let (prompt_text, temp_file) = if prompt.len() > LARGE_PROMPT_THRESHOLD {
                        match NamedTempFile::new() {
                            Ok(mut file) => {
                                if let Err(e) = file.write_all(prompt.as_bytes()) {
                                    tracing::warn!("Failed to write prompt to temp file: {e}");
                                    (prompt.to_string(), None)
                                } else {
                                    let path = file.path().display().to_string();
                                    (
                                        format!("Please read and execute the task in {path}"),
                                        Some(file),
                                    )
                                }
                            }
                            Err(e) => {
                                tracing::warn!("Failed to create temp file: {e}");
                                (prompt.to_string(), None)
                            }
                        }
                    } else {
                        (prompt.to_string(), None)
                    };

                    if let Some(ref flag) = self.prompt_flag {
                        args.push(flag.clone());
                    }
                    args.push(prompt_text);
                    (None, temp_file)
                }
            }
            PromptMode::Stdin => (Some(prompt.to_string()), None),
        };

        tracing::debug!(
            command = %self.command,
            args_count = args.len(),
            prompt_len = prompt.len(),
            interactive = interactive,
            uses_stdin = stdin_input.is_some(),
            uses_temp_file = temp_file.is_some(),
            "Built CLI command"
        );
        tracing::trace!(prompt = %prompt, "Full prompt content");

        CommandSpec {
            command: self.command.clone(),
            args,
            stdin_input,
            _temp_file: temp_file,
        }
    }

    /// Filters args for interactive mode per spec table.
    fn filter_args_for_interactive(&self, args: Vec<String>) -> Vec<String> {
        match self.command.as_str() {
            "kiro-cli" => args
                .into_iter()
                .filter(|a| a != "--no-interactive")
                .collect(),
            "codex" => args.into_iter().filter(|a| a != "--full-auto").collect(),
            "amp" => args
                .into_iter()
                .filter(|a| a != "--dangerously-allow-all")
                .collect(),
            "copilot" => args
                .into_iter()
                .filter(|a| a != "--allow-all-tools")
                .collect(),
            "roo" => args
                .into_iter()
                .filter(|a| a != "--print" && a != "--ephemeral")
                .collect(),
            _ => args, // claude, gemini, opencode unchanged
        }
    }
}
