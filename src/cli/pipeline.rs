//! Pipeline CLI command — runs the full discover -> score -> tailor -> export chain.

use snafu::ResultExt as _;

use crate::{
    db::Database,
    error::Result,
    pipeline::{steps, PipelineConfig},
};

/// Run the full pipeline with the given configuration.
pub async fn run(db: &Database, config: &PipelineConfig, json: bool) -> Result<()> {
    let summary = steps::run_pipeline(db, config).await?;

    if json {
        println!(
            "{}",
            serde_json::to_string(&summary).context(crate::error::JsonSnafu)?
        );
    } else {
        eprintln!("\nPipeline complete!");
    }

    Ok(())
}
