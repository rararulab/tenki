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
use tempfile::TempDir;

type DynResult<T> = Result<T, Box<dyn std::error::Error>>;

const SYNTHETIC_SKILLS: &str = "Python,FastAPI,LLM,RAG,Prompt Engineering,Vector Database,Docker";
const SYNTHETIC_NOTE: &str =
    "Synthetic profile: 3 years Python engineer targeting Tokyo LLM/AI roles";

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

#[derive(Debug, Clone)]
struct DemoOptions {
    query:    String,
    location: String,
    source:   String,
    top_n:    String,
    keep_tmp: bool,
}

#[derive(Debug)]
struct DemoRuntime {
    data_tmp:        TempDir,
    resume_tmp:      TempDir,
    data_dir:        PathBuf,
    resume_template: PathBuf,
    resume_repo:     PathBuf,
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> DynResult<()> {
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

fn ensure_command_exists(command: &str, probe_arg: &str) -> DynResult<()> {
    match Command::new(command).arg(probe_arg).output() {
        Ok(_) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            Err(format!("required command not found on PATH: {command}").into())
        }
        Err(e) => Err(format!("failed to execute {command}: {e}").into()),
    }
}

fn ensure_prerequisites() -> DynResult<()> {
    for (cmd, arg) in [
        ("tenki", "--help"),
        ("opencli", "--help"),
        ("git", "--version"),
        ("make", "--version"),
        ("typst", "--version"),
    ] {
        ensure_command_exists(cmd, arg)?;
    }
    Ok(())
}

fn run_tenki(data_dir: &Path, args: &[&str]) -> DynResult<Output> {
    let output = Command::new("tenki")
        .args(args)
        .env("TENKI_DATA_DIR", data_dir)
        .output()?;
    Ok(output)
}

fn run_tenki_checked(data_dir: &Path, args: &[&str]) -> DynResult<String> {
    let output = run_tenki(data_dir, args)?;
    if output.status.success() {
        Ok(String::from_utf8(output.stdout)?)
    } else {
        Err(Box::new(CmdError {
            args:   args.iter().map(std::string::ToString::to_string).collect(),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        }))
    }
}

fn run_tenki_json(data_dir: &Path, args: &[&str]) -> DynResult<Value> {
    let stdout = run_tenki_checked(data_dir, args)?;
    Ok(serde_json::from_str(&stdout)?)
}

fn run_git(repo: &Path, args: &[&str]) -> DynResult<()> {
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

fn run_make_pdf(repo: &Path) -> DynResult<()> {
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

fn load_options() -> DemoOptions {
    DemoOptions {
        query:    env::var("QUERY").unwrap_or_else(|_| "python llm ai".to_string()),
        location: env::var("LOCATION").unwrap_or_else(|_| "Tokyo".to_string()),
        source:   env::var("SOURCE").unwrap_or_else(|_| "linkedin".to_string()),
        top_n:    env::var("TOP_N").unwrap_or_else(|_| "10".to_string()),
        keep_tmp: env::var("KEEP_TMP").map(|v| v == "1").unwrap_or(false),
    }
}

fn init_runtime() -> DynResult<DemoRuntime> {
    let data_tmp = tempfile::tempdir()?;
    let resume_tmp = tempfile::tempdir()?;
    let data_dir = data_tmp.path().to_path_buf();
    let resume_template =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/fake_resume_repo");
    let resume_repo = resume_tmp.path().join("fake-resume-repo");

    Ok(DemoRuntime {
        data_tmp,
        resume_tmp,
        data_dir,
        resume_template,
        resume_repo,
    })
}

fn print_banner(options: &DemoOptions, runtime: &DemoRuntime) {
    println!("=== tenki pre-application example ===");
    println!(
        "query={} | location={} | source={} | top_n={}",
        options.query, options.location, options.source, options.top_n
    );
    println!("TENKI_DATA_DIR={}", runtime.data_dir.display());
    println!();
}

fn create_fake_resume_repo(runtime: &DemoRuntime) -> DynResult<()> {
    println!("--- Step 1: Create fake resume repo (Typst + Makefile + real PDF) ---");
    if !runtime.resume_template.exists() {
        return Err(format!(
            "resume template folder missing: {}",
            runtime.resume_template.display()
        )
        .into());
    }

    copy_dir_recursive(&runtime.resume_template, &runtime.resume_repo)?;
    run_git(&runtime.resume_repo, &["init", "-q"])?;
    run_git(
        &runtime.resume_repo,
        &["config", "user.name", "Tenki Example"],
    )?;
    run_git(
        &runtime.resume_repo,
        &["config", "user.email", "tenki-example@example.local"],
    )?;
    run_git(&runtime.resume_repo, &["add", "."])?;
    run_git(
        &runtime.resume_repo,
        &["commit", "-q", "-m", "init fake resume repo template"],
    )?;

    run_make_pdf(&runtime.resume_repo)?;
    let pdf_path = runtime.resume_repo.join("build/resume.pdf");
    if !pdf_path.is_file() {
        return Err(format!("expected rendered PDF missing: {}", pdf_path.display()).into());
    }

    println!("resume_repo={}", runtime.resume_repo.display());
    println!("rendered_pdf={}", pdf_path.display());
    println!();
    Ok(())
}

fn set_tenki_config(data_dir: &Path, key: &str, value: &str) -> DynResult<()> {
    run_tenki_checked(data_dir, &["config", "set", key, value])?;
    Ok(())
}

fn init_tenki_and_preferences(runtime: &DemoRuntime, options: &DemoOptions) -> DynResult<()> {
    println!("--- Step 2: Initialize tenki + configure preferences ---");
    run_tenki_checked(&runtime.data_dir, &["init"])?;
    set_tenki_config(
        &runtime.data_dir,
        "resume.repo_path",
        &runtime.resume_repo.to_string_lossy(),
    )?;
    set_tenki_config(&runtime.data_dir, "resume.build_command", "make pdf")?;
    set_tenki_config(&runtime.data_dir, "resume.output_path", "build/resume.pdf")?;
    set_tenki_config(&runtime.data_dir, "preferences.query", &options.query)?;
    set_tenki_config(&runtime.data_dir, "preferences.location", &options.location)?;
    set_tenki_config(&runtime.data_dir, "preferences.sources", &options.source)?;

    // Force analyze/tailor keyword fallback so this example doesn't require an
    // agent CLI.
    set_tenki_config(&runtime.data_dir, "agent.backend", "not-a-real-backend")?;
    println!();
    Ok(())
}

fn discover_jobs(runtime: &DemoRuntime, options: &DemoOptions) -> DynResult<()> {
    println!("--- Step 3: Discover real jobs via opencli ---");
    let discover = run_tenki_json(
        &runtime.data_dir,
        &[
            "discover",
            "--source",
            &options.source,
            "--query",
            &options.query,
            "--location",
            &options.location,
            "--json",
        ],
    )?;
    println!("{discover}");
    println!();
    Ok(())
}

fn inject_synthetic_profile(runtime: &DemoRuntime) -> DynResult<()> {
    println!("--- Step 4: Inject synthetic profile into discovered jobs ---");
    let discovered_apps = run_tenki_json(
        &runtime.data_dir,
        &["app", "list", "--status", "discovered", "--json"],
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
            &runtime.data_dir,
            &[
                "app",
                "update",
                short_id,
                "--skills",
                SYNTHETIC_SKILLS,
                "--notes",
                SYNTHETIC_NOTE,
                "--status",
                "bookmarked",
                "--json",
            ],
        )?;
        println!("profile injected: {short_id}");
    }
    println!();
    Ok(())
}

fn score_and_tailor(runtime: &DemoRuntime, options: &DemoOptions) -> DynResult<()> {
    println!("--- Step 5: Score + tailor (stop before export/apply) ---");
    let analyze = run_tenki_json(
        &runtime.data_dir,
        &["analyze", "--unscored", "--top-n", &options.top_n, "--json"],
    )?;
    let tailor = run_tenki_json(
        &runtime.data_dir,
        &[
            "tailor",
            "--untailored",
            "--top-n",
            &options.top_n,
            "--json",
        ],
    )?;
    println!("analyze: {analyze}");
    println!("tailor:  {tailor}");
    println!();
    Ok(())
}

fn print_preapply_snapshot(runtime: &DemoRuntime) -> DynResult<()> {
    println!("--- Step 6: Pre-application review snapshot ---");
    let final_apps = run_tenki_json(&runtime.data_dir, &["app", "list", "--json"])?;
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
    Ok(())
}

fn finalize(runtime: DemoRuntime, keep_tmp: bool) {
    if keep_tmp {
        let kept_data = runtime.data_tmp.keep();
        let kept_resume_root = runtime.resume_tmp.keep();
        let kept_resume_repo = kept_resume_root.join("fake-resume-repo");
        println!("KEEP_TMP=1; temp dirs kept:");
        println!("  TENKI_DATA_DIR={}", kept_data.display());
        println!("  RESUME_REPO={}", kept_resume_repo.display());
    }
}

fn main() -> DynResult<()> {
    ensure_prerequisites()?;

    let options = load_options();
    let runtime = init_runtime()?;

    print_banner(&options, &runtime);
    create_fake_resume_repo(&runtime)?;
    init_tenki_and_preferences(&runtime, &options)?;
    discover_jobs(&runtime, &options)?;
    inject_synthetic_profile(&runtime)?;
    score_and_tailor(&runtime, &options)?;
    print_preapply_snapshot(&runtime)?;
    finalize(runtime, options.keep_tmp);

    Ok(())
}
