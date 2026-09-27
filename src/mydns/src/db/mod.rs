use anyhow::Context;
use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
    SqlitePool,
};
use std::{str::FromStr, time::Duration};

pub mod blocklist;
pub mod cache;
pub mod migrations;
pub mod records;
pub mod settings;
pub mod users;
pub mod zones;

#[cfg(test)]
mod tests;

/// Initialises the SQLite connection pool and runs all DDL migrations.
pub async fn init(db_path: &str) -> anyhow::Result<SqlitePool> {
    let options = SqliteConnectOptions::from_str(&format!("sqlite://{}?mode=rwc", db_path))
        .with_context(|| format!("Failed to parse SQLite database URL for '{}'", db_path))?
        .journal_mode(SqliteJournalMode::Wal)
        .busy_timeout(Duration::from_secs(5));

    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .with_context(|| format!("Failed to open SQLite database at '{}'", db_path))?;

    migrations::run_migrations(&pool).await?;
    Ok(pool)
}
