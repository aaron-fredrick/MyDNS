use sqlx::SqlitePool;

pub(super) async fn run_migrations(pool: &SqlitePool) -> anyhow::Result<()> {
    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS operational_periods (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            start_utc TEXT NOT NULL, end_utc TEXT NOT NULL, timezone TEXT NOT NULL,
            queries INTEGER NOT NULL DEFAULT 0, responses INTEGER NOT NULL DEFAULT 0,
            blocked INTEGER NOT NULL DEFAULT 0,
            blocked_reasons TEXT NOT NULL,
            upstream_requests INTEGER NOT NULL DEFAULT 0, upstream_successes INTEGER NOT NULL DEFAULT 0,
            upstream_failures INTEGER NOT NULL DEFAULT 0, upstream_timeouts INTEGER NOT NULL DEFAULT 0,
            upstream_retries INTEGER NOT NULL DEFAULT 0,
            cache_hits INTEGER NOT NULL DEFAULT 0, cache_misses INTEGER NOT NULL DEFAULT 0,
            cache_evictions INTEGER NOT NULL DEFAULT 0,
            record_types TEXT NOT NULL, transports TEXT NOT NULL, response_codes TEXT NOT NULL,
            resolution_outcomes TEXT NOT NULL, resolution_paths TEXT NOT NULL,
            response_latency TEXT NOT NULL, upstream_latency TEXT NOT NULL,
            UNIQUE(start_utc, end_utc)
        )
    "#).execute(pool).await?;

    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS historical_buckets (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp TEXT NOT NULL, resolution_seconds INTEGER NOT NULL,
            request_count INTEGER NOT NULL DEFAULT 0, response_count INTEGER NOT NULL DEFAULT 0,
            blocked_count INTEGER NOT NULL DEFAULT 0,
            blocked_reasons TEXT NOT NULL,
            cache_hits INTEGER NOT NULL DEFAULT 0, cache_misses INTEGER NOT NULL DEFAULT 0,
            cache_evictions INTEGER NOT NULL DEFAULT 0,
            upstream_requests INTEGER NOT NULL DEFAULT 0, upstream_successes INTEGER NOT NULL DEFAULT 0,
            upstream_failures INTEGER NOT NULL DEFAULT 0, upstream_timeouts INTEGER NOT NULL DEFAULT 0,
            upstream_retries INTEGER NOT NULL DEFAULT 0,
            record_types TEXT NOT NULL, transports TEXT NOT NULL, response_codes TEXT NOT NULL,
            resolution_outcomes TEXT NOT NULL, resolution_paths TEXT NOT NULL,
            response_latency TEXT NOT NULL, upstream_latency TEXT NOT NULL,
            UNIQUE(timestamp, resolution_seconds)
        )
    "#).execute(pool).await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_historical_buckets_timestamp ON historical_buckets(timestamp)")
        .execute(pool).await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_historical_buckets_resolution_time ON historical_buckets(resolution_seconds, timestamp)")
        .execute(pool).await?;
    Ok(())
}
