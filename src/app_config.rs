//! Application configuration backed by TOML file.

use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

static APP_CONFIG: OnceLock<AppConfig> = OnceLock::new();

/// Application configuration.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    /// Default values for new applications.
    pub defaults: DefaultsConfig,
    /// Display preferences.
    pub display:  DisplayConfig,
    /// LLM provider configuration for AI-powered features.
    pub llm:      LlmConfig,
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

/// LLM provider configuration for AI-powered features.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LlmConfig {
    /// API key (can also be set via `LLM_API_KEY` env var).
    pub api_key:  Option<String>,
    /// Base URL for OpenAI-compatible API (default: `OpenRouter`).
    pub base_url: String,
    /// Model identifier.
    pub model:    String,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            api_key:  None,
            base_url: "https://openrouter.ai/api/v1".to_string(),
            model:    "google/gemini-2.5-flash".to_string(),
        }
    }
}

impl LlmConfig {
    /// Merge environment variable overrides into a resolved copy.
    ///
    /// Priority: `LLM_API_KEY`, `LLM_BASE_URL`, `LLM_MODEL` env vars
    /// override values from the config file.
    #[must_use]
    pub fn resolve(&self) -> Self {
        Self {
            api_key:  std::env::var("LLM_API_KEY")
                .ok()
                .or_else(|| self.api_key.clone()),
            base_url: std::env::var("LLM_BASE_URL").unwrap_or_else(|_| self.base_url.clone()),
            model:    std::env::var("LLM_MODEL").unwrap_or_else(|_| self.model.clone()),
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
                .unwrap_or_default();
            settings.try_deserialize().unwrap_or_default()
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
    let content = toml::to_string_pretty(cfg).expect("config serialization should not fail");
    std::fs::write(path, content)
}
