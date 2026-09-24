use anyhow::Context;
use chrono::{DateTime, Duration, Utc};
use sqlx::{Row, SqliteConnection};

use super::types::{HistoryBucket, OperationalPeriodSnapshot};
use crate::observability::database::ObservabilityDatabase;

pub async fn persist(
    database: &ObservabilityDatabase,
    periods: &[OperationalPeriodSnapshot],
    buckets: &[HistoryBucket],
) -> anyhow::Result<()> {
    if periods.is_empty() && buckets.is_empty() {
        return Ok(());
    }

    let mut transaction = database.pool().begin().await?;

    for snapshot in periods {
        insert_operational_period(&mut *transaction, snapshot).await?;
    }

    for bucket in buckets {
        insert_history_bucket(&mut *transaction, bucket).await?;
    }

    transaction.commit().await?;
    Ok(())
}

async fn insert_operational_period(
    connection: &mut SqliteConnection,
    snapshot: &OperationalPeriodSnapshot,
) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        INSERT INTO operational_periods (
            start_utc, end_utc, timezone, queries, responses, blocked, blocked_reasons,
            upstream_requests, upstream_successes, upstream_failures, upstream_timeouts,
            upstream_retries, cache_hits, cache_misses, cache_evictions, record_types,
            transports, response_codes, resolution_outcomes, resolution_paths,
            response_latency, upstream_latency
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(start_utc, end_utc) DO NOTHING
        "#,
    )
    .bind(snapshot.start_utc.to_rfc3339())
    .bind(snapshot.end_utc.to_rfc3339())
    .bind(&snapshot.timezone)
    .bind(snapshot.queries as i64)
    .bind(snapshot.responses as i64)
    .bind(snapshot.blocked as i64)
    .bind(serialize(
        &snapshot.blocked_reason_counts,
        "blocked reasons",
    )?)
    .bind(snapshot.upstream_requests as i64)
    .bind(snapshot.upstream_successes as i64)
    .bind(snapshot.upstream_failures as i64)
    .bind(snapshot.upstream_timeouts as i64)
    .bind(snapshot.upstream_retries as i64)
    .bind(snapshot.cache_hits as i64)
    .bind(snapshot.cache_misses as i64)
    .bind(snapshot.cache_evictions as i64)
    .bind(serialize(&snapshot.record_type_counts, "record types")?)
    .bind(serialize(&snapshot.transport_counts, "transports")?)
    .bind(serialize(&snapshot.response_code_counts, "response codes")?)
    .bind(serialize(
        &snapshot.resolution_outcome_counts,
        "resolution outcomes",
    )?)
    .bind(serialize(
        &snapshot.resolution_path_counts,
        "resolution paths",
    )?)
    .bind(serialize(&snapshot.response_latency, "response latency")?)
    .bind(serialize(&snapshot.upstream_latency, "upstream latency")?)
    .execute(connection)
    .await?;

    Ok(())
}

async fn insert_history_bucket(
    connection: &mut SqliteConnection,
    bucket: &HistoryBucket,
) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        INSERT INTO historical_buckets (
            timestamp, resolution_seconds, request_count, response_count, blocked_count,
            blocked_reasons, cache_hits, cache_misses, cache_evictions,
            upstream_requests, upstream_successes, upstream_failures, upstream_timeouts,
            upstream_retries, record_types, transports, response_codes,
            resolution_outcomes, resolution_paths, response_latency, upstream_latency
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(timestamp, resolution_seconds) DO NOTHING
        "#,
    )
    .bind(bucket.timestamp.to_rfc3339())
    .bind(bucket.resolution_seconds as i64)
    .bind(bucket.request_count as i64)
    .bind(bucket.response_count as i64)
    .bind(bucket.blocked_count as i64)
    .bind(serialize(&bucket.blocked_reason_counts, "blocked reasons")?)
    .bind(bucket.cache_hits as i64)
    .bind(bucket.cache_misses as i64)
    .bind(bucket.cache_evictions as i64)
    .bind(bucket.upstream_requests as i64)
    .bind(bucket.upstream_successes as i64)
    .bind(bucket.upstream_failures as i64)
    .bind(bucket.upstream_timeouts as i64)
    .bind(bucket.upstream_retries as i64)
    .bind(serialize(&bucket.record_type_counts, "record types")?)
    .bind(serialize(&bucket.transport_counts, "transports")?)
    .bind(serialize(&bucket.response_code_counts, "response codes")?)
    .bind(serialize(
        &bucket.resolution_outcome_counts,
        "resolution outcomes",
    )?)
    .bind(serialize(
        &bucket.resolution_path_counts,
        "resolution paths",
    )?)
    .bind(serialize(&bucket.response_latency, "response latency")?)
    .bind(serialize(&bucket.upstream_latency, "upstream latency")?)
    .execute(connection)
    .await?;

    Ok(())
}

