//! Integration tests for `tenki analyze`.

mod common;

use predicates::prelude::*;

#[test]
fn analyze_keyword_fallback_produces_json() {
    let tmp = common::tenki_initialized();

    // Add an application with JD text
    common::tenki_with(&tmp)
        .args([
            "app",
            "add",
            "--company",
            "TestCo",
            "--position",
            "Rust Developer",
            "--jd-text",
            "We need a senior Rust and Python developer with Go experience",
            "--json",
        ])
        .assert()
        .success();

    // Get the app ID
    let output = common::tenki_with(&tmp)
        .args(["app", "list", "--json"])
        .output()
        .expect("list should succeed");
    let apps: serde_json::Value = serde_json::from_slice(&output.stdout).expect("valid JSON");
    let id = apps[0]["id"].as_str().expect("id should be a string");
    let short_id = &id[..8];

    // Update with skills
    common::tenki_with(&tmp)
        .args(["app", "update", short_id, "--skills", "Rust, Python, Go"])
        .assert()
        .success();

    // Run analyze with --json (no LLM_API_KEY set, so keyword fallback)
    let analyze_output = common::tenki_with(&tmp)
        .args(["analyze", short_id, "--json"])
        .output()
        .expect("analyze should succeed");
    assert!(analyze_output.status.success(), "analyze should exit 0");

    let result: serde_json::Value =
        serde_json::from_slice(&analyze_output.stdout).expect("valid JSON output");
    assert_eq!(result["method"], "keyword");
    assert!(result["fitness_score"].as_f64().expect("score is f64") > 0.0);
    assert!(result["reason"].as_str().is_some());

    // Verify persistence via app show
    let show_output = common::tenki_with(&tmp)
        .args(["app", "show", short_id, "--json"])
        .output()
        .expect("show should succeed");
    let app: serde_json::Value = serde_json::from_slice(&show_output.stdout).expect("valid JSON");
    assert!(app["fitness_score"].as_f64().is_some());
    assert!(app["fitness_notes"].as_str().is_some());
}

#[test]
fn analyze_missing_jd_text_fails() {
    let tmp = common::tenki_initialized();

    // Add an application without JD text
    common::tenki_with(&tmp)
        .args([
            "app",
            "add",
            "--company",
            "NoCo",
            "--position",
            "Dev",
            "--json",
        ])
        .assert()
        .success();

    let output = common::tenki_with(&tmp)
        .args(["app", "list", "--json"])
        .output()
        .expect("list should succeed");
    let apps: serde_json::Value = serde_json::from_slice(&output.stdout).expect("valid JSON");
    let id = apps[0]["id"].as_str().expect("id");
    let short_id = &id[..8];

    // Analyze should fail because no JD text
    common::tenki_with(&tmp)
        .args(["analyze", short_id])
        .assert()
        .failure()
        .stderr(predicate::str::contains("missing JD text"));
}
