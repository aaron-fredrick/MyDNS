use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::SqlitePool;
use std::str::FromStr;

pub async fn init(db_path: &str) -> anyhow::Result<SqlitePool> {
    let options = SqliteConnectOptions::from_str(&format!("sqlite:{}", db_path))?
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to connect to observability database {}: {}", db_path, e))?;

    run_migrations(&pool).await?;
    Ok(pool)
}

async fn run_migrations(pool: &SqlitePool) -> anyhow::Result<()> {
    // 24-hour operational snapshots
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS operational_periods (
            id                 INTEGER PRIMARY KEY AUTOINCREMENT,
            start_utc          TEXT NOT NULL,
            end_utc            TEXT NOT NULL,
            timezone           TEXT NOT NULL,
            queries            INTEGER NOT NULL DEFAULT 0,
            responses          INTEGER NOT NULL DEFAULT 0,
            blocked            INTEGER NOT NULL DEFAULT 0,
            upstream_requests  INTEGER NOT NULL DEFAULT 0,
            upstream_successes INTEGER NOT NULL DEFAULT 0,
            upstream_failures  INTEGER NOT NULL DEFAULT 0,
            upstream_timeouts  INTEGER NOT NULL DEFAULT 0,
            upstream_retries   INTEGER NOT NULL DEFAULT 0,
            cache_hits         INTEGER NOT NULL DEFAULT 0,
            cache_misses       INTEGER NOT NULL DEFAULT 0,
            cache_evictions    INTEGER NOT NULL DEFAULT 0,
            record_types       TEXT NOT NULL,
            transports         TEXT NOT NULL,
            response_codes     TEXT NOT NULL,
            outcomes           TEXT NOT NULL,
            resolution_paths   TEXT NOT NULL,
            response_latency   TEXT NOT NULL,
            upstream_latency   TEXT NOT NULL
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query("CREATE UNIQUE INDEX IF NOT EXISTS idx_operational_periods_dates ON operational_periods(start_utc, end_utc)")
        .execute(pool)
        .await?;

    // Historical buckets
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS historical_buckets (
            id                   INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp            TEXT NOT NULL,
            resolution           INTEGER NOT NULL,
            request_count        INTEGER NOT NULL DEFAULT 0,
            error_count          INTEGER NOT NULL DEFAULT 0,
            upstream_requests    INTEGER NOT NULL DEFAULT 0,
            upstream_failures    INTEGER NOT NULL DEFAULT 0,
            response_latency     TEXT NOT NULL,
            upstream_latency     TEXT NOT NULL
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query("CREATE UNIQUE INDEX IF NOT EXISTS idx_historical_buckets_time_res ON historical_buckets(timestamp, resolution)")
        .execute(pool)
        .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_historical_buckets_timestamp ON historical_buckets(timestamp)")
        .execute(pool)
        .await?;

    Ok(())
}
