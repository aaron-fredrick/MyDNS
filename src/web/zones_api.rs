use std::sync::Arc;

use axum::{
    extract::{Path, State},
    Json,
};
use serde::Deserialize;

use crate::db::records;
use crate::dns::zone_trie::ZoneTrie;
use crate::state::AppState;
use crate::web::auth::JwtClaims;
use crate::web::error::ApiError;

#[derive(Deserialize)]
pub struct AddZoneRequest {
    pub name: String,
}

fn validate_zone_name(name: &str) -> Result<String, ApiError> {
    // The root zone "." is the only single-character zone allowed.
    if name == "." {
        return Ok(".".to_string());
    }
    let normalized = name.trim_end_matches('.').to_lowercase();
    if normalized.is_empty() {
        return Err(ApiError::BadRequest("Zone name must not be empty".into()));
    }
    // Reject anything that looks like a URL path or contains invalid chars.
    if normalized.contains('/') || normalized.contains(':') || normalized.contains('@') {
        return Err(ApiError::BadRequest(
            "Zone name must be a plain DNS domain (e.g. home.local or mydns.local)".into(),
        ));
    }
    // Each label must be non-empty and at most 63 chars; no leading/trailing hyphens.
    for label in normalized.split('.') {
        if label.is_empty() || label.len() > 63 {
            return Err(ApiError::BadRequest(
                "Zone name contains an empty or oversized label".into(),
            ));
        }
        if label.starts_with('-') || label.ends_with('-') {
            return Err(ApiError::BadRequest(
                "DNS labels must not start or end with '-'".into(),
            ));
        }
        if !label.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-') {
            return Err(ApiError::BadRequest(
                "DNS labels may only contain letters, digits, and hyphens".into(),
            ));
        }
    }
    Ok(normalized)
}

/// Rebuilds the live `ZoneTrie` from the current set of DB zones and swaps it
/// into `AppState`. Called after every add/remove so DNS behaviour is
/// immediately updated without a restart.
async fn reload_trie(state: &Arc<AppState>) -> Result<(), ApiError> {
    let zone_names = records::listZoneNames(&state.db)
        .await
        .map_err(|e| ApiError::Internal(e))?;
    let new_trie = ZoneTrie::from_zones(&zone_names);
    *state.zone_trie.write().await = new_trie;
    tracing::info!(zones = ?zone_names, "Zone trie reloaded");
    Ok(())
}

/// `GET /api/v1/zones`
#[allow(non_snake_case)]
pub async fn listZones(
    _claims: JwtClaims,
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let zones = records::listZones(&state.db).await?;
    Ok(Json(serde_json::json!({ "zones": zones })))
}

/// `POST /api/v1/zones`
#[allow(non_snake_case)]
pub async fn addZone(
    _claims: JwtClaims,
    State(state): State<Arc<AppState>>,
    Json(body): Json<AddZoneRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let canonical = validate_zone_name(&body.name)?;

    // Attempt insert; if it fails with a unique constraint the zone already exists.
    let zone = records::addZone(&state.db, &canonical)
        .await
        .map_err(|e| {
            let msg = e.to_string();
            if msg.contains("UNIQUE") || msg.contains("unique") {
                ApiError::BadRequest(format!("Zone '{}' already exists", canonical))
            } else {
                ApiError::Internal(anyhow::anyhow!(msg))
            }
        })?;

    reload_trie(&state).await?;

    tracing::info!(zone = %canonical, "Authoritative zone added");
    let _ = state
        .log_tx
        .send(format!("[ZONES] ADD zone={}", canonical));

    Ok(Json(serde_json::json!({ "zone": zone })))
}

/// `DELETE /api/v1/zones/:name`
#[allow(non_snake_case)]
pub async fn removeZone(
    _claims: JwtClaims,
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let canonical = validate_zone_name(&name)?;

    let removed = records::removeZone(&state.db, &canonical)
        .await
        .map_err(|e| ApiError::Internal(e))?;

    if !removed {
        return Err(ApiError::NotFound(format!(
            "Zone '{}' not found",
            canonical
        )));
    }

    reload_trie(&state).await?;

    tracing::info!(zone = %canonical, "Authoritative zone removed");
    let _ = state
        .log_tx
        .send(format!("[ZONES] REMOVE zone={}", canonical));

    Ok(Json(serde_json::json!({ "removed": canonical })))
}
