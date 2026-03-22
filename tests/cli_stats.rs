mod common;
use predicates::prelude::*;

#[test]
fn stats_counts_applications() {
    let tmp = common::tenki_initialized();
    common::tenki_with(&tmp)
        .args(["app", "add", "--company", "A", "--position", "X", "--json"])
        .assert()
        .success();
    common::tenki_with(&tmp)
        .args([
            "app", "add", "--company", "B", "--position", "Y", "--status", "applied", "--json",
        ])
        .assert()
        .success();

    common::tenki_with(&tmp)
        .args(["stats", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"total\":2"));
}
