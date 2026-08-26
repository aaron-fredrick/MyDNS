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
use crate::web::validation;

async fn cache_invalidation_names(
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

async fn invalidate_caches(state: &Arc<AppState>, names: &[String]) -> anyhow::Result<()> {
    let names = cache_invalidation_names(&state.db, names).await?;
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
    validation::validate_create_record(&body)?;

    let allowed_zones = state.config.read().await.allowed_zones.clone();
    validation::validate_zone(&body.name, &allowed_zones)?;

    body.name = body.name.trim_end_matches('.').to_lowercase();
    body.record_type = body.record_type.trim().to_ascii_uppercase();
    body.value = body.value.trim().to_string();

    let row = records::createRecord(&state.db, &body).await?;
    invalidate_caches(&state, std::slice::from_ref(&body.name)).await?;

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
    validation::validate_update_record(&body)?;

    // If name is being changed, check it against allowed zones
    if let Some(ref new_name) = body.name {
        let allowed_zones = state.config.read().await.allowed_zones.clone();
        validation::validate_zone(new_name, &allowed_zones)?;
    }

    let old = records::getRecord(&state.db, id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Record {} not found", id)))?;

    let old_name = old.name.clone();
    let mut invalidation_names =
        cache_invalidation_names(&state.db, std::slice::from_ref(&old_name)).await?;

    let new_name = body
        .name
        .as_deref()
        .unwrap_or(&old.name)
        .trim_end_matches('.')
        .to_lowercase();
    let new_type = body
        .record_type
        .as_deref()
        .unwrap_or(&old.record_type)
        .trim()
        .to_ascii_uppercase();
    let new_value = body
        .value
        .as_deref()
        .unwrap_or(&old.value)
        .trim()
        .to_string();
    let new_ttl = body.ttl.unwrap_or(old.ttl as u32);
    let new_priority = if new_type == "MX" {
        body.priority.or(old.priority.map(|value| value as u16))
    } else {
        None
    };

    validation::validate_record(&new_name, &new_type, &new_value, new_ttl, new_priority)?;

    let mut body = body;
    if let Some(ref mut name) = body.name {
        *name = new_name;
    }
    if let Some(ref mut record_type) = body.record_type {
        *record_type = new_type;
    }
    if let Some(ref mut value) = body.value {
        *value = new_value;
    }

    let updated = records::updateRecord(&state.db, id, &body)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Record {} not found", id)))?;

    invalidation_names
        .extend(cache_invalidation_names(&state.db, std::slice::from_ref(&updated.name)).await?);
    invalidate_caches(&state, &invalidation_names).await?;

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

    let invalidation_names =
        cache_invalidation_names(&state.db, std::slice::from_ref(&row.name)).await?;

    let deleted = records::deleteRecord(&state.db, id).await?;
    if !deleted {
        return Err(ApiError::NotFound(format!("Record {} not found", id)));
    }

    invalidate_caches(&state, &invalidation_names).await?;

    tracing::info!(id, "DNS record deleted");
    let _ = state.log_tx.send(format!("[CRUD] DELETE record id={}", id));

    Ok(Json(serde_json::json!({ "deleted": id })))
}
