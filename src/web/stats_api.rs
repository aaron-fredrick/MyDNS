use std::sync::Arc;

use axum::{extract::State, Json};

use crate::state::AppState;
use crate::web::error::ApiError;

/// `GET /api/v1/stats`
///
/// Returns low-cost in-process resolver observability data. The frontend is
/// responsible only for presentation; authoritative operational metrics are
/// collected by the Rust DNS path and exposed here as an API contract.
#[allow(non_snake_case)]
pub async fn getStats(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let (hits, misses) = state.cache_stats.snapshot();
    let cache_size = state.cache.read().await.len();
    let record_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM dns_records")
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);

    let total_cache = hits + misses;
    let cache_hit_rate = if total_cache == 0 {
        0.0
    } else {
        hits as f64 / total_cache as f64 * 100.0
    };

    let mut stats = state.metrics.snapshot();
    if let Some(object) = stats.as_object_mut() {
        object.insert("cache_hits".into(), serde_json::json!(hits));
        object.insert("cache_misses".into(), serde_json::json!(misses));
        object.insert("cache_hit_rate".into(), serde_json::json!(cache_hit_rate));
        object.insert("cache_size".into(), serde_json::json!(cache_size));
        object.insert("record_count".into(), serde_json::json!(record_count));
    }

    Ok(Json(stats))
}
