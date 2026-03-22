mod common;
use predicates::prelude::*;

#[test]
fn interview_lifecycle() {
    let tmp = common::tenki_initialized();
    // Add app
    let out = common::tenki_with(&tmp)
        .args(["app", "add", "--company", "X", "--position", "Y", "--json"])
        .output()
        .expect("run");
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).expect("parse");
    let app_id = &v["id"].as_str().expect("id")[..8];

    // Add interview
    let out = common::tenki_with(&tmp)
        .args([
            "interview", "add", "--app-id", app_id, "--round", "1", "--type", "technical", "--json",
        ])
        .output()
        .expect("run");
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).expect("parse");
    let iid = &v["id"].as_str().expect("id")[..8];

    // Update
    common::tenki_with(&tmp)
        .args([
            "interview", "update", iid, "--status", "completed", "--outcome", "pass", "--json",
        ])
        .assert()
        .success();

    // Add note
    common::tenki_with(&tmp)
        .args(["interview", "note", iid, "Great conversation", "--json"])
        .assert()
        .success();

    // List
    common::tenki_with(&tmp)
        .args(["interview", "list", app_id, "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("completed"));
}
