use std::sync::Arc;

use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::config::{self, ResolverMode, ResolverPriority};
use crate::db;
use crate::dns::upstream::UpstreamResolver;
use crate::error::ApiError;
use crate::state::AppState;
use crate::web::auth::JwtClaims;

#[derive(Serialize)]
pub struct SettingsResponse {
    pub resolver_mode: String,
    pub resolver_priority: String,
    pub cloudflare_dns: String,
    pub router_dns: Option<String>,
    pub root_hints: Vec<String>,
}

#[derive(Deserialize)]
pub struct UpdateSettings {
    pub resolver_mode: Option<String>,
    pub resolver_priority: Option<String>,
    pub cloudflare_dns: Option<String>,
    pub router_dns: Option<String>,
}

/// `GET /api/v1/settings`
pub async fn get_settings(
    _claims: JwtClaims,
    State(state): State<Arc<AppState>>,
) -> Result<Json<SettingsResponse>, ApiError> {
    let cfg = state.config.read().await;
    let root_hints = if cfg.root_hints.is_empty() {
        config::default_root_hints()
            .into_iter()
            .map(|a| a.to_string())
            .collect()
    } else {
        cfg.root_hints.iter().map(|a| a.to_string()).collect()
    };

    Ok(Json(SettingsResponse {
        resolver_mode: cfg.resolver_mode.to_string(),
        resolver_priority: cfg.resolver_priority.to_string(),
        cloudflare_dns: cfg.cloudflare_dns.to_string(),
        router_dns: cfg.router_dns.map(|a| a.to_string()),
        root_hints,
    }))
}

/// `PUT /api/v1/settings`
///
/// Applies changes immediately to the live [`AppState`] and persists them to
/// the `settings` DB table so they survive a restart.
pub async fn update_settings(
    _claims: JwtClaims,
    State(state): State<Arc<AppState>>,
    Json(body): Json<UpdateSettings>,
) -> Result<Json<SettingsResponse>, ApiError> {
    let mut cfg = state.config.write().await;

    if let Some(ref mode_str) = body.resolver_mode {
        cfg.resolver_mode = mode_str
            .parse::<ResolverMode>()
            .map_err(|e| ApiError::BadRequest(e.to_string()))?;
        db::set_setting(&state.db, "resolver_mode", mode_str).await?;
    }

    if let Some(ref prio_str) = body.resolver_priority {
        cfg.resolver_priority = prio_str
            .parse::<ResolverPriority>()
            .map_err(|e| ApiError::BadRequest(e.to_string()))?;
        db::set_setting(&state.db, "resolver_priority", prio_str).await?;
    }

    if let Some(ref addr_str) = body.cloudflare_dns {
        cfg.cloudflare_dns = addr_str
            .parse()
            .map_err(|_| ApiError::BadRequest("Invalid cloudflare_dns address".into()))?;
        db::set_setting(&state.db, "cloudflare_dns", addr_str).await?;
    }

    if let Some(ref addr_str) = body.router_dns {
        let addr = addr_str
            .parse()
            .map_err(|_| ApiError::BadRequest("Invalid router_dns address".into()))?;
        cfg.router_dns = Some(addr);
        db::set_setting(&state.db, "router_dns", addr_str).await?;
    }

    // Rebuild the upstream resolver chain with the updated config.
    let new_upstream = UpstreamResolver::from_config(
        cfg.resolver_mode.clone(),
        cfg.resolver_priority.clone(),
        cfg.cloudflare_dns,
        cfg.router_dns,
        cfg.root_hints.clone(),
    )?;
    drop(cfg); // release write lock before acquiring upstream write lock
    *state.upstream.write().await = new_upstream;

    tracing::info!("Resolver settings updated");
    let _ = state
        .log_tx
        .send("[SETTINGS] Resolver settings updated".to_string());

    // Re-read to build response.
    let cfg = state.config.read().await;
    let root_hints = if cfg.root_hints.is_empty() {
        config::default_root_hints()
            .into_iter()
            .map(|a| a.to_string())
            .collect()
    } else {
        cfg.root_hints.iter().map(|a| a.to_string()).collect()
    };

    Ok(Json(SettingsResponse {
        resolver_mode: cfg.resolver_mode.to_string(),
        resolver_priority: cfg.resolver_priority.to_string(),
        cloudflare_dns: cfg.cloudflare_dns.to_string(),
        router_dns: cfg.router_dns.map(|a| a.to_string()),
        root_hints,
    }))
}
