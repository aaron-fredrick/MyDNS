use std::sync::Arc;

use anyhow::Context;
use axum::{
    routing::{get, post, put, delete},
    Router,
};
use tokio_util::sync::CancellationToken;
use tower_http::cors::CorsLayer;

use crate::state::AppState;
use crate::web::{
    auth, cache_api, dashboard, records_api, settings_api, stats_api, ws,
};

/// Constructs the full Axum router and binds the HTTP server.
pub async fn run(state: Arc<AppState>, cancel: CancellationToken) -> anyhow::Result<()> {
    let port = state.config.read().await.http_port;

    let api_routes = Router::new()
        .route("/auth/login", post(auth::login))
        .route("/records", get(records_api::list).post(records_api::create))
        .route(
            "/records/:id",
            put(records_api::update).delete(records_api::delete),
        )
        .route("/stats", get(stats_api::get_stats))
        .route("/settings", get(settings_api::get_settings).put(settings_api::update_settings))
        .route("/cache", get(cache_api::list_cache).delete(cache_api::clear_cache))
        .route("/cache/:name/:rtype", delete(cache_api::delete_cache_entry));

    let app = Router::new()
        .route("/", get(dashboard::serve_dashboard))
        .route("/ws", get(ws::ws_handler))
        .nest("/api/v1", api_routes)
        .layer(CorsLayer::permissive())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port))
        .await
        .with_context(|| format!("Failed to bind HTTP server on port {}", port))?;

    tracing::info!(port, "HTTP dashboard server listening");

    axum::serve(listener, app)
        .with_graceful_shutdown(async move { cancel.cancelled().await })
        .await
        .context("HTTP server error")?;

    Ok(())
}
