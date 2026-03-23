//! Real pre-application example using tenki + opencli (no mock).
//!
//! Scenario:
//! - Synthetic 3-year Python candidate profile
//! - Target Tokyo LLM/AI roles from `LinkedIn`
//! - Stop after tailor (before export/apply)
//! - Resume template from `examples/fake_resume_repo` (Typst + Makefile)
//!
//! Run:
//! `cargo run --example pipeline_demo`
//!
//! Optional env:
//! - `QUERY` (default: `python llm ai`)
//! - `LOCATION` (default: `Tokyo`)
//! - `SOURCE` (default: `linkedin`)
//! - `TOP_N` (default: `10`)
//! - `KEEP_TMP=1` to keep temp dirs for inspection

use std::{
    env, fmt, fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

use serde_json::Value;

#[derive(Debug)]
struct CmdError {
    args:   Vec<String>,
    stdout: String,
    stderr: String,
}

impl fmt::Display for CmdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "tenki command failed: tenki {}\nstdout:\n{}\nstderr:\n{}",
            self.args.join(" "),
            self.stdout,
            self.stderr
        )
    }
}

impl std::error::Error for CmdError {}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        let file_type = entry.file_type()?;

        if file_type.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else if file_type.is_file() {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

fn ensure_command_exists(command: &str, probe_arg: &str) -> Result<(), Box<dyn std::error::Error>> {
    match Command::new(command).arg(probe_arg).output() {
        Ok(_) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            Err(format!("required command not found on PATH: {command}").into())
        }
        Err(e) => Err(format!("failed to execute {command}: {e}").into()),
    }
}

fn run_tenki(data_dir: &Path, args: &[String]) -> Result<Output, Box<dyn std::error::Error>> {
    let output = Command::new("tenki")
        .args(args)
        .env("TENKI_DATA_DIR", data_dir)
        .output()?;
    Ok(output)
}

fn run_tenki_checked(
    data_dir: &Path,
    args: &[String],
) -> Result<String, Box<dyn std::error::Error>> {
    let output = run_tenki(data_dir, args)?;
    if output.status.success() {
        Ok(String::from_utf8(output.stdout)?)
    } else {
        Err(Box::new(CmdError {
            args:   args.to_vec(),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        }))
    }
}

fn run_tenki_json(data_dir: &Path, args: &[String]) -> Result<Value, Box<dyn std::error::Error>> {
    let stdout = run_tenki_checked(data_dir, args)?;
    Ok(serde_json::from_str(&stdout)?)
}

fn run_git(repo: &Path, args: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new("git").args(args).current_dir(repo).output()?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr)
        )
        .into())
    }
}

fn run_make_pdf(repo: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new("make").arg("pdf").current_dir(repo).output()?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "make pdf failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into())
    }
}

