//! Centralized path management for application data directories.
//!
//! All paths derive from a single data root, resolved once via `OnceLock`.

use std::{
    path::{Path, PathBuf},
    sync::OnceLock,
};

static DATA_DIR: OnceLock<PathBuf> = OnceLock::new();

/// Root data directory: `~/.{{project-name}}`
pub fn data_dir() -> &'static Path {
    DATA_DIR.get_or_init(|| {
        dirs::home_dir()
            .expect("home directory must be resolvable")
            .join(".{{project-name}}")
    })
}

/// Config file path: `<data>/config.toml`
pub fn config_file() -> PathBuf { data_dir().join("config.toml") }

/// Cache directory: `<data>/cache`
pub fn cache_dir() -> PathBuf { data_dir().join("cache") }
