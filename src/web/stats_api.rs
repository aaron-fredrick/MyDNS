use std::sync::Arc;

use axum::{extract::State, Json};

use crate::state::AppState;
use crate::web::error::ApiError;

/// `GET /api/v1/stats`
///
/// Returns server uptime, cache hit/miss counts, cache size, and record count.
pub async fn get_stats(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let uptime_secs = state.start_time.elapsed().as_secs();
    let (hits, misses) = state.cache_stats.snapshot();
    let cache_size = state.cache.read().await.len();
    let record_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM dns_records")
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);

    Ok(Json(serde_json::json!({
        "uptime_secs":    uptime_secs,
        "cache_hits":     hits,
        "cache_misses":   misses,
        "cache_size":     cache_size,
        "record_count":   record_count,
    })))
}
