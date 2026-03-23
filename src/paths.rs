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
        env::var("TENKI_DATA_DIR").map_or_else(
            |_| {
                dirs::home_dir()
                    .unwrap_or_else(|| {
                        eprintln!("fatal: unable to determine home directory");
                        std::process::exit(1);
                    })
                    .join(".tenki")
            },
            PathBuf::from,
        )
    })
}

pub fn db_path() -> PathBuf { data_dir().join("tenki.db") }
pub fn config_file() -> PathBuf { data_dir().join("config.toml") }
