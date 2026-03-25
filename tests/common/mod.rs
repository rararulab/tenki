//! Shared test helpers for integration tests.

#![allow(dead_code)]

use assert_cmd::Command;
use tempfile::TempDir;

/// Create a tenki command with an isolated temp data directory.
pub fn tenki_cmd() -> (Command, TempDir) {
    let tmp = TempDir::new().expect("create temp dir");
    let mut cmd = Command::cargo_bin("tenki").expect("binary exists");
    cmd.env("TENKI_DATA_DIR", tmp.path());
    (cmd, tmp)
}

/// Initialize a temp DB and return the `TempDir` for reuse.
pub fn tenki_initialized() -> TempDir {
    let (mut cmd, tmp) = tenki_cmd();
    cmd.arg("init").assert().success();
    tmp
}

/// Create a command reusing an existing temp dir.
pub fn tenki_with(tmp: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin("tenki").expect("binary exists");
    cmd.env("TENKI_DATA_DIR", tmp.path());
    cmd
}

/// Run a tenki command, assert success, and parse stdout as JSON.
///
/// Panics with full stdout/stderr on failure — prevents the bare
/// "expected value at line 1 column 1" messages that hide root causes.
pub fn run_json(cmd: &mut Command) -> serde_json::Value {
    let out = cmd.output().expect("spawn");
    assert!(
        out.status.success(),
        "command failed (exit {}):\nstderr: {}",
        out.status,
        String::from_utf8_lossy(&out.stderr),
    );
    serde_json::from_slice(&out.stdout).unwrap_or_else(|e| {
        panic!(
            "JSON parse: {e}\nstdout: {:?}\nstderr: {}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr),
        )
    })
}

/// Add a test application and return its 8-char ID prefix.
pub fn add_test_app(tmp: &TempDir) -> String {
    let v = run_json(tenki_with(tmp).args([
        "app",
        "add",
        "--company",
        "X",
        "--position",
        "Y",
        "--json",
    ]));
    v["id"].as_str().expect("id field")[..8].to_string()
}
