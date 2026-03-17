use std::sync::Arc;

use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::config::ResolverPriority;
use crate::db;
use crate::dns::upstream::UpstreamResolver;
use crate::state::AppState;
use crate::web::auth::JwtClaims;
use crate::web::error::ApiError;

#[derive(Serialize)]
pub struct SettingsResponse {
    pub resolver_priority: String,
    pub cloudflare_dns: String,
    pub router_dns: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateSettings {
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
    Ok(Json(SettingsResponse {
        resolver_priority: cfg.resolver_priority.to_string(),
        cloudflare_dns: cfg.cloudflare_dns.to_string(),
        router_dns: cfg.router_dns.map(|a| a.to_string()),
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
        cfg.resolver_priority.clone(),
        cfg.cloudflare_dns,
        cfg.router_dns,
    )
    .map_err(anyhow::Error::from)?;
    drop(cfg); // release write lock before acquiring upstream write lock
    *state.upstream.write().await = new_upstream;

    tracing::info!("Resolver settings updated");
    let _ = state.log_tx.send("[SETTINGS] Resolver settings updated".to_string());

    // Re-read to build response.
    let cfg = state.config.read().await;
    Ok(Json(SettingsResponse {
        resolver_priority: cfg.resolver_priority.to_string(),
        cloudflare_dns: cfg.cloudflare_dns.to_string(),
        router_dns: cfg.router_dns.map(|a| a.to_string()),
    }))
}
