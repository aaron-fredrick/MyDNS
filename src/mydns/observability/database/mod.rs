use anyhow::Context;
use sqlx::{sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions}, SqlitePool};
use std::{str::FromStr, time::Duration};

pub mod migrations;

pub struct ObservabilityDatabase { pool: SqlitePool }

impl ObservabilityDatabase {
    pub async fn init(db_path: &str) -> anyhow::Result<Self> {
        let options = SqliteConnectOptions::from_str(&format!("sqlite://{}?mode=rwc", db_path))
            .with_context(|| format!("Failed to parse observability SQLite URL for '{}'", db_path))?
            .journal_mode(SqliteJournalMode::Wal)
            .busy_timeout(Duration::from_secs(5));
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .with_context(|| format!("Failed to open observability database at '{}'", db_path))?;
        migrations::run_migrations(&pool).await?;
        Ok(Self { pool })
    }

    pub fn pool(&self) -> &SqlitePool { &self.pool }

    pub async fn close(self) { self.pool.close().await; }
}