fn serialize<T: serde::Serialize>(value: &T, label: &str) -> anyhow::Result<String> {
    serde_json::to_string(value).with_context(|| format!("serialize {label}"))
}

const ROLLUPS: &[(u32, u32, i64, i64)] = &[
    (60, 3600, 24 * 60 * 60, 3600),
    (3600, 21600, 48 * 60 * 60, 21600),
    (21600, 43200, 7 * 24 * 60 * 60, 43200),
    (43200, 86400, 30 * 24 * 60 * 60, 86400),
];

pub async fn roll_up(database: &ObservabilityDatabase, now: DateTime<Utc>) -> anyhow::Result<()> {
    let mut transaction = database.pool().begin().await?;

    for &(source_resolution, target_resolution, minimum_age_seconds, alignment_seconds) in ROLLUPS {
        roll_up_resolution(
            &mut transaction,
            now,
            source_resolution,
            target_resolution,
            minimum_age_seconds,
            alignment_seconds,
        )
        .await?;
    }

    transaction.commit().await?;
    Ok(())
}

async fn roll_up_resolution(
    connection: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    now: DateTime<Utc>,
    source_resolution: u32,
    target_resolution: u32,
    minimum_age_seconds: i64,
    alignment_seconds: i64,
) -> anyhow::Result<()> {
    let cutoff = now - Duration::seconds(minimum_age_seconds);

    let rows = sqlx::query(
        r#"
        SELECT timestamp, resolution_seconds, request_count, response_count, blocked_count,
               blocked_reasons, cache_hits, cache_misses, cache_evictions,
               upstream_requests, upstream_successes, upstream_failures, upstream_timeouts,
               upstream_retries, record_types, transports, response_codes,
               resolution_outcomes, resolution_paths, response_latency, upstream_latency
        FROM historical_buckets
        WHERE resolution_seconds = ?
          AND timestamp < ?
        ORDER BY timestamp ASC
        "#,
    )
    .bind(source_resolution as i64)
    .bind(cutoff.to_rfc3339())
    .fetch_all(&mut **connection)
    .await?;

    if rows.is_empty() {
        return Ok(());
    }

    let mut current_group: Option<HistoryBucket> = None;
    let mut current_source_rows = Vec::new();

    for row in rows {
        let bucket = decode_bucket(&row)?;
        let group_start = align_timestamp(bucket.timestamp, alignment_seconds);
        let group_end = group_start + Duration::seconds(target_resolution as i64);

        if group_end > cutoff {
            continue;
        }

        match current_group.as_mut() {
            Some(group) if group.timestamp == group_start => {
                group.merge(&bucket);
                current_source_rows.push(bucket.timestamp);
            }
            Some(_) => {
                persist_rollup_group(
                    connection,
                    current_group.take().unwrap(),
                    &current_source_rows,
                    source_resolution,
                    target_resolution,
                )
                .await?;

                current_source_rows.clear();
                let mut group = HistoryBucket::with_resolution(group_start, target_resolution);
                group.merge(&bucket);
                current_group = Some(group);
                current_source_rows.push(bucket.timestamp);
            }
            None => {
                let mut group = HistoryBucket::with_resolution(group_start, target_resolution);
                group.merge(&bucket);
                current_group = Some(group);
                current_source_rows.push(bucket.timestamp);
            }
        }
    }

    if let Some(group) = current_group {
        persist_rollup_group(
            connection,
            group,
            &current_source_rows,
            source_resolution,
            target_resolution,
        )
        .await?;
    }

    Ok(())
}

