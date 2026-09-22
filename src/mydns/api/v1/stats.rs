use std::sync::Arc;

use axum::{
    extract::{Query, State},
    Json,
};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::error::ApiError;
use crate::state::AppState;

const DEFAULT_HISTORY_RANGE: Duration = Duration::hours(1);
const MAX_HISTORY_RANGE: Duration = Duration::hours(24);

#[derive(Debug, Deserialize)]
pub struct HistoryQuery {
    /// Inclusive UTC start timestamp in RFC3339 format.
    pub from: Option<String>,
    /// Inclusive UTC end timestamp in RFC3339 format.
    pub to: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct HistoryResponse {
    server_time: DateTime<Utc>,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
    resolution_seconds: u32,
    oldest_available: Option<DateTime<Utc>>,
    latest_available: Option<DateTime<Utc>>,
    samples: Vec<crate::observability::HistorySample>,
}

/// `GET /api/v1/stats`
pub async fn get_stats(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let (hits, misses) = state.cache_stats.snapshot();
    let cache_size = state.cache.read().await.len();
    let record_count: i64 = crate::db::records::count_records(&state.db)
        .await
        .unwrap_or(0);
    let blocklist_size: i64 = crate::db::blocklist::count_entries(&state.db)
        .await
        .unwrap_or(0);

    let total_cache = hits + misses;
    let cache_hit_rate = if total_cache == 0 {
        0.0
    } else {
        hits as f64 / total_cache as f64 * 100.0
    };

    let stats = state.metrics.snapshot();
    let mut value = serde_json::to_value(stats)
        .map_err(|error| ApiError::Internal(anyhow::Error::new(error)))?;

    if let Some(object) = value.as_object_mut() {
        object.insert("cache_hits".into(), json!(hits));
        object.insert("cache_misses".into(), json!(misses));
        object.insert("cache_hit_rate".into(), json!(cache_hit_rate));
        object.insert("cache_size".into(), json!(cache_size));
        object.insert("record_count".into(), json!(record_count));
        object.insert("blocklist_size".into(), json!(blocklist_size));
    }

    Ok(Json(value))
}

/// `GET /api/v1/stats/history?from=<rfc3339>&to=<rfc3339>`
pub async fn get_stats_history(
    State(state): State<Arc<AppState>>,
    Query(query): Query<HistoryQuery>,
) -> Result<Json<HistoryResponse>, ApiError> {
    let server_time = Utc::now();
    let mut to = parse_timestamp(query.to.as_deref(), "to")?.unwrap_or(server_time);
    if to > server_time {
        to = server_time;
    }

    let requested_from = parse_timestamp(query.from.as_deref(), "from")?;
    let mut from = requested_from.unwrap_or(to - DEFAULT_HISTORY_RANGE);

    if from > to {
        return Err(ApiError::BadRequest(
            "history 'from' must not be after 'to'".to_string(),
        ));
    }

    if to - from > MAX_HISTORY_RANGE {
        from = to - MAX_HISTORY_RANGE;
    }

    let history = state.metrics.history(from, to);

    Ok(Json(HistoryResponse {
        server_time,
        from,
        to,
        resolution_seconds: history.resolution_seconds,
        oldest_available: history.oldest_available,
        latest_available: history.latest_available,
        samples: history.samples,
    }))
}

fn parse_timestamp(value: Option<&str>, field: &str) -> Result<Option<DateTime<Utc>>, ApiError> {
    value
        .map(|value| {
            DateTime::parse_from_rfc3339(value)
                .map(|timestamp| timestamp.with_timezone(&Utc))
                .map_err(|error| {
                    ApiError::BadRequest(format!(
                        "invalid '{}' timestamp '{}': {}",
                        field, value, error
                    ))
                })
        })
        .transpose()
}

#[cfg(test)]
mod tests {
    use super::parse_timestamp;
    use crate::error::ApiError;
    use chrono::{Datelike, Timelike};

    #[test]
    fn parses_rfc3339_and_normalizes_to_utc() {
        let parsed = parse_timestamp(Some("2026-09-21T12:30:45+05:30"), "from")
            .unwrap()
            .unwrap();
        assert_eq!(parsed.year(), 2026);
        assert_eq!(parsed.month(), 9);
        assert_eq!(parsed.day(), 21);
        assert_eq!(parsed.hour(), 7);
        assert_eq!(parsed.minute(), 0);
        assert_eq!(parsed.second(), 45);
    }

    #[test]
    fn absent_timestamp_is_none() {
        assert!(parse_timestamp(None, "to").unwrap().is_none());
    }

    #[test]
    fn malformed_timestamp_returns_bad_request_with_field_context() {
        let error = parse_timestamp(Some("not-a-timestamp"), "to").unwrap_err();
        match error {
            ApiError::BadRequest(message) => {
                assert!(message.contains("invalid 'to' timestamp"));
                assert!(message.contains("not-a-timestamp"));
            }
            other => panic!("expected BadRequest, got {other:?}"),
        }
    }
}
