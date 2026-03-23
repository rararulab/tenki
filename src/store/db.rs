//! Database store wrapping `SQLite` connection pool.

use sqlx::{Sqlite, SqlitePool};

use super::err::Result;

/// Thin wrapper around a `SQLite` connection pool.
#[derive(Clone)]
pub struct DBStore {
    pool: SqlitePool,
}

impl DBStore {
    /// Creates a new store from an existing connection pool.
    pub const fn new(pool: SqlitePool) -> Self { Self { pool } }

    /// Returns a reference to the underlying connection pool.
    pub const fn pool(&self) -> &SqlitePool { &self.pool }

    /// Acquires a single connection from the pool.
    pub async fn acquire(&self) -> Result<sqlx::pool::PoolConnection<Sqlite>> {
        Ok(self.pool.acquire().await?)
    }
}

impl From<DBStore> for SqlitePool {
    fn from(store: DBStore) -> Self { store.pool }
}
