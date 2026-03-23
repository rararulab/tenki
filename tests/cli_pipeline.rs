//! Integration tests for `tenki pipeline`.

use std::process::Command;

fn tenki() -> Command { Command::new(env!("CARGO_BIN_EXE_tenki")) }

#[test]
fn pipeline_run_help_shows_options() {
    let output = tenki()
        .args(["pipeline", "run", "--help"])
        .output()
        .expect("failed to run tenki");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--query"));
    assert!(stdout.contains("--sources"));
    assert!(stdout.contains("--top-n"));
    assert!(stdout.contains("--min-score"));
    assert!(stdout.contains("--skip-tailor"));
}

#[test]
fn pipeline_run_missing_query_fails() {
    let output = tenki()
        .args(["pipeline", "run", "--json"])
        .output()
        .expect("failed to run tenki");
    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("preferences.query"));
}
