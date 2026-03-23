mod common;
use predicates::prelude::*;

#[test]
fn init_creates_database() {
    let (mut cmd, _tmp) = common::tenki_cmd();
    cmd.arg("init")
        .assert()
        .success()
        .stderr(predicate::str::contains("initialized"));
}

#[test]
fn app_add_and_list_json() {
    let tmp = common::tenki_initialized();
    // Add
    common::tenki_with(&tmp)
        .args([
            "app",
            "add",
            "--company",
            "Acme",
            "--position",
            "SRE",
            "--json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"id\""));
    // List
    common::tenki_with(&tmp)
        .args(["app", "list", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Acme"));
}

#[test]
fn app_add_default_bookmarked_has_null_stage() {
    let tmp = common::tenki_initialized();
    common::tenki_with(&tmp)
        .args([
            "app",
            "add",
            "--company",
            "NullStageCo",
            "--position",
            "Python Engineer",
            "--json",
        ])
        .assert()
        .success();

    let output = common::tenki_with(&tmp)
        .args(["app", "list", "--company", "NullStageCo", "--json"])
        .output()
        .expect("run");
    let apps: serde_json::Value = serde_json::from_slice(&output.stdout).expect("parse");
    let app = &apps.as_array().expect("array")[0];
    assert_eq!(app["status"], "bookmarked");
    assert!(app["stage"].is_null());
}

#[test]
fn app_add_applied_status_sets_applied_stage() {
    let tmp = common::tenki_initialized();
    common::tenki_with(&tmp)
        .args([
            "app",
            "add",
            "--company",
            "AppliedStageCo",
            "--position",
            "ML Engineer",
            "--status",
            "applied",
            "--json",
        ])
        .assert()
        .success();

    let output = common::tenki_with(&tmp)
        .args(["app", "list", "--company", "AppliedStageCo", "--json"])
        .output()
        .expect("run");
    let apps: serde_json::Value = serde_json::from_slice(&output.stdout).expect("parse");
    let app = &apps.as_array().expect("array")[0];
    assert_eq!(app["status"], "applied");
    assert_eq!(app["stage"], "applied");
}

#[test]
fn app_add_discovered_status_has_null_stage() {
    let tmp = common::tenki_initialized();
    common::tenki_with(&tmp)
        .args([
            "app",
            "add",
            "--company",
            "DiscoveredStageCo",
            "--position",
            "LLM Engineer",
            "--status",
            "discovered",
            "--json",
        ])
        .assert()
        .success();

    let output = common::tenki_with(&tmp)
        .args(["app", "list", "--company", "DiscoveredStageCo", "--json"])
        .output()
        .expect("run");
    let apps: serde_json::Value = serde_json::from_slice(&output.stdout).expect("parse");
    let app = &apps.as_array().expect("array")[0];
    assert_eq!(app["status"], "discovered");
    assert!(app["stage"].is_null());
}

#[test]
fn app_show_update_delete() {
    let tmp = common::tenki_initialized();
    // Add
    let output = common::tenki_with(&tmp)
        .args([
            "app",
            "add",
            "--company",
            "TestCo",
            "--position",
            "Dev",
            "--json",
        ])
        .output()
        .expect("run");
    let v: serde_json::Value = serde_json::from_slice(&output.stdout).expect("parse");
    let id = v["id"].as_str().expect("id");
    let short = &id[..8];

    // Show
    common::tenki_with(&tmp)
        .args(["app", "show", short, "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("TestCo"));

    // Update
    common::tenki_with(&tmp)
        .args(["app", "update", short, "--status", "applied", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("applied"));

    // Delete
    common::tenki_with(&tmp)
        .args(["app", "delete", short, "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("deleted"));

    // List empty
    common::tenki_with(&tmp)
        .args(["app", "list", "--json"])
        .assert()
        .success()
        .stdout("[]\n");
}

#[test]
fn app_list_with_filters() {
    let tmp = common::tenki_initialized();
    common::tenki_with(&tmp)
        .args([
            "app",
            "add",
            "--company",
            "Google",
            "--position",
            "SRE",
            "--source",
            "linkedin",
            "--json",
        ])
        .assert()
        .success();
    common::tenki_with(&tmp)
        .args([
            "app",
            "add",
            "--company",
            "Meta",
            "--position",
            "SWE",
            "--source",
            "referral",
            "--json",
        ])
        .assert()
        .success();
    // Filter by source
    common::tenki_with(&tmp)
        .args(["app", "list", "--source", "linkedin", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Google"))
        .stdout(predicate::str::contains("Meta").not());
}

#[test]
fn app_add_rejects_invalid_url() {
    let tmp = common::tenki_initialized();
    common::tenki_with(&tmp)
        .args([
            "app",
            "add",
            "--company",
            "X",
            "--position",
            "Y",
            "--jd-url",
            "not-a-url",
            "--json",
        ])
        .assert()
        .failure()
        .stdout(predicate::str::contains("invalid URL"));
}

#[test]
fn interview_add_rejects_invalid_date() {
    let tmp = common::tenki_initialized();
    let out = common::tenki_with(&tmp)
        .args(["app", "add", "--company", "X", "--position", "Y", "--json"])
        .output()
        .expect("run");
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).expect("parse");
    let app_id = &v["id"].as_str().expect("id")[..8];

    common::tenki_with(&tmp)
        .args([
            "interview",
            "add",
            "--app-id",
            app_id,
            "--round",
            "1",
            "--scheduled-at",
            "not-a-date",
            "--json",
        ])
        .assert()
        .failure()
        .stdout(predicate::str::contains("invalid date"));
}