#[allow(clippy::too_many_lines)] // Example script-style flow is intentionally linear and verbose.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    ensure_command_exists("tenki", "--help")?;
    ensure_command_exists("opencli", "--help")?;
    ensure_command_exists("git", "--version")?;
    ensure_command_exists("make", "--version")?;
    ensure_command_exists("typst", "--version")?;

    let query = env::var("QUERY").unwrap_or_else(|_| "python llm ai".to_string());
    let location = env::var("LOCATION").unwrap_or_else(|_| "Tokyo".to_string());
    let source = env::var("SOURCE").unwrap_or_else(|_| "linkedin".to_string());
    let top_n = env::var("TOP_N").unwrap_or_else(|_| "10".to_string());
    let keep_tmp = env::var("KEEP_TMP").map(|v| v == "1").unwrap_or(false);

    let data_tmp = tempfile::tempdir()?;
    let resume_tmp = tempfile::tempdir()?;
    let data_dir = data_tmp.path().to_path_buf();
    let resume_template =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/fake_resume_repo");
    let resume_repo = resume_tmp.path().join("fake-resume-repo");

    println!("=== tenki pre-application example ===");
    println!("query={query} | location={location} | source={source} | top_n={top_n}");
    println!("TENKI_DATA_DIR={}", data_dir.display());
    println!();

    println!("--- Step 1: Create fake resume repo (Typst + Makefile + real PDF) ---");
    if !resume_template.exists() {
        return Err(format!(
            "resume template folder missing: {}",
            resume_template.display()
        )
        .into());
    }
    copy_dir_recursive(&resume_template, &resume_repo)?;
    run_git(&resume_repo, &["init", "-q"])?;
    run_git(&resume_repo, &["config", "user.name", "Tenki Example"])?;
    run_git(
        &resume_repo,
        &["config", "user.email", "tenki-example@example.local"],
    )?;
    run_git(&resume_repo, &["add", "."])?;
    run_git(
        &resume_repo,
        &["commit", "-q", "-m", "init fake resume repo template"],
    )?;
    run_make_pdf(&resume_repo)?;
    let pdf_path = resume_repo.join("build/resume.pdf");
    if !pdf_path.is_file() {
        return Err(format!("expected rendered PDF missing: {}", pdf_path.display()).into());
    }
    println!("resume_repo={}", resume_repo.display());
    println!("rendered_pdf={}", pdf_path.display());
    println!();

    println!("--- Step 2: Initialize tenki + configure preferences ---");
    run_tenki_checked(&data_dir, &[String::from("init")])?;
    run_tenki_checked(
        &data_dir,
        &[
            String::from("config"),
            String::from("set"),
            String::from("resume.repo_path"),
            resume_repo.to_string_lossy().to_string(),
        ],
    )?;
    run_tenki_checked(
        &data_dir,
        &[
            String::from("config"),
            String::from("set"),
            String::from("resume.build_command"),
            String::from("make pdf"),
        ],
    )?;
    run_tenki_checked(
        &data_dir,
        &[
            String::from("config"),
            String::from("set"),
            String::from("resume.output_path"),
            String::from("build/resume.pdf"),
        ],
    )?;
    run_tenki_checked(
        &data_dir,
        &[
            String::from("config"),
            String::from("set"),
            String::from("preferences.query"),
            query.clone(),
        ],
    )?;
    run_tenki_checked(
        &data_dir,
        &[
            String::from("config"),
            String::from("set"),
            String::from("preferences.location"),
            location.clone(),
        ],
    )?;
    run_tenki_checked(
        &data_dir,
        &[
            String::from("config"),
            String::from("set"),
            String::from("preferences.sources"),
            source.clone(),
        ],
    )?;
    // Force analyze/tailor keyword fallback so this example doesn't require an
    // agent CLI.
    run_tenki_checked(
        &data_dir,
        &[
            String::from("config"),
            String::from("set"),
            String::from("agent.backend"),
            String::from("not-a-real-backend"),
        ],
    )?;
    println!();

    println!("--- Step 3: Discover real jobs via opencli ---");
    let discover = run_tenki_json(
        &data_dir,
        &[
            String::from("discover"),
            String::from("--source"),
            source,
            String::from("--query"),
            query,
            String::from("--location"),
            location,
            String::from("--json"),
        ],
    )?;
    println!("{discover}");
    println!();

    println!("--- Step 4: Inject synthetic profile into discovered jobs ---");
    let discovered_apps = run_tenki_json(
        &data_dir,
        &[
            String::from("app"),
            String::from("list"),
            String::from("--status"),
            String::from("discovered"),
            String::from("--json"),
        ],
    )?;
    let apps = discovered_apps
        .as_array()
        .ok_or("unexpected JSON format from `tenki app list --json`")?;
    if apps.is_empty() {
        return Err("no discovered jobs found; try a broader QUERY/LOCATION and rerun".into());
    }

    for app in apps {
        let full_id = app
            .get("id")
            .and_then(Value::as_str)
            .ok_or("application missing id")?;
        let short_id = &full_id[..8];
        run_tenki_checked(
            &data_dir,
            &[
                String::from("app"),
                String::from("update"),
                short_id.to_string(),
                String::from("--skills"),
                String::from("Python,FastAPI,LLM,RAG,Prompt Engineering,Vector Database,Docker"),
                String::from("--notes"),
                String::from(
                    "Synthetic profile: 3 years Python engineer targeting Tokyo LLM/AI roles",
                ),
                String::from("--status"),
                String::from("bookmarked"),
                String::from("--json"),
            ],
        )?;
        println!("profile injected: {short_id}");
    }
    println!();

    println!("--- Step 5: Score + tailor (stop before export/apply) ---");
    let analyze = run_tenki_json(
        &data_dir,
        &[
            String::from("analyze"),
            String::from("--unscored"),
            String::from("--top-n"),
            top_n.clone(),
            String::from("--json"),
        ],
    )?;
    let tailor = run_tenki_json(
        &data_dir,
        &[
            String::from("tailor"),
            String::from("--untailored"),
            String::from("--top-n"),
            top_n,
            String::from("--json"),
        ],
    )?;
    println!("analyze: {analyze}");
    println!("tailor:  {tailor}");
    println!();

    println!("--- Step 6: Pre-application review snapshot ---");
    let final_apps = run_tenki_json(
        &data_dir,
        &[
            String::from("app"),
            String::from("list"),
            String::from("--json"),
        ],
    )?;
    let final_apps = final_apps
        .as_array()
        .ok_or("unexpected JSON format from final app list")?;
    for app in final_apps {
        let id = app.get("id").and_then(Value::as_str).unwrap_or("");
        let company = app.get("company").and_then(Value::as_str).unwrap_or("");
        let position = app.get("position").and_then(Value::as_str).unwrap_or("");
        let score = app
            .get("fitness_score")
            .and_then(Value::as_f64)
            .map_or_else(|| "N/A".to_string(), |v| format!("{v:.1}"));
        let tailored = app
            .get("tailored_summary")
            .and_then(Value::as_str)
            .is_some();
        println!(
            "{} | {} | {} | score={} | tailored={}",
            &id[..id.len().min(8)],
            company,
            position,
            score,
            tailored
        );
    }
    println!();
    println!("Done: flow stops here (pre-application). No export/apply executed.");

    if keep_tmp {
        let kept_data: PathBuf = data_tmp.keep();
        let kept_resume_root: PathBuf = resume_tmp.keep();
        let kept_resume_repo = kept_resume_root.join("fake-resume-repo");
        println!("KEEP_TMP=1; temp dirs kept:");
        println!("  TENKI_DATA_DIR={}", kept_data.display());
        println!("  RESUME_REPO={}", kept_resume_repo.display());
    }

    Ok(())
}
