//! `SQLite` store layer.

mod config;
mod db;
mod err;

pub use config::DatabaseConfig;
pub use db::DBStore;
pub use err::Error as StoreError;
