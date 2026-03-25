//! Integration tests for the `analyze` command.

mod common;

use common::{tenki_initialized, tenki_with};
use predicates::prelude::*;

#[test]
fn analyze_keyword_fallback() {
    let tmp = tenki_initialized();

    // Add an application with JD text
    let add_json = common::run_json(
        tenki_with(&tmp).args([
            "app",
            "add",
            "--company",
            "Acme Corp",
            "--position",
            "Rust Developer",
            "--jd-text",
            "We need experience in Rust, Python, Docker, and Kubernetes for backend services",
            "--json",
        ]),
    );
    let short_id = &add_json["id"].as_str().expect("id field")[..8];

    // Update with skills
    tenki_with(&tmp)
        .args([
            "app",
            "update",
            short_id,
            "--skills",
            "Rust, Python, Go, Docker",
        ])
        .assert()
        .success();

    // Analyze with --json (should fall back to keyword scoring since no agent CLI
    // is available)
    let json = common::run_json(tenki_with(&tmp).args(["analyze", short_id, "--json"]));
    assert_eq!(json["ok"], true);
    assert_eq!(json["action"], "analyze");
    assert_eq!(json["method"], "keyword");
    assert!(
        json["score"].as_f64().expect("score") > 0.0,
        "score should be positive"
    );
}

#[test]
fn analyze_missing_jd_error() {
    let tmp = tenki_initialized();

    // Add an application without JD text
    let add_json = common::run_json(
        tenki_with(&tmp).args([
            "app",
            "add",
            "--company",
            "NoJD Inc",
            "--position",
            "Engineer",
            "--json",
        ]),
    );
    let short_id = &add_json["id"].as_str().expect("id field")[..8];

    // Analyze should fail with missing JD error
    tenki_with(&tmp)
        .args(["analyze", short_id, "--json"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("missing JD text"));
}
