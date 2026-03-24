//! Integration tests for `tenki pipeline`.

mod common;
use std::process::Command;

use serde_json::Value;

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
    let tmp = common::tenki_initialized();
    let output = common::tenki_with(&tmp)
        .args(["pipeline", "run", "--json"])
        .output()
        .expect("failed to run tenki");
    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("preferences.query"));
}

#[test]
fn pipeline_run_json_contains_applications_and_errors() {
    let tmp = common::tenki_initialized();
    let output = common::tenki_with(&tmp)
        .args([
            "pipeline",
            "run",
            "--json",
            "--query",
            "rust developer",
            "--sources",
            "linkedin",
            "--skip-tailor",
            "--skip-export",
        ])
        .output()
        .expect("failed to run tenki");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Pipeline may fail at discover step (no opencli in CI).
    // If it produced a successful pipeline JSON with `applications`, verify shape.
    // Otherwise the failure must be an opencli/discover error.
    let has_applications = serde_json::from_str::<Value>(&stdout)
        .ok()
        .and_then(|json| json.get("applications").and_then(Value::as_array).cloned())
        .is_some();

    if has_applications {
        let json: Value = serde_json::from_str(&stdout).unwrap();
        assert!(json["applications"].is_array());
        assert!(json["errors"].is_array());
    } else {
        // Pipeline failed before producing summary — expect opencli/discover error.
        let stderr = String::from_utf8_lossy(&output.stderr);
        let combined = format!("{stdout}{stderr}");
        assert!(
            combined.contains("opencli")
                || combined.contains("discover")
                || combined.contains("OPENCLI")
                || combined.contains("Discover")
                || combined.contains("OpenCli"),
            "unexpected error output: {combined}"
        );
    }
}
