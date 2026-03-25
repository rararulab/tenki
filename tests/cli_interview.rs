mod common;
use predicates::prelude::*;

#[test]
fn interview_lifecycle() {
    let tmp = common::tenki_initialized();
    let app_id = common::add_test_app(&tmp);

    // Add interview
    let v = common::run_json(
        common::tenki_with(&tmp).args([
            "interview",
            "add",
            "--app-id",
            &app_id,
            "--round",
            "1",
            "--type",
            "technical",
            "--json",
        ]),
    );
    let iid = &v["id"].as_str().expect("id")[..8];

    // Update
    common::tenki_with(&tmp)
        .args([
            "interview",
            "update",
            iid,
            "--status",
            "completed",
            "--outcome",
            "pass",
            "--json",
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
        .args(["interview", "list", &app_id, "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("completed"));
}
