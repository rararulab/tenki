//! Centralized path management for tenki data directories.

use std::{
    env,
    path::{Path, PathBuf},
    sync::OnceLock,
};

static DATA_DIR: OnceLock<PathBuf> = OnceLock::new();

/// Returns the root data directory for tenki.
///
/// If the `TENKI_DATA_DIR` environment variable is set, its value is used
/// as-is. Otherwise, falls back to `~/.tenki`. The result is cached on
/// first access via `OnceLock`.
pub fn data_dir() -> &'static Path {
    DATA_DIR.get_or_init(|| {
        env::var("TENKI_DATA_DIR").map(PathBuf::from).unwrap_or_else(|_| {
            dirs::home_dir()
                .expect("home directory must be resolvable")
                .join(".tenki")
        })
    })
}

pub fn db_path() -> PathBuf { data_dir().join("tenki.db") }
pub fn config_file() -> PathBuf { data_dir().join("config.toml") }
#[allow(dead_code)]
pub fn cache_dir() -> PathBuf { data_dir().join("cache") }
