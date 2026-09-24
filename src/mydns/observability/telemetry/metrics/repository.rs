use anyhow::Context;
use crate::observability::database::ObservabilityDatabase;
use super::types::{HistoryBucket, OperationalPeriodSnapshot};

pub async fn persist(
    database: &ObservabilityDatabase,
    periods: &[OperationalPeriodSnapshot],
    buckets: &[HistoryBucket],
) -> anyhow::Result<()> {
    if periods.is_empty() && buckets.is_empty() { return Ok(()); }
    let mut transaction = database.pool().begin().await?;

    for snapshot in periods {
        sqlx::query(r#"
            INSERT INTO operational_periods (
                start_utc, end_utc, timezone, queries, responses, blocked, blocked_reasons,
                upstream_requests, upstream_successes, upstream_failures, upstream_timeouts,
                upstream_retries, cache_hits, cache_misses, cache_evictions, record_types,
                transports, response_codes, resolution_outcomes, resolution_paths,
                response_latency, upstream_latency
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(start_utc, end_utc) DO NOTHING
        "#)
        .bind(snapshot.start_utc.to_rfc3339()).bind(snapshot.end_utc.to_rfc3339())
        .bind(&snapshot.timezone)
        .bind(snapshot.queries as i64).bind(snapshot.responses as i64).bind(snapshot.blocked as i64)
        .bind(serde_json::to_string(&snapshot.blocked_reason_counts).context("serialize blocked reasons")?)
        .bind(snapshot.upstream_requests as i64).bind(snapshot.upstream_successes as i64)
        .bind(snapshot.upstream_failures as i64).bind(snapshot.upstream_timeouts as i64)
        .bind(snapshot.upstream_retries as i64).bind(snapshot.cache_hits as i64).bind(snapshot.cache_misses as i64)
        .bind(snapshot.cache_evictions as i64)
        .bind(serde_json::to_string(&snapshot.record_type_counts).context("serialize record types")?)
        .bind(serde_json::to_string(&snapshot.transport_counts).context("serialize transports")?)
        .bind(serde_json::to_string(&snapshot.response_code_counts).context("serialize response codes")?)
        .bind(serde_json::to_string(&snapshot.resolution_outcome_counts).context("serialize resolution outcomes")?)
        .bind(serde_json::to_string(&snapshot.resolution_path_counts).context("serialize resolution paths")?)
        .bind(serde_json::to_string(&snapshot.response_latency).context("serialize response latency")?)
        .bind(serde_json::to_string(&snapshot.upstream_latency).context("serialize upstream latency")?)
        .execute(&mut *transaction).await?;
    }

    for bucket in buckets {
        sqlx::query(r#"
            INSERT INTO historical_buckets (
                timestamp, resolution_seconds, request_count, response_count, blocked_count, blocked_reasons,
                cache_hits, cache_misses, cache_evictions, upstream_requests, upstream_successes,
                upstream_failures, upstream_timeouts, upstream_retries, record_types, transports,
                response_codes, resolution_outcomes, resolution_paths, response_latency, upstream_latency
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(timestamp, resolution_seconds) DO NOTHING
        "#)
        .bind(bucket.timestamp.to_rfc3339()).bind(bucket.resolution_seconds as i64)
        .bind(bucket.request_count as i64).bind(bucket.response_count as i64).bind(bucket.blocked_count as i64)
        .bind(serde_json::to_string(&bucket.blocked_reason_counts).context("serialize blocked reasons")?)
        .bind(bucket.cache_hits as i64).bind(bucket.cache_misses as i64).bind(bucket.cache_evictions as i64)
        .bind(bucket.upstream_requests as i64).bind(bucket.upstream_successes as i64)
        .bind(bucket.upstream_failures as i64).bind(bucket.upstream_timeouts as i64).bind(bucket.upstream_retries as i64)
        .bind(serde_json::to_string(&bucket.record_type_counts).context("serialize record types")?)
        .bind(serde_json::to_string(&bucket.transport_counts).context("serialize transports")?)
        .bind(serde_json::to_string(&bucket.response_code_counts).context("serialize response codes")?)
        .bind(serde_json::to_string(&bucket.resolution_outcome_counts).context("serialize resolution outcomes")?)
        .bind(serde_json::to_string(&bucket.resolution_path_counts).context("serialize resolution paths")?)
        .bind(serde_json::to_string(&bucket.response_latency).context("serialize response latency")?)
        .bind(serde_json::to_string(&bucket.upstream_latency).context("serialize upstream latency")?)
        .execute(&mut *transaction).await?;
    }

    transaction.commit().await?;
    Ok(())
}
