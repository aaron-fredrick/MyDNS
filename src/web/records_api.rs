use std::sync::Arc;

use axum::{
    extract::{Path, State},
    Json,
};

use crate::db::records::{self, CreateRecord, UpdateRecord};
use crate::state::AppState;
use crate::web::auth::JwtClaims;
use crate::web::error::ApiError;

/// `GET /api/v1/records`
#[allow(non_snake_case)]
pub async fn listRecords(
    _claims: JwtClaims,
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let rows = records::listRecords(&state.db).await?;
    Ok(Json(serde_json::json!({ "records": rows })))
}

/// `POST /api/v1/records`
#[allow(non_snake_case)]
pub async fn createRecord(
    _claims: JwtClaims,
    State(state): State<Arc<AppState>>,
    Json(mut body): Json<CreateRecord>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Normalize name before storing
    body.name = body.name.trim_end_matches('.').to_lowercase();

    let row = records::createRecord(&state.db, &body).await?;

    // Invalidate any stale cache entry for this name so the new record is
    // picked up immediately on the next DNS query.
    if let Ok(rtype) = body.record_type.parse::<hickory_proto::rr::RecordType>() {
        state.cache.write().await.remove(&body.name, rtype);
    }

    tracing::info!(name = %row.name, r#type = %row.record_type, "DNS record created");
    let _ = state.log_tx.send(format!(
        "[CRUD] CREATE record id={} name={}",
        row.id, row.name
    ));

    Ok(Json(serde_json::json!({ "record": row })))
}

/// `PUT /api/v1/records/:id`
#[allow(non_snake_case)]
pub async fn updateRecord(
    _claims: JwtClaims,
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    Json(body): Json<UpdateRecord>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Fetch old record to know which cache key to invalidate.
    if let Ok(Some(old)) = records::getRecord(&state.db, id).await {
        if let Ok(rtype) = old.record_type.parse::<hickory_proto::rr::RecordType>() {
            state.cache.write().await.remove(&old.name, rtype);
        }
    }

    let mut body = body;
    if let Some(ref mut name) = body.name {
        *name = name.trim_end_matches('.').to_lowercase();
    }

    let updated = records::updateRecord(&state.db, id, &body)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Record {} not found", id)))?;

    tracing::info!(id, name = %updated.name, "DNS record updated");
    let _ = state.log_tx.send(format!("[CRUD] UPDATE record id={}", id));

    Ok(Json(serde_json::json!({ "record": updated })))
}

/// `DELETE /api/v1/records/:id`
#[allow(non_snake_case)]
pub async fn deleteRecord(
    _claims: JwtClaims,
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Remove from cache before deleting from DB.
    if let Ok(Some(row)) = records::getRecord(&state.db, id).await {
        if let Ok(rtype) = row.record_type.parse::<hickory_proto::rr::RecordType>() {
            state.cache.write().await.remove(&row.name, rtype);
        }
    }

    let deleted = records::deleteRecord(&state.db, id).await?;
    if !deleted {
        return Err(ApiError::NotFound(format!("Record {} not found", id)));
    }

    tracing::info!(id, "DNS record deleted");
    let _ = state.log_tx.send(format!("[CRUD] DELETE record id={}", id));

    Ok(Json(serde_json::json!({ "deleted": id })))
}
