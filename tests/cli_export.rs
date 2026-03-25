mod common;
use predicates::prelude::*;

#[test]
fn export_without_flags_prints_usage_hint() {
    let tmp = common::tenki_initialized();
    let app_id = common::add_test_app(&tmp);

    // Export without --typ or --pdf should hint to specify a format
    common::tenki_with(&tmp)
        .args(["export", &app_id])
        .assert()
        .success()
        .stderr(predicate::str::contains("--typ").or(predicate::str::contains("--pdf")));
}

#[test]
fn export_typ_no_resume_stored() {
    let tmp = common::tenki_initialized();
    let v = common::run_json(
        common::tenki_with(&tmp).args([
            "app",
            "add",
            "--company",
            "Acme",
            "--position",
            "SRE",
            "--json",
        ]),
    );
    let app_id = &v["id"].as_str().expect("id")[..8];

    // Export typ when no resume is stored — should succeed but print message
    common::tenki_with(&tmp)
        .args(["export", app_id, "--typ"])
        .assert()
        .success()
        .stderr(predicate::str::contains("No resume typ found"));
}

#[test]
fn export_pdf_no_resume_stored() {
    let tmp = common::tenki_initialized();
    let v = common::run_json(
        common::tenki_with(&tmp).args([
            "app",
            "add",
            "--company",
            "Acme",
            "--position",
            "SRE",
            "--json",
        ]),
    );
    let app_id = &v["id"].as_str().expect("id")[..8];

    // Export pdf when no resume is stored — should succeed but print message
    common::tenki_with(&tmp)
        .args(["export", app_id, "--pdf"])
        .assert()
        .success()
        .stderr(predicate::str::contains("No resume pdf found"));
}

#[test]
fn export_typ_after_import() {
    let tmp = common::tenki_initialized();
    let v = common::run_json(
        common::tenki_with(&tmp).args([
            "app",
            "add",
            "--company",
            "Acme",
            "--position",
            "SRE",
            "--json",
        ]),
    );
    let app_id = &v["id"].as_str().expect("id")[..8];

    // Create a temporary typ file and import it
    let typ_path = tmp.path().join("resume.typ");
    std::fs::write(&typ_path, "#set page(paper: \"a4\")\nHello World").expect("write typ");

    common::tenki_with(&tmp)
        .args(["import", app_id, "--typ", typ_path.to_str().expect("path")])
        .assert()
        .success();

    // Export should now produce a file
    let out_path = tmp.path().join("exported.typ");
    common::tenki_with(&tmp)
        .args([
            "export",
            app_id,
            "--typ",
            "-o",
            out_path.to_str().expect("path"),
        ])
        .assert()
        .success();

    // Verify the exported file exists and has content
    let content = std::fs::read_to_string(&out_path).expect("read exported typ");
    assert!(
        content.contains("Hello World"),
        "exported typ should contain imported content"
    );
}
