//! What the tests of every module share.

use crate::store::MIGRATOR;
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};

/// A database in memory with the migrations applied.
pub async fn memory() -> SqlitePool {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("open in-memory database");
    MIGRATOR.run(&pool).await.expect("apply migrations");
    pool
}
