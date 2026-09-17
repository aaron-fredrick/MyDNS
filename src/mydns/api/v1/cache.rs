use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use hickory_proto::rr::RecordType;
use serde::Serialize;
use std::sync::Arc;

use crate::error::ApiError;
use crate::state::AppState;
use crate::web::auth::JwtClaims;

#[derive(Serialize)]
pub struct CacheEntryInfo {
    pub name: String,
    pub record_type: String,
    pub ttl_remaining: u32,
    pub values: Vec<String>,
}

/// `GET /api/v1/cache`
pub async fn list_cache(
    State(state): State<Arc<AppState>>,
    _claims: JwtClaims,
) -> Result<Json<Vec<CacheEntryInfo>>, ApiError> {
    use std::collections::HashMap;

    let mut map: HashMap<(String, String), CacheEntryInfo> = HashMap::new();

    // 1. Get memory entries
    {
        let cache = state.cache.read().await;
        for (name, rtype, ttl, values) in cache.list_all() {
            map.insert(
                (name.clone(), rtype.to_string()),
                CacheEntryInfo {
                    name,
                    record_type: rtype.to_string(),
                    ttl_remaining: ttl,
                    values,
                },
            );
        }
    }

    // 2. Get DB entries
    if let Ok(db_entries) = crate::db::records::list_cache_entries(&state.db).await {
        let now = chrono::Utc::now().timestamp();
        for row in db_entries {
            let key = (row.name.clone(), row.record_type.clone());
            let ttl = (row.expires_at - now).max(0) as u32;

            map.entry(key)
                .and_modify(|e| {
                    if !e.values.contains(&row.value) {
                        e.values.push(row.value.clone());
                    }
                    // Keep the lowest TTL
                    e.ttl_remaining = e.ttl_remaining.min(ttl);
                })
                .or_insert(CacheEntryInfo {
                    name: row.name,
                    record_type: row.record_type,
                    ttl_remaining: ttl,
                    values: vec![row.value],
                });
        }
    }

    let mut list: Vec<_> = map.into_values().collect();
    list.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(Json(list))
}

/// `DELETE /api/v1/cache`
pub async fn clear_cache(
    State(state): State<Arc<AppState>>,
    _claims: JwtClaims,
) -> Result<StatusCode, ApiError> {
    // Clear Memory
    state.cache.write().await.clear();

    // Clear DB
    let _ = crate::db::records::clear_cache(&state.db).await;

    let _ = state.log_tx.send("[CRUD] Cache cleared".to_string());
    tracing::info!("DNS cache cleared by admin");

    Ok(StatusCode::NO_CONTENT)
}

/// `DELETE /api/v1/cache/:name/:type`
pub async fn delete_cache_entry(
    State(state): State<Arc<AppState>>,
    _claims: JwtClaims,
    Path((name, rtype_str)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    let rtype = rtype_str
        .parse::<RecordType>()
        .map_err(|_| ApiError::BadRequest(format!("Invalid record type: {}", rtype_str)))?;

    // Delete from Memory
    state.cache.write().await.remove(&name, rtype);

    // Delete from DB
    let _ = crate::db::records::delete_cache_entry(&state.db, &name, &rtype_str).await;

    let _ = state
        .log_tx
        .send(format!("[CRUD] Cache entry deleted: {} {}", name, rtype));
    tracing::info!(name = %name, r#type = %rtype, "Cache entry deleted by admin");

    Ok(StatusCode::NO_CONTENT)
}
