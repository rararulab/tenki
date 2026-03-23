//! Integration tests for `tenki discover`.

use std::process::Command;

fn tenki() -> Command {
    Command::new(env!("CARGO_BIN_EXE_tenki"))
}

#[test]
fn discover_missing_query_shows_error() {
    let output = tenki()
        .args(["discover", "--json"])
        .output()
        .expect("failed to run tenki");
    assert!(!output.status.success());
}

#[test]
fn discover_help_shows_options() {
    let output = tenki()
        .args(["discover", "--help"])
        .output()
        .expect("failed to run tenki");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--query"));
    assert!(stdout.contains("--source"));
    assert!(stdout.contains("--location"));
}
