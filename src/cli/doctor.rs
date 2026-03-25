//! Doctor command — verify critical configuration and dependencies.

use serde::Serialize;

use crate::app_config;

/// Severity level for a doctor check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
enum Severity {
    /// Must pass for tenki to function.
    Critical,
    /// Recommended but not blocking.
    Warning,
}

/// Result of a single doctor check.
#[derive(Debug, Serialize)]
struct CheckResult {
    name: &'static str,
    severity: Severity,
    passed: bool,
    message: String,
}

/// Aggregated doctor report.
#[derive(Debug, Serialize)]
struct DoctorReport {
    ok: bool,
    action: &'static str,
    checks: Vec<CheckResult>,
}

/// Run all doctor checks and report results.
pub fn run(json: bool) -> bool {
    let cfg = app_config::load();
    let checks = vec![
        check_config_file(),
        check_database(),
        check_agent_backend(cfg),
        check_opencli(),
        check_resume_config(cfg),
        check_preferences(cfg),
    ];

    let all_critical_pass = checks
        .iter()
        .filter(|c| c.severity == Severity::Critical)
        .all(|c| c.passed);

    if json {
        let report = DoctorReport {
            ok: all_critical_pass,
            action: "doctor",
            checks,
        };
        println!(
            "{}",
            serde_json::to_string(&report).expect("doctor report serialization")
        );
    } else {
        for check in &checks {
            let icon = if check.passed {
                "\x1b[32m✓\x1b[0m"
            } else if check.severity == Severity::Critical {
                "\x1b[31m✗\x1b[0m"
            } else {
                "\x1b[33m!\x1b[0m"
            };
            eprintln!("{icon} {}: {}", check.name, check.message);
        }
        eprintln!();
        if all_critical_pass {
            eprintln!("All critical checks passed.");
        } else {
            eprintln!("Some critical checks failed. Run `tenki doctor --json` for details.");
        }
    }

    all_critical_pass
}

/// Check that the config file exists.
fn check_config_file() -> CheckResult {
    let path = crate::paths::config_file();
    let exists = path.exists();
    CheckResult {
        name: "config_file",
        severity: Severity::Warning,
        passed: exists,
        message: if exists {
            format!("{}", path.display())
        } else {
            format!("{} not found — using defaults", path.display())
        },
    }
}

/// Check that the database file exists.
fn check_database() -> CheckResult {
    let path = crate::paths::db_path();
    let exists = path.exists();
    CheckResult {
        name: "database",
        severity: Severity::Critical,
        passed: exists,
        message: if exists {
            format!("{}", path.display())
        } else {
            format!("{} not found — run `tenki init`", path.display())
        },
    }
}

/// Check agent backend configuration and binary availability.
fn check_agent_backend(cfg: &app_config::AppConfig) -> CheckResult {
    let backend = &cfg.agent.backend;
    let binary = resolve_agent_binary(backend, cfg.agent.command.as_deref());

    let found = which_binary(binary);
    CheckResult {
        name: "agent_backend",
        severity: Severity::Critical,
        passed: found,
        message: if found {
            format!("{backend} ({binary})")
        } else {
            format!("{backend} — `{binary}` not found on PATH. Install it or change `agent.backend`.")
        },
    }
}

/// Map backend name to the expected binary.
fn resolve_agent_binary<'a>(backend: &'a str, command_override: Option<&'a str>) -> &'a str {
    if let Some(cmd) = command_override {
        return cmd;
    }
    match backend {
        "claude" => "claude",
        "kiro" | "kiro-acp" => "kiro-cli",
        "gemini" => "gemini",
        "codex" => "codex",
        "amp" => "amp",
        "copilot" => "copilot",
        "opencode" => "opencode",
        "pi" => "pi",
        "roo" => "roo",
        _ => backend,
    }
}

/// Check that opencli is on PATH.
fn check_opencli() -> CheckResult {
    let found = which_binary("opencli");
    CheckResult {
        name: "opencli",
        severity: Severity::Critical,
        passed: found,
        message: if found {
            "opencli found".to_string()
        } else {
            "`opencli` not found on PATH — required for `tenki discover`".to_string()
        },
    }
}

/// Check resume repository configuration.
fn check_resume_config(cfg: &app_config::AppConfig) -> CheckResult {
    let r = &cfg.resume;
    let mut missing = Vec::new();
    if r.repo_path.is_none() {
        missing.push("resume.repo_path");
    }
    if r.build_command.is_none() {
        missing.push("resume.build_command");
    }
    if r.output_path.is_none() {
        missing.push("resume.output_path");
    }

    let passed = missing.is_empty();
    CheckResult {
        name: "resume_config",
        severity: Severity::Warning,
        passed,
        message: if passed {
            format!("repo={}", r.repo_path.as_deref().unwrap_or(""))
        } else {
            format!("missing: {} — needed for resume export", missing.join(", "))
        },
    }
}

/// Check job search preferences.
fn check_preferences(cfg: &app_config::AppConfig) -> CheckResult {
    let p = &cfg.preferences;
    let mut missing = Vec::new();
    if p.query.is_none() {
        missing.push("preferences.query");
    }
    if p.location.is_none() {
        missing.push("preferences.location");
    }
    if p.sources.is_empty() {
        missing.push("preferences.sources");
    }

    let passed = missing.is_empty();
    CheckResult {
        name: "preferences",
        severity: Severity::Warning,
        passed,
        message: if passed {
            format!(
                "query={} location={} sources={}",
                p.query.as_deref().unwrap_or(""),
                p.location.as_deref().unwrap_or(""),
                p.sources.join(",")
            )
        } else {
            format!(
                "missing: {} — set for `pipeline run` defaults",
                missing.join(", ")
            )
        },
    }
}

/// Check if a binary is available on PATH.
fn which_binary(name: &str) -> bool {
    std::process::Command::new("which")
        .arg(name)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}
