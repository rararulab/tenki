//! Resume export — agent-driven resume editing + PDF build.
//!
//! Flow: read tailored content -> agent edits resume source -> build PDF ->
//! store -> git restore.

use std::path::Path;

use crate::{
    agent::{CliBackend, CliExecutor},
    app_config,
    db::Database,
    domain::Application,
    error::{Result, TenkiError},
};

/// Export a tailored resume PDF for a single application.
///
/// Sends tailored content to the agent backend, which edits the resume source
/// files in `resume.repo_path`. Then runs the build command, reads the output
/// PDF, stores it in the database, and restores the repo to a clean state.
pub async fn export_one(db: &Database, app: &Application) -> Result<()> {
    let cfg = app_config::load();
    let resume_cfg = &cfg.resume;

    let repo_path = resume_cfg
        .repo_path
        .as_deref()
        .ok_or(TenkiError::ResumeConfigMissing)?;
    let build_command = resume_cfg
        .build_command
        .as_deref()
        .ok_or(TenkiError::ResumeConfigMissing)?;
    let output_path = resume_cfg
        .output_path
        .as_deref()
        .ok_or(TenkiError::ResumeConfigMissing)?;

    let repo = Path::new(repo_path);
    if !repo.exists() {
        return Err(TenkiError::BuildCommandFailed {
            message: format!("resume repo not found: {repo_path}"),
        });
    }

    // Build prompt incorporating tailored fields from the application
    let jd_text = app.jd_text.as_deref().unwrap_or("(no JD)");
    let headline = app.tailored_headline.as_deref().unwrap_or(&app.position);
    let summary = app.tailored_summary.as_deref().unwrap_or("");
    let skills = app.tailored_skills.as_deref().unwrap_or("");

    let prompt = format!(
        r"Modify the resume files in this directory for the following job application.

Position: {headline}
Company: {}

Tailored Summary: {summary}
Key Skills: {skills}

Job Description:
{jd_text}

Edit the resume source files to match this job. Keep formatting intact. Be concise.",
        app.company
    );

    // Step 1: Agent edits resume source files inside the repo directory
    let backend_name = &cfg.agent.backend;
    let backend =
        CliBackend::from_name(backend_name).map_err(|e| TenkiError::BuildCommandFailed {
            message: e.to_string(),
        })?;
    let executor = CliExecutor::new(backend);

    let timeout = std::time::Duration::from_secs(u64::from(cfg.agent.idle_timeout_secs));
    let result = executor
        .execute_capture_with_cwd(&prompt, Some(timeout), Some(repo))
        .await
        .map_err(|e| TenkiError::BuildCommandFailed {
            message: e.to_string(),
        })?;

    if !result.success {
        let _ = git_restore(repo);
        return Err(TenkiError::BuildCommandFailed {
            message: format!("agent exited with code {:?}", result.exit_code),
        });
    }

    // Verify the agent actually modified files before building
    if !has_git_changes(repo) {
        let _ = git_restore(repo);
        return Err(TenkiError::BuildCommandFailed {
            message: "agent produced no file changes in resume repo".to_string(),
        });
    }

    // Step 2: Run build command to generate PDF
    let build_output = std::process::Command::new("sh")
        .arg("-c")
        .arg(build_command)
        .current_dir(repo)
        .output()
        .map_err(|e| TenkiError::BuildCommandFailed {
            message: e.to_string(),
        })?;

    if !build_output.status.success() {
        let stderr = String::from_utf8_lossy(&build_output.stderr);
        let _ = git_restore(repo);
        return Err(TenkiError::BuildCommandFailed {
            message: format!("build failed: {stderr}"),
        });
    }

    // Step 3: Read PDF and store in database
    let pdf_path = repo.join(output_path);
    let pdf_bytes = std::fs::read(&pdf_path).map_err(|e| TenkiError::BuildCommandFailed {
        message: format!("cannot read PDF at {}: {e}", pdf_path.display()),
    })?;

    db.store_resume_pdf(&app.id, &pdf_bytes).await?;

    // Step 4: Restore repo to clean state so next export starts fresh
    git_restore(repo)?;

    Ok(())
}

/// Check whether the resume repo has uncommitted changes (i.e. the agent edited files).
fn has_git_changes(repo: &Path) -> bool {
    std::process::Command::new("git")
        .args(["diff", "--stat"])
        .current_dir(repo)
        .output()
        .map(|o| !o.stdout.is_empty())
        .unwrap_or(false)
}

/// Restore the resume repo to a clean state via `git checkout .`.
fn git_restore(repo: &Path) -> Result<()> {
    let output = std::process::Command::new("git")
        .args(["checkout", "."])
        .current_dir(repo)
        .output()
        .map_err(|e| TenkiError::BuildCommandFailed {
            message: e.to_string(),
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(TenkiError::BuildCommandFailed {
            message: format!("git restore failed: {stderr}"),
        });
    }
    Ok(())
}
