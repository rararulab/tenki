//! Integration tests for the `tailor` command.

mod common;

use common::{tenki_initialized, tenki_with};
use predicates::prelude::*;

#[test]
fn tailor_keyword_fallback() {
    let tmp = tenki_initialized();

    // Add an application with JD text
    let add_json = common::run_json(tenki_with(&tmp).args([
        "app",
        "add",
        "--company",
        "Acme Corp",
        "--position",
        "Rust Developer",
        "--jd-text",
        "We need experience in Rust, Python, Docker, and Kubernetes for backend services",
        "--json",
    ]));
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

    // Tailor with --json (should fall back to keyword tailoring since no agent CLI
    // is available)
    let json = common::run_json(tenki_with(&tmp).args(["tailor", short_id, "--json"]));
    assert_eq!(json["ok"], true);
    assert_eq!(json["action"], "tailor");
    assert_eq!(json["method"], "keyword");
    assert!(
        json["headline"]
            .as_str()
            .expect("headline")
            .contains("Rust Developer"),
        "headline should contain position"
    );
    assert!(
        !json["skills"].as_str().expect("skills").is_empty(),
        "skills should not be empty"
    );
    // Matched skills should include Rust, Python, Docker but not Go
    let skills = json["skills"].as_str().expect("skills");
    assert!(skills.contains("Rust"), "skills should contain Rust");
    assert!(skills.contains("Python"), "skills should contain Python");
    assert!(skills.contains("Docker"), "skills should contain Docker");
    assert!(
        !skills.contains("Go"),
        "skills should not contain Go (not in JD)"
    );
}

#[test]
fn tailor_missing_jd_error() {
    let tmp = tenki_initialized();

    // Add an application without JD text
    let add_json = common::run_json(tenki_with(&tmp).args([
        "app",
        "add",
        "--company",
        "NoJD Inc",
        "--position",
        "Engineer",
        "--json",
    ]));
    let short_id = &add_json["id"].as_str().expect("id field")[..8];

    // Tailor should fail with missing JD error
    tenki_with(&tmp)
        .args(["tailor", short_id, "--json"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("missing JD text"));
}
