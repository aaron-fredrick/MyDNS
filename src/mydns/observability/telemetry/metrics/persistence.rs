use std::sync::Arc;
use std::time::Duration;
use sqlx::SqlitePool;
use tokio_util::sync::CancellationToken;

use super::aggregator::MetricsAggregator;

pub fn spawn_persistence(
    metrics: Arc<MetricsAggregator>,
    pool: SqlitePool,
    timezone: String,
    cancel: CancellationToken,
) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(60));
        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if let Err(e) = persist_metrics(&metrics, &pool, &timezone).await {
                        tracing::error!(error = %e, "Failed to persist telemetry metrics");
                    }
                }
                _ = cancel.cancelled() => {
                    if let Err(e) = persist_metrics(&metrics, &pool, &timezone).await {
                        tracing::error!(error = %e, "Failed to persist telemetry metrics during shutdown");
                    }
                    break;
                }
            }
        }
    });
}

async fn persist_metrics(metrics: &MetricsAggregator, pool: &SqlitePool, timezone: &str) -> anyhow::Result<()> {
    // 1. Finalize operational period if needed
    if let Some(snapshot) = metrics.try_finalize_period(timezone) {
        let record_types = serde_json::to_string(&snapshot.record_type_counts).unwrap_or_default();
        let transports = serde_json::to_string(&snapshot.transport_counts).unwrap_or_default();
        let response_codes = serde_json::to_string(&snapshot.response_code_counts).unwrap_or_default();
        let outcomes = serde_json::to_string(&snapshot.resolution_outcome_counts).unwrap_or_default();
        let paths = serde_json::to_string(&snapshot.resolution_path_counts).unwrap_or_default();
        
        let response_latency = serde_json::to_string(&snapshot.response_latency).unwrap_or_default();
        let upstream_latency = serde_json::to_string(&snapshot.upstream_latency).unwrap_or_default();

        sqlx::query(
            r#"
            INSERT INTO operational_periods (
                start_utc, end_utc, timezone, queries, responses, blocked,
                upstream_requests, upstream_successes, upstream_failures, upstream_timeouts, upstream_retries,
                cache_hits, cache_misses, cache_evictions,
                record_types, transports, response_codes, outcomes, resolution_paths,
                response_latency, upstream_latency
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(snapshot.start_utc.to_rfc3339())
        .bind(snapshot.end_utc.to_rfc3339())
        .bind(timezone)
        .bind(snapshot.queries as i64)
        .bind(snapshot.responses as i64)
        .bind(snapshot.blocked as i64)
        .bind(snapshot.upstream_requests as i64)
        .bind(snapshot.upstream_successes as i64)
        .bind(snapshot.upstream_failures as i64)
        .bind(snapshot.upstream_timeouts as i64)
        .bind(snapshot.upstream_retries as i64)
        .bind(snapshot.cache_hits as i64)
        .bind(snapshot.cache_misses as i64)
        .bind(snapshot.cache_evictions as i64)
        .bind(record_types)
        .bind(transports)
        .bind(response_codes)
        .bind(outcomes)
        .bind(paths)
        .bind(response_latency)
        .bind(upstream_latency)
        .execute(pool)
        .await?;
    }

    // 2. Persist finalized 1-minute buckets
    let buckets = metrics.take_finalized_buckets();
    for bucket in buckets {
        let response_latency = serde_json::to_string(&bucket.response_latency).unwrap_or_default();
        let upstream_latency = serde_json::to_string(&bucket.upstream_latency).unwrap_or_default();

        sqlx::query(
            r#"
            INSERT INTO historical_buckets (
                timestamp, resolution, request_count, error_count,
                upstream_requests, upstream_failures,
                response_latency, upstream_latency
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT DO NOTHING
            "#,
        )
        .bind(bucket.timestamp.to_rfc3339())
        .bind(bucket.resolution_seconds)
        .bind(bucket.request_count as i64)
        .bind(bucket.error_count as i64)
        .bind(bucket.upstream_requests as i64)
        .bind(bucket.upstream_failures as i64)
        .bind(response_latency)
        .bind(upstream_latency)
        .execute(pool)
        .await?;
    }

    Ok(())
}
