mod common;
use predicates::prelude::*;

#[test]
fn stage_set_returns_json_with_expected_fields() {
    let tmp = common::tenki_initialized();
    let app_id = common::add_test_app(&tmp);

    // Set stage and verify JSON fields
    let v = common::run_json(
        common::tenki_with(&tmp).args(["stage", "set", &app_id, "applied", "--json"]),
    );
    assert!(v.get("id").is_some(), "response should contain 'id'");
    assert!(
        v.get("stage").is_some() || v.get("current_stage").is_some(),
        "response should contain stage info"
    );
}

#[test]
fn stage_set_with_note() {
    let tmp = common::tenki_initialized();
    let app_id = common::add_test_app(&tmp);

    // Set stage with note
    common::tenki_with(&tmp)
        .args([
            "stage",
            "set",
            &app_id,
            "recruiter-screen",
            "--note",
            "Passed initial review",
            "--json",
        ])
        .assert()
        .success();

    // Verify note appears in list
    common::tenki_with(&tmp)
        .args(["stage", "list", &app_id, "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Passed initial review"));
}

#[test]
fn stage_transitions() {
    let tmp = common::tenki_initialized();
    let app_id = common::add_test_app(&tmp);

    // Set stage
    common::tenki_with(&tmp)
        .args(["stage", "set", &app_id, "recruiter-screen", "--json"])
        .assert()
        .success();

    common::tenki_with(&tmp)
        .args([
            "stage",
            "set",
            &app_id,
            "technical",
            "--note",
            "Phone screen done",
            "--json",
        ])
        .assert()
        .success();

    // List events
    common::tenki_with(&tmp)
        .args(["stage", "list", &app_id, "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("technical"));
}

#[test]
fn stage_list_shows_full_history() {
    let tmp = common::tenki_initialized();
    let app_id = common::add_test_app(&tmp);

    // Transition through multiple stages
    common::tenki_with(&tmp)
        .args(["stage", "set", &app_id, "applied", "--json"])
        .assert()
        .success();
    common::tenki_with(&tmp)
        .args(["stage", "set", &app_id, "recruiter-screen", "--json"])
        .assert()
        .success();
    common::tenki_with(&tmp)
        .args(["stage", "set", &app_id, "technical", "--json"])
        .assert()
        .success();
    common::tenki_with(&tmp)
        .args(["stage", "set", &app_id, "offer", "--json"])
        .assert()
        .success();

    // List should contain all stages
    let v = common::run_json(
        common::tenki_with(&tmp).args(["stage", "list", &app_id, "--json"]),
    );
    let stdout = v.to_string();
    assert!(stdout.contains("applied"), "should contain applied");
    assert!(
        stdout.contains("recruiter_screen"),
        "should contain recruiter_screen"
    );
    assert!(stdout.contains("technical"), "should contain technical");
    assert!(stdout.contains("offer"), "should contain offer");
}

#[test]
fn stage_set_rejects_invalid_stage() {
    let tmp = common::tenki_initialized();
    let app_id = common::add_test_app(&tmp);

    // Invalid stage value should fail (clap validation)
    common::tenki_with(&tmp)
        .args(["stage", "set", &app_id, "nonexistent-stage", "--json"])
        .assert()
        .failure();
}
