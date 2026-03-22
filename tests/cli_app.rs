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
