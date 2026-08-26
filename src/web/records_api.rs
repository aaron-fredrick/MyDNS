use std::collections::HashSet;
use std::sync::Arc;

use axum::{
    extract::{Path, State},
    Json,
};

use crate::db::records::{self, CreateRecord, UpdateRecord};
use crate::state::AppState;
use crate::web::auth::JwtClaims;
use crate::web::error::ApiError;

async fn cacheInvalidationNames(
    pool: &sqlx::SqlitePool,
    names: &[String],
) -> anyhow::Result<Vec<String>> {
    let mut invalidation = HashSet::new();
    for name in names {
        let normalized = name.trim_end_matches('.').to_lowercase();
        invalidation.insert(normalized.clone());
        for dependent in records::findCnameDependents(pool, &normalized).await? {
            invalidation.insert(dependent);
        }
    }
    Ok(invalidation.into_iter().collect())
}

async fn invalidateCaches(state: &Arc<AppState>, names: &[String]) -> anyhow::Result<()> {
    let names = cacheInvalidationNames(&state.db, names).await?;
    for name in &names {
        records::deleteCacheForName(&state.db, name).await?;
        state.cache.write().await.removeName(name);
    }
    Ok(())
}

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
    body.name = body.name.trim_end_matches('.').to_lowercase();

    let row = records::createRecord(&state.db, &body).await?;
    invalidateCaches(&state, std::slice::from_ref(&body.name)).await?;

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
    let old = records::getRecord(&state.db, id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Record {} not found", id)))?;

    let old_name = old.name.clone();
    // Capture dependents before the mutation because changing a CNAME target
    // can remove the old dependency from dns_records.
    let mut invalidation_names =
        cacheInvalidationNames(&state.db, std::slice::from_ref(&old_name)).await?;

    let mut body = body;
    if let Some(ref mut name) = body.name {
        *name = name.trim_end_matches('.').to_lowercase();
    }

    let updated = records::updateRecord(&state.db, id, &body)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Record {} not found", id)))?;

    invalidation_names.extend(
        cacheInvalidationNames(&state.db, std::slice::from_ref(&updated.name)).await?,
    );
    invalidateCaches(&state, &invalidation_names).await?;

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

    // Capture CNAME dependents before deleting the authoritative record.
    let invalidation_names =
        cacheInvalidationNames(&state.db, std::slice::from_ref(&row.name)).await?;

    let deleted = records::deleteRecord(&state.db, id).await?;
    if !deleted {
        return Err(ApiError::NotFound(format!("Record {} not found", id)));
    }

    invalidateCaches(&state, &invalidation_names).await?;

    tracing::info!(id, "DNS record deleted");
    let _ = state.log_tx.send(format!("[CRUD] DELETE record id={}", id));

    Ok(Json(serde_json::json!({ "deleted": id })))
}
