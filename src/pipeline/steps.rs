//! Individual pipeline step implementations.

use crate::{
    cli,
    db::Database,
    domain::ListApplicationParams,
    error::Result,
    extractor::{opencli, DiscoverParams, Extractor},
    pipeline::{PipelineConfig, PipelineSummary},
};

/// Execute the full pipeline: discover → score → tailor → export.
pub async fn run_pipeline(db: &Database, config: &PipelineConfig) -> Result<PipelineSummary> {
    let mut summary = PipelineSummary {
        ok: true,
        action: "pipeline_run",
        discovered: 0,
        imported: 0,
        scored: 0,
        above_threshold: 0,
        tailored: 0,
        exported: 0,
    };

    // Step 1: Discover
    eprintln!("[1/5] Discovering jobs...");
    let extractor = opencli::OpenCliExtractor;
    let params = DiscoverParams::builder()
        .query(config.query.clone())
        .maybe_location(config.location.clone())
        .build();

    let jobs = if config.sources.is_empty() {
        extractor.discover(&params).await?
    } else {
        let mut all = Vec::new();
        for src in &config.sources {
            let found = opencli::search_source(src, &params).await?;
            all.extend(found);
        }
        all
    };

    summary.discovered = jobs.len();
    for job in &jobs {
        if db.import_discovered_job(job).await?.is_some() {
            summary.imported += 1;
        }
    }
    eprintln!(
        "  -> {} found, {} new",
        summary.discovered, summary.imported
    );

    // Step 2: Score all unscored
    eprintln!("[2/5] Scoring unscored applications...");
    let unscored = db.list_unscored().await?;
    for app in &unscored {
        cli::analyze::run(db, &app.id, false, None).await?;
        summary.scored += 1;
    }
    eprintln!("  -> {} scored", summary.scored);

    // Step 3: Filter by min_score, take top_n
    eprintln!(
        "[3/5] Filtering (min_score={}, top_n={})...",
        config.min_score, config.top_n
    );
    let all_apps = db
        .list_applications(&ListApplicationParams::default())
        .await?;
    let mut qualified: Vec<_> = all_apps
        .into_iter()
        .filter(|a| a.fitness_score.unwrap_or(0.0) >= config.min_score)
        .collect();
    qualified.sort_by(|a, b| {
        b.fitness_score
            .unwrap_or(0.0)
            .partial_cmp(&a.fitness_score.unwrap_or(0.0))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    qualified.truncate(config.top_n as usize);
    summary.above_threshold = qualified.len();
    eprintln!("  -> {} above threshold", summary.above_threshold);

    // Step 4: Tailor
    if config.skip_tailor {
        eprintln!("[4/5] Tailoring skipped");
    } else {
        eprintln!("[4/5] Tailoring top applications...");
        for app in &qualified {
            if app.tailored_summary.is_none() {
                cli::tailor::run(db, &app.id, false, None).await?;
                summary.tailored += 1;
            }
        }
        eprintln!("  -> {} tailored", summary.tailored);
    }

    // Step 5: Export tailored resumes as PDFs
    if config.skip_export {
        eprintln!("[5/5] Export skipped");
    } else {
        eprintln!("[5/5] Exporting resumes...");
        for app in &qualified {
            if app.tailored_summary.is_some() {
                let fresh_app = db.get_application(&app.id).await?;
                cli::resume_export::export_one(db, &fresh_app).await?;
                summary.exported += 1;
                eprintln!("  -> {} @ {} done", app.position, app.company);
            }
        }
        eprintln!("  -> {} exported", summary.exported);
    }

    Ok(summary)
}
