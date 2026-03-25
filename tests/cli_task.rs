mod common;
use predicates::prelude::*;

#[test]
fn task_add_returns_json_with_id() {
    let tmp = common::tenki_initialized();
    let out = common::tenki_with(&tmp)
        .args(["app", "add", "--company", "X", "--position", "Y", "--json"])
        .output()
        .expect("run");
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).expect("parse");
    let app_id = &v["id"].as_str().expect("id")[..8];

    // Add task and verify JSON contains id
    let out = common::tenki_with(&tmp)
        .args([
            "task",
            "add",
            "--app-id",
            app_id,
            "--type",
            "todo",
            "Send thank-you email",
            "--json",
        ])
        .output()
        .expect("run");
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).expect("parse json");
    assert!(v.get("id").is_some(), "response should contain 'id'");
}

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
            "task",
            "add",
            "--app-id",
            app_id,
            "--type",
            "prep",
            "Review system design",
            "--json",
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

#[test]
fn task_update_reflected_in_list() {
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
            "task",
            "add",
            "--app-id",
            app_id,
            "--type",
            "todo",
            "Original title",
            "--json",
        ])
        .output()
        .expect("run");
    assert!(
        out.status.success(),
        "task add failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap_or_else(|e| {
        panic!(
            "parse: {e}\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        )
    });
    let tid = &v["id"].as_str().expect("id")[..8];

    // Update title and add notes
    common::tenki_with(&tmp)
        .args([
            "task",
            "update",
            tid,
            "--title",
            "Updated title",
            "--notes",
            "Some extra context",
            "--json",
        ])
        .assert()
        .success();

    // Verify update is reflected in list
    common::tenki_with(&tmp)
        .args(["task", "list", app_id, "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Updated title"));
}

#[test]
fn task_add_with_due_date() {
    let tmp = common::tenki_initialized();
    let out = common::tenki_with(&tmp)
        .args(["app", "add", "--company", "X", "--position", "Y", "--json"])
        .output()
        .expect("run");
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).expect("parse");
    let app_id = &v["id"].as_str().expect("id")[..8];

    // Add task with due date
    let out = common::tenki_with(&tmp)
        .args([
            "task",
            "add",
            "--app-id",
            app_id,
            "--type",
            "follow-up",
            "--due-date",
            "2026-04-01",
            "Follow up with recruiter",
            "--json",
        ])
        .output()
        .expect("run");
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).expect("parse json");
    assert!(v.get("id").is_some(), "response should contain 'id'");

    // Verify due date and title appear in list
    common::tenki_with(&tmp)
        .args(["task", "list", app_id, "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Follow up with recruiter"))
        .stdout(predicate::str::contains("2026-04-01"));
}

#[test]
fn task_done_reflected_in_list() {
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
            "task",
            "add",
            "--app-id",
            app_id,
            "--type",
            "prep",
            "Practice coding",
            "--json",
        ])
        .output()
        .expect("run");
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).expect("parse");
    let tid = &v["id"].as_str().expect("id")[..8];

    // Mark done
    common::tenki_with(&tmp)
        .args(["task", "done", tid, "--json"])
        .assert()
        .success();

    // Verify is_completed is true in list output
    common::tenki_with(&tmp)
        .args(["task", "list", app_id, "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"is_completed\":true"));
}

#[test]
fn task_list_all_pending() {
    let tmp = common::tenki_initialized();
    let out = common::tenki_with(&tmp)
        .args(["app", "add", "--company", "X", "--position", "Y", "--json"])
        .output()
        .expect("run");
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).expect("parse");
    let app_id = &v["id"].as_str().expect("id")[..8];

    // Add two tasks
    common::tenki_with(&tmp)
        .args([
            "task", "add", "--app-id", app_id, "--type", "todo", "Task A", "--json",
        ])
        .assert()
        .success();
    common::tenki_with(&tmp)
        .args([
            "task", "add", "--app-id", app_id, "--type", "prep", "Task B", "--json",
        ])
        .assert()
        .success();

    // List without app_id filter (all pending)
    common::tenki_with(&tmp)
        .args(["task", "list", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Task A"))
        .stdout(predicate::str::contains("Task B"));
}
