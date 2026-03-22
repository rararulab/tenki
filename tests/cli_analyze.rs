//! Integration tests for the `tenki analyze` command.

mod common;

use predicates::prelude::*;

#[test]
fn analyze_with_skills_produces_score() {
    let tmp = common::tenki_initialized();

    // Add an application with JD text
    let add_output = common::tenki_with(&tmp)
        .args([
            "app",
            "add",
            "--company",
            "Acme Corp",
            "--position",
            "Senior Engineer",
            "--jd-text",
            "We need a senior Rust developer with experience in async programming, PostgreSQL, \
             Docker, and CI/CD pipelines",
            "--json",
        ])
        .output()
        .expect("run app add");
    assert!(add_output.status.success(), "app add failed");

    let add_json: serde_json::Value =
        serde_json::from_slice(&add_output.stdout).expect("parse add JSON");
    let app_id = add_json["id"].as_str().expect("id field");
    let short_id = &app_id[..8];

    // Update with skills
    common::tenki_with(&tmp)
        .args([
            "app",
            "update",
            short_id,
            "--skills",
            "rust,python,docker,kubernetes,postgresql",
        ])
        .assert()
        .success();

    // Run analyze with --json
    let analyze_output = common::tenki_with(&tmp)
        .args(["analyze", short_id, "--json"])
        .output()
        .expect("run analyze");
    assert!(analyze_output.status.success(), "analyze failed");

    let result: serde_json::Value =
        serde_json::from_slice(&analyze_output.stdout).expect("parse analyze JSON");

    // Verify structure
    assert!(
        result["fitness_score"].is_f64(),
        "fitness_score should be a float"
    );
    let score = result["fitness_score"].as_f64().expect("score");
    assert!(score > 0.0, "score should be > 0 with matching skills");
    assert!(score <= 1.0, "score should be <= 1.0");

    // rust, docker, postgresql should match; python and kubernetes should not
    let matched: Vec<String> = result["matched_skills"]
        .as_array()
        .expect("matched array")
        .iter()
        .map(|v| v.as_str().expect("string").to_string())
        .collect();
    assert!(matched.contains(&"rust".to_string()), "rust should match");
    assert!(
        matched.contains(&"docker".to_string()),
        "docker should match"
    );
    assert!(
        matched.contains(&"postgresql".to_string()),
        "postgresql should match"
    );

    assert!(
        result["unmatched_skills"]
            .as_array()
            .expect("unmatched array")
            .iter()
            .any(|v| v.as_str().expect("string") == "kubernetes"),
        "kubernetes should not match"
    );

    // Verify score is persisted via app show
    let show_output = common::tenki_with(&tmp)
        .args(["app", "show", short_id, "--json"])
        .output()
        .expect("run app show");
    let show_json: serde_json::Value =
        serde_json::from_slice(&show_output.stdout).expect("parse show JSON");
    assert_eq!(
        show_json["fitness_score"].as_f64(),
        Some(score),
        "fitness_score should be persisted"
    );
}

#[test]
fn analyze_without_jd_text_fails() {
    let tmp = common::tenki_initialized();

    // Add application without JD text
    let add_output = common::tenki_with(&tmp)
        .args([
            "app",
            "add",
            "--company",
            "No JD Corp",
            "--position",
            "Engineer",
            "--json",
        ])
        .output()
        .expect("run app add");
    assert!(add_output.status.success());

    let add_json: serde_json::Value =
        serde_json::from_slice(&add_output.stdout).expect("parse JSON");
    let short_id = &add_json["id"].as_str().expect("id")[..8];

    // Analyze should fail
    common::tenki_with(&tmp)
        .args(["analyze", short_id])
        .assert()
        .failure()
        .stderr(predicate::str::contains("no JD text"));
}

#[test]
fn analyze_without_skills_returns_zero() {
    let tmp = common::tenki_initialized();

    // Add application with JD text but no skills
    let add_output = common::tenki_with(&tmp)
        .args([
            "app",
            "add",
            "--company",
            "Skills Corp",
            "--position",
            "Developer",
            "--jd-text",
            "Looking for a Rust developer",
            "--json",
        ])
        .output()
        .expect("run app add");
    assert!(add_output.status.success());

    let add_json: serde_json::Value =
        serde_json::from_slice(&add_output.stdout).expect("parse JSON");
    let short_id = &add_json["id"].as_str().expect("id")[..8];

    let analyze_output = common::tenki_with(&tmp)
        .args(["analyze", short_id, "--json"])
        .output()
        .expect("run analyze");
    assert!(analyze_output.status.success());

    let result: serde_json::Value =
        serde_json::from_slice(&analyze_output.stdout).expect("parse JSON");
    assert_eq!(result["fitness_score"].as_f64(), Some(0.0));
    assert_eq!(
        result["notes"].as_str(),
        Some("No skills listed for comparison.")
    );
}
