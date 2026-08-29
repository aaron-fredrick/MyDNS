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
///
/// Returns all records including dev (ephemeral) records so the UI can
/// distinguish and display them with an appropriate badge.
#[allow(non_snake_case)]
pub async fn listRecords(
    _claims: JwtClaims,
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let rows = records::listAllRecords(&state.db).await?;
    Ok(Json(serde_json::json!({ "records": rows })))
}

/// `POST /api/v1/records`
///
/// When `is_dev = false` (default), the record name must belong to an
/// authoritative zone in the DB. When `is_dev = true`, zone validation is
/// skipped and the record is marked ephemeral — it will be purged on the next
/// server restart.
#[allow(non_snake_case)]
pub async fn createRecord(
    _claims: JwtClaims,
    State(state): State<Arc<AppState>>,
    Json(mut body): Json<CreateRecord>,
) -> Result<Json<serde_json::Value>, ApiError> {
    validation::validate_create_record(&body)?;

    if body.is_dev {
        // Dev records bypass zone ownership — they exist solely for ephemeral
        // local testing and resolve via the record index during this session.
    } else {
        // For authoritative records, validate against the live DB zones so
        // that zone changes made via the API are reflected immediately.
        let zone_names = records::listZoneNames(&state.db)
            .await
            .map_err(|e| ApiError::Internal(e))?;
        validation::validate_zone(&body.name, &zone_names)?;
    }

    body.name = body.name.trim_end_matches('.').to_lowercase();
    body.record_type = body.record_type.trim().to_ascii_uppercase();
    body.value = body.value.trim().to_string();

    let row = records::createRecord(&state.db, &body).await?;
    invalidate_caches(&state, std::slice::from_ref(&body.name)).await?;
    state.record_index.write().await.upsert(row.clone());

    tracing::info!(
        name = %row.name,
        r#type = %row.record_type,
        is_dev = row.is_dev,
        "DNS record created"
    );
    let _ = state.log_tx.send(format!(
        "[CRUD] CREATE record id={} name={} is_dev={}",
        row.id, row.name, row.is_dev
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

    // If name is being changed, check it against allowed zones (unless the
    // existing record is a dev record — dev records may point at any domain).
    let old = records::getRecord(&state.db, id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Record not found".into()))?;

    if let Some(ref new_name) = body.name {
        if !old.is_dev {
            let zone_names = records::listZoneNames(&state.db)
                .await
                .map_err(|e| ApiError::Internal(e))?;
            validation::validate_zone(new_name, &zone_names)?;
        }
    }

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

    {
        let mut index = state.record_index.write().await;
        index.remove_by_id(id);
        index.upsert(updated.clone());
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

    let invalidation_names =
        cache_invalidation_names(&state.db, std::slice::from_ref(&row.name)).await?;

    let deleted = records::deleteRecord(&state.db, id).await?;
    if !deleted {
        return Err(ApiError::NotFound(format!("Record {} not found", id)));
    }

    invalidate_caches(&state, &invalidation_names).await?;
    state.record_index.write().await.remove_by_id(id);

    tracing::info!(id, "DNS record deleted");
    let _ = state.log_tx.send(format!("[CRUD] DELETE record id={}", id));

    Ok(Json(serde_json::json!({ "deleted": id })))
}
