use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;

use crate::{
    db, dns::blocklist::BlocklistIndex, error::ApiError, state::AppState, web::auth::JwtClaims,
};

/// `GET /api/v1/blocklist`
///
/// Returns all domains in the blocklist (both enabled and disabled).
pub async fn list_blocklist(
    _claims: JwtClaims,
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<db::blocklist::BlocklistEntry>>, ApiError> {
    let entries = db::blocklist::list_entries(&state.db)
        .await
        .map_err(ApiError::Internal)?;
    Ok(Json(entries))
}

/// `POST /api/v1/blocklist`
///
/// Adds a new domain to the blocklist.
pub async fn add_blocklist_entry(
    _claims: JwtClaims,
    State(state): State<Arc<AppState>>,
    Json(payload): Json<db::blocklist::CreateBlocklistEntry>,
) -> Result<Json<db::blocklist::BlocklistEntry>, ApiError> {
    // Validate domain before hitting SQLite
    if let Err(e) = db::blocklist::normalize_domain(&payload.domain) {
        return Err(ApiError::BadRequest(e.to_string()));
    }

    let entry = db::blocklist::create_entry(&state.db, &payload)
        .await
        .map_err(|e| {
            if e.to_string().contains("UNIQUE constraint failed") {
                ApiError::BadRequest("Domain is already in the blocklist".to_string())
            } else {
                ApiError::Internal(e)
            }
        })?;

    reload_blocklist_index(&state).await?;
    tracing::info!(domain = %entry.domain, "Blocklist entry added");
    let _ = state.log_tx.send(format!(
        "[BLOCKLIST] Added domain={} source={} enabled={}",
        entry.domain, entry.source, entry.enabled
    ));

    Ok(Json(entry))
}

/// `PUT /api/v1/blocklist/:id`
///
/// Updates an existing blocklist entry (e.g., toggles enabled status).
pub async fn update_blocklist_entry(
    _claims: JwtClaims,
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    Json(payload): Json<db::blocklist::UpdateBlocklistEntry>,
) -> Result<Json<db::blocklist::BlocklistEntry>, ApiError> {
    let entry = db::blocklist::update_entry(&state.db, id, &payload)
        .await
        .map_err(ApiError::Internal)?;

    let entry = match entry {
        Some(e) => e,
        None => return Err(ApiError::NotFound("Blocklist entry not found".into())),
    };

    reload_blocklist_index(&state).await?;
    tracing::info!(domain = %entry.domain, enabled = %entry.enabled, "Blocklist entry updated");
    let _ = state.log_tx.send(format!(
        "[BLOCKLIST] Updated domain={} enabled={}",
        entry.domain, entry.enabled
    ));

    Ok(Json(entry))
}

/// `DELETE /api/v1/blocklist/:id`
///
/// Removes a domain from the blocklist.
pub async fn delete_blocklist_entry(
    _claims: JwtClaims,
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    // Fetch it first just so we can log the domain name
    let entry = db::blocklist::get_entry(&state.db, id)
        .await
        .map_err(ApiError::Internal)?;

    let domain = match entry {
        Some(e) => e.domain,
        None => return Err(ApiError::NotFound("Blocklist entry not found".into())),
    };

    let deleted = db::blocklist::delete_entry(&state.db, id)
        .await
        .map_err(ApiError::Internal)?;

    if !deleted {
        return Err(ApiError::NotFound("Blocklist entry not found".into()));
    }

    reload_blocklist_index(&state).await?;
    tracing::info!(domain = %domain, "Blocklist entry removed");
    let _ = state
        .log_tx
        .send(format!("[BLOCKLIST] Removed domain={}", domain));

    Ok(StatusCode::NO_CONTENT)
}

/// Hot-reloads the in-memory blocklist index from the database.
async fn reload_blocklist_index(state: &AppState) -> Result<(), ApiError> {
    let enabled_domains = db::blocklist::list_enabled_domains(&state.db)
        .await
        .map_err(ApiError::Internal)?;

    let new_index = BlocklistIndex::from_domains(&enabled_domains);

    // Acquire write lock and swap
    let mut current_index = state.blocklist_index.write().await;
    *current_index = new_index;

    Ok(())
}
