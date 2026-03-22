//! Database store wrapping `SQLite` connection pool.

use sqlx::{Sqlite, SqlitePool};

use super::err::Result;

#[derive(Clone)]
pub struct DBStore {
    pool: SqlitePool,
}

impl DBStore {
    pub const fn new(pool: SqlitePool) -> Self { Self { pool } }

    pub const fn pool(&self) -> &SqlitePool { &self.pool }

    pub async fn acquire(&self) -> Result<sqlx::pool::PoolConnection<Sqlite>> {
        Ok(self.pool.acquire().await?)
    }
}

impl From<DBStore> for SqlitePool {
    fn from(store: DBStore) -> Self { store.pool }
}
