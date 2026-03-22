mod common;
use predicates::prelude::*;

#[test]
fn stage_transitions() {
    let tmp = common::tenki_initialized();
    let out = common::tenki_with(&tmp)
        .args(["app", "add", "--company", "X", "--position", "Y", "--json"])
        .output()
        .expect("run");
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).expect("parse");
    let app_id = &v["id"].as_str().expect("id")[..8];

    // Set stage
    common::tenki_with(&tmp)
        .args(["stage", "set", app_id, "recruiter-screen", "--json"])
        .assert()
        .success();

    common::tenki_with(&tmp)
        .args([
            "stage",
            "set",
            app_id,
            "technical",
            "--note",
            "Phone screen done",
            "--json",
        ])
        .assert()
        .success();

    // List events
    common::tenki_with(&tmp)
        .args(["stage", "list", app_id, "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("technical"));
}
