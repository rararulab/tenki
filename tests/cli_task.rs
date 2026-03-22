mod common;
use predicates::prelude::*;

#[test]
fn task_lifecycle() {
    let tmp = common::tenki_initialized();
    let out = common::tenki_with(&tmp)
        .args(["app", "add", "--company", "X", "--position", "Y", "--json"])
        .output()
        .expect("run");
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).expect("parse");
    let app_id = &v["id"].as_str().expect("id")[..8];

    // Add task
    let out = common::tenki_with(&tmp)
        .args([
            "task", "add", "--app-id", app_id, "--type", "prep", "Review system design", "--json",
        ])
        .output()
        .expect("run");
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).expect("parse");
    let tid = &v["id"].as_str().expect("id")[..8];

    // List
    common::tenki_with(&tmp)
        .args(["task", "list", app_id, "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Review system design"));

    // Done
    common::tenki_with(&tmp)
        .args(["task", "done", tid, "--json"])
        .assert()
        .success();

    // Delete
    common::tenki_with(&tmp)
        .args(["task", "delete", tid, "--json"])
        .assert()
        .success();
}
