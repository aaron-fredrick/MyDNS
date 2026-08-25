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

    // A newly-created authoritative record must supersede any cached answer
    // for this name, including answers that depended on a CNAME chain.
    records::deleteCacheForName(&state.db, &body.name).await?;
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
    // Fetch old record to invalidate both its old and new cache keys if the
    // mutation changes the name or record type.
    let old = records::getRecord(&state.db, id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Record {} not found", id)))?;

    let old_name = old.name.clone();
    let mut body = body;
    if let Some(ref mut name) = body.name {
        *name = name.trim_end_matches('.').to_lowercase();
    }

    let updated = records::updateRecord(&state.db, id, &body)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Record {} not found", id)))?;

    records::deleteCacheForName(&state.db, &old_name).await?;
    if updated.name != old_name {
        records::deleteCacheForName(&state.db, &updated.name).await?;
    }
    {
        let mut cache = state.cache.write().await;
        if let Ok(rtype) = old.record_type.parse::<hickory_proto::rr::RecordType>() {
            cache.remove(&old_name, rtype);
        }
        if let Ok(rtype) = updated.record_type.parse::<hickory_proto::rr::RecordType>() {
            cache.remove(&updated.name, rtype);
        }
    }

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
    let row = records::getRecord(&state.db, id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Record {} not found", id)))?;

    let deleted = records::deleteRecord(&state.db, id).await?;
    if !deleted {
        return Err(ApiError::NotFound(format!("Record {} not found", id)));
    }

    // Remove all cached answers for the name, since cached records may depend
    // on the deleted record through a CNAME chain.
    records::deleteCacheForName(&state.db, &row.name).await?;
    if let Ok(rtype) = row.record_type.parse::<hickory_proto::rr::RecordType>() {
        state.cache.write().await.remove(&row.name, rtype);
    }

    tracing::info!(id, "DNS record deleted");
    let _ = state.log_tx.send(format!("[CRUD] DELETE record id={}", id));

    Ok(Json(serde_json::json!({ "deleted": id })))
}
