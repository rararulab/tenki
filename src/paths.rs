//! Centralized path management for tenki data directories.

use std::{
    path::{Path, PathBuf},
    sync::OnceLock,
};

static DATA_DIR: OnceLock<PathBuf> = OnceLock::new();

pub fn data_dir() -> &'static Path {
    DATA_DIR.get_or_init(|| {
        dirs::home_dir()
            .expect("home directory must be resolvable")
            .join(".tenki")
    })
}

pub fn db_path() -> PathBuf { data_dir().join("tenki.db") }
pub fn config_file() -> PathBuf { data_dir().join("config.toml") }
#[allow(dead_code)]
pub fn cache_dir() -> PathBuf { data_dir().join("cache") }
