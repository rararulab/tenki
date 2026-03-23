//! Application configuration backed by TOML file.

use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

static APP_CONFIG: OnceLock<AppConfig> = OnceLock::new();

/// Application configuration.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    /// Default values for new applications.
    pub defaults:    DefaultsConfig,
    /// Display preferences.
    pub display:     DisplayConfig,
    /// Agent backend configuration.
    pub agent:       crate::agent::AgentConfig,
    /// Resume repository configuration.
    pub resume:      ResumeConfig,
    /// Job search preferences for pipeline defaults.
    pub preferences: JobPreferencesConfig,
}

/// Resume repository configuration for automated PDF generation.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ResumeConfig {
    /// Path to the resume git repository.
    pub repo_path:     Option<String>,
    /// Command to build the resume PDF (e.g. "make pdf").
    pub build_command: Option<String>,
    /// Relative path to the built PDF within the repo.
    pub output_path:   Option<String>,
}

/// Preferred job search filters used by `pipeline run` when flags are omitted.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct JobPreferencesConfig {
    /// Preferred search query (e.g. "rust backend engineer").
    pub query:    Option<String>,
    /// Preferred location filter.
    pub location: Option<String>,
    /// Preferred source platforms (e.g. `["linkedin"]`).
    pub sources:  Vec<String>,
}

/// Default values applied when creating new applications.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DefaultsConfig {
    /// Default status when adding an application (default: "bookmarked").
    pub status: String,
    /// Default source for new applications (e.g. "linkedin").
    pub source: Option<String>,
}

impl Default for DefaultsConfig {
    fn default() -> Self {
        Self {
            status: "bookmarked".to_string(),
            source: None,
        }
    }
}

/// Display preferences for CLI output.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DisplayConfig {
    /// Date format for human-readable output (default: "%Y-%m-%d").
    pub date_format: String,
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            date_format: "%Y-%m-%d".to_string(),
        }
    }
}

/// Load config from TOML file, falling back to defaults.
pub fn load() -> &'static AppConfig {
    APP_CONFIG.get_or_init(|| {
        let path = crate::paths::config_file();
        if path.exists() {
            let settings = config::Config::builder()
                .add_source(config::File::from(path.as_ref()))
                .build()
                .unwrap_or_else(|e| {
                    tracing::warn!(
                        "failed to load config from {}: {e}, using defaults",
                        path.display()
                    );
                    config::Config::default()
                });
            settings.try_deserialize().unwrap_or_else(|e| {
                tracing::warn!("failed to parse config: {e}, using defaults");
                AppConfig::default()
            })
        } else {
            AppConfig::default()
        }
    })
}

/// Save config to TOML file.
pub fn save(cfg: &AppConfig) -> std::io::Result<()> {
    let path = crate::paths::config_file();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let content = toml::to_string_pretty(cfg)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(path, content)
}
