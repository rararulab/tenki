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
