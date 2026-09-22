use std::sync::Arc;

use anyhow::Context;
use tokio_util::sync::CancellationToken;

use crate::state::AppState;
use crate::web::routes;

/// Binds the HTTP server, serves the application until the cancellation token
/// fires, and shuts down gracefully.
pub async fn run(state: Arc<AppState>, cancel: CancellationToken) -> anyhow::Result<()> {
    let config = state.config.read().await.clone();
    let port = config.http_port;
    let cors = routes::build_cors_layer(&config)?;

    let app = routes::build_app(Arc::clone(&state), cors);

    let listener = tokio::net::TcpListener::bind((config.http_host, port))
        .await
        .with_context(|| {
            format!(
                "Failed to bind HTTP server on {}:{}",
                config.http_host, port
            )
        })?;

    tracing::info!(host = %config.http_host, port, "HTTP server listening");

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(async move { cancel.cancelled().await })
    .await
    .context("HTTP server error")?;

    Ok(())
}
