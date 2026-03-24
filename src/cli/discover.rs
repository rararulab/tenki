//! Discover command — find jobs via `OpenCLI` extractors.

use serde::Serialize;
use snafu::ResultExt as _;

use crate::{
    db::Database,
    error::Result,
    extractor::{DiscoverParams, Extractor, opencli},
};

/// Discover result summary.
#[derive(Debug, Serialize)]
struct DiscoverResult {
    ok:         bool,
    action:     &'static str,
    discovered: usize,
    imported:   usize,
    skipped:    usize,
}

/// Run the discover command.
pub async fn run(
    db: &Database,
    source: Option<&str>,
    query: &str,
    location: Option<&str>,
    limit: Option<u32>,
    json: bool,
) -> Result<()> {
    let params = DiscoverParams::builder()
        .query(query.to_string())
        .maybe_location(location.map(String::from))
        .maybe_limit(limit)
        .build();

    let jobs = if let Some(src) = source {
        if !json {
            eprintln!(
                "[discover] calling opencli source={src} query={query:?} location={location:?} \
                 limit={}",
                limit.map_or_else(|| "default".to_string(), |v| v.to_string())
            );
        }
        let found = opencli::search_source(src, &params).await?;
        if !json {
            eprintln!("[discover] source={src} returned {} jobs", found.len());
        }
        found
    } else {
        let extractor = opencli::OpenCliExtractor;
        let mut all = Vec::new();
        for src in extractor.sources() {
            if !json {
                eprintln!(
                    "[discover] calling opencli source={src} query={query:?} location={location:?} \
                     limit={}",
                    limit.map_or_else(|| "default".to_string(), |v| v.to_string())
                );
            }
            let found = opencli::search_source(src, &params).await?;
            if !json {
                eprintln!("[discover] source={src} returned {} jobs", found.len());
            }
            all.extend(found);
        }
        all
    };

    let discovered = jobs.len();
    let mut imported = 0usize;
    let mut skipped = 0usize;

    for job in &jobs {
        match db.import_discovered_job(job).await? {
            Some(_id) => {
                imported += 1;
                if !json {
                    eprintln!("  + {} @ {}", job.title, job.company);
                }
            }
            None => {
                skipped += 1;
            }
        }
    }

    let result = DiscoverResult {
        ok: true,
        action: "discover",
        discovered,
        imported,
        skipped,
    };

    if json {
        println!(
            "{}",
            serde_json::to_string(&result).context(crate::error::JsonSnafu)?
        );
    } else {
        eprintln!(
            "Discovered {discovered} jobs: {imported} imported, {skipped} duplicates skipped"
        );
    }

    Ok(())
}
