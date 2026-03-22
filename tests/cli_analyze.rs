mod common;
use predicates::prelude::*;

#[test]
fn analyze_keyword_fallback_works() {
    let tmp = common::tenki_initialized();

    // Add an application with jd_text
    let output = common::tenki_with(&tmp)
        .args([
            "app",
            "add",
            "--company",
            "Acme Corp",
            "--position",
            "Rust Developer",
            "--jd-text",
            "We need a developer skilled in Rust, Python, and Kubernetes. Experience with \
             distributed systems is a plus.",
            "--json",
        ])
        .output()
        .expect("run add");
    let add_json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("parse add json");
    let id = add_json["id"].as_str().expect("id field");
    let short_id = &id[..8];

    // Update with skills so keyword matching can work
    common::tenki_with(&tmp)
        .args(["app", "update", short_id, "--skills", "Rust,Python,Go"])
        .assert()
        .success();

    // Run analyze --json (agent CLI won't be available, so falls back to keyword)
    let analyze_output = common::tenki_with(&tmp)
        .args(["analyze", short_id, "--json"])
        .output()
        .expect("run analyze");
    assert!(analyze_output.status.success(), "analyze should succeed");

    let result: serde_json::Value =
        serde_json::from_slice(&analyze_output.stdout).expect("parse analyze json");
    assert_eq!(result["method"], "keyword");
    assert!(result["fitness_score"].as_f64().is_some());
    // Rust + Python match = 50 + 5 + 5 = 60
    let score = result["fitness_score"].as_f64().unwrap();
    assert!(
        score >= 50.0,
        "score should be at least 50 (base), got {score}"
    );

    // Verify score persisted via app show --json
    let show_output = common::tenki_with(&tmp)
        .args(["app", "show", short_id, "--json"])
        .output()
        .expect("run show");
    let show_json: serde_json::Value =
        serde_json::from_slice(&show_output.stdout).expect("parse show json");
    assert!(
        show_json["fitness_score"].as_f64().is_some(),
        "fitness_score should be persisted"
    );
}

#[test]
fn analyze_missing_jd_text_errors() {
    let tmp = common::tenki_initialized();

    // Add an application WITHOUT jd_text
    let output = common::tenki_with(&tmp)
        .args([
            "app",
            "add",
            "--company",
            "NoCorp",
            "--position",
            "Tester",
            "--json",
        ])
        .output()
        .expect("run add");
    let add_json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("parse add json");
    let id = add_json["id"].as_str().expect("id field");
    let short_id = &id[..8];

    // Run analyze --json — should fail with missing JD text error
    common::tenki_with(&tmp)
        .args(["analyze", short_id, "--json"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("missing JD text"));
}