async fn persist_rollup_group(
    connection: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    group: HistoryBucket,
    source_timestamps: &[DateTime<Utc>],
    source_resolution: u32,
    target_resolution: u32,
) -> anyhow::Result<()> {
    let expected_rows = target_resolution / source_resolution;
    if source_timestamps.len() as u32 != expected_rows {
        return Ok(());
    }

    insert_history_bucket(&mut **connection, &group).await?;

    for timestamp in source_timestamps {
        sqlx::query(
            "DELETE FROM historical_buckets WHERE timestamp = ? AND resolution_seconds = ?",
        )
        .bind(timestamp.to_rfc3339())
        .bind(source_resolution as i64)
        .execute(&mut **connection)
        .await?;
    }

    Ok(())
}

fn align_timestamp(timestamp: DateTime<Utc>, resolution_seconds: i64) -> DateTime<Utc> {
    let seconds = timestamp.timestamp();
    DateTime::<Utc>::from_timestamp(seconds - seconds.rem_euclid(resolution_seconds), 0)
        .expect("valid aligned timestamp")
}

fn decode_bucket(row: &sqlx::sqlite::SqliteRow) -> anyhow::Result<HistoryBucket> {
    Ok(HistoryBucket {
        timestamp: row
            .try_get::<String, _>("timestamp")?
            .parse()
            .context("parse bucket timestamp")?,
        resolution_seconds: row.try_get::<i64, _>("resolution_seconds")? as u32,
        request_count: row.try_get::<i64, _>("request_count")? as u64,
        response_count: row.try_get::<i64, _>("response_count")? as u64,
        blocked_count: row.try_get::<i64, _>("blocked_count")? as u64,
        blocked_reason_counts: deserialize(row.try_get("blocked_reasons")?, "blocked reasons")?,
        record_type_counts: deserialize(row.try_get("record_types")?, "record types")?,
        transport_counts: deserialize(row.try_get("transports")?, "transports")?,
        response_code_counts: deserialize(row.try_get("response_codes")?, "response codes")?,
        resolution_outcome_counts: deserialize(
            row.try_get("resolution_outcomes")?,
            "resolution outcomes",
        )?,
        resolution_path_counts: deserialize(row.try_get("resolution_paths")?, "resolution paths")?,
        cache_hits: row.try_get::<i64, _>("cache_hits")? as u64,
        cache_misses: row.try_get::<i64, _>("cache_misses")? as u64,
        cache_evictions: row.try_get::<i64, _>("cache_evictions")? as u64,
        upstream_requests: row.try_get::<i64, _>("upstream_requests")? as u64,
        upstream_successes: row.try_get::<i64, _>("upstream_successes")? as u64,
        upstream_failures: row.try_get::<i64, _>("upstream_failures")? as u64,
        upstream_timeouts: row.try_get::<i64, _>("upstream_timeouts")? as u64,
        upstream_retries: row.try_get::<i64, _>("upstream_retries")? as u64,
        response_latency: deserialize(row.try_get("response_latency")?, "response latency")?,
        upstream_latency: deserialize(row.try_get("upstream_latency")?, "upstream latency")?,
    })
}

fn deserialize<T: serde::de::DeserializeOwned>(value: String, label: &str) -> anyhow::Result<T> {
    serde_json::from_str(&value).with_context(|| format!("deserialize {label}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::observability::database::ObservabilityDatabase;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn persistence_round_trip_and_rollup() {
        let file = NamedTempFile::new().unwrap();
        let db = ObservabilityDatabase::init(file.path().to_str().unwrap())
            .await
            .unwrap();

        let start = DateTime::parse_from_rfc3339("2026-09-22T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);

        let mut buckets = Vec::new();
        for minute in 0..60 {
            let timestamp = start + Duration::minutes(minute);
            let mut minute_bucket = HistoryBucket::new(timestamp);
            minute_bucket.request_count = 1;
            buckets.push(minute_bucket);
        }

        persist(&db, &[], &buckets).await.unwrap();
        roll_up(&db, start + Duration::hours(25)).await.unwrap();

        let hourly_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM historical_buckets WHERE resolution_seconds = 3600",
        )
        .fetch_one(db.pool())
        .await
        .unwrap();

        let minute_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM historical_buckets WHERE resolution_seconds = 60",
        )
        .fetch_one(db.pool())
        .await
        .unwrap();

        assert_eq!(hourly_count, 1);
        assert_eq!(minute_count, 0);
    }
}
