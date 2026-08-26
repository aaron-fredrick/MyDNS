use std::sync::Arc;

#[cfg(not(debug_assertions))]
use std::net::IpAddr;

use anyhow::Context;
#[cfg(not(debug_assertions))]
use axum::http::{header, HeaderValue, Method};
use axum::{
    extract::DefaultBodyLimit,
    http::{header as http_header, HeaderName, HeaderValue as HV},
    routing::{delete, get, post, put},
    Router,
};
use tokio_util::sync::CancellationToken;
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use tower_http::set_header::SetResponseHeaderLayer;

use crate::state::AppState;
use crate::web::{auth, cache_api, dashboard, records_api, settings_api, stats_api, ws};

/// Maximum allowed HTTP request body size (64 KiB).
const MAX_BODY_BYTES: usize = 64 * 1024;

/// Constructs the full Axum router and binds the HTTP server.
pub async fn run(state: Arc<AppState>, cancel: CancellationToken) -> anyhow::Result<()> {
    let config = state.config.read().await.clone();
    let port = config.http_port;
    let cors = build_cors_layer(&config)?;

    let api_routes = Router::new()
        .route("/auth/login", post(auth::login))
        .route(
            "/records",
            get(records_api::listRecords).post(records_api::createRecord),
        )
        .route(
            "/records/:id",
            put(records_api::updateRecord).delete(records_api::deleteRecord),
        )
        .route("/stats", get(stats_api::getStats))
        .route(
            "/settings",
            get(settings_api::getSettings).put(settings_api::updateSettings),
        )
        .route(
            "/cache",
            get(cache_api::listCache).delete(cache_api::clearCache),
        )
        .route("/cache/:name/:rtype", delete(cache_api::deleteCacheEntry));

    let security_headers = ServiceBuilder::new()
        .layer(SetResponseHeaderLayer::overriding(
            HeaderName::from_static("x-content-type-options"),
            HV::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            HeaderName::from_static("x-frame-options"),
            HV::from_static("DENY"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            HeaderName::from_static("referrer-policy"),
            HV::from_static("strict-origin-when-cross-origin"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            http_header::CONTENT_SECURITY_POLICY,
            // Dashboard-appropriate CSP: allow same-origin scripts/styles,
            // websocket connections back to self, and nothing else.
            HV::from_static(
                "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; connect-src 'self' ws: wss:;",
            ),
        ));

    let app = Router::new()
        .route("/", get(dashboard::serveDashboard))
        .route("/style.css", get(dashboard::serveStyles))
        .route("/app.js", get(dashboard::serveScripts))
        .route("/ws", get(ws::wsHandler))
        .nest("/api/v1", api_routes)
        .layer(DefaultBodyLimit::max(MAX_BODY_BYTES))
        .layer(security_headers)
        .layer(cors)
        .with_state(state);

    let listener = tokio::net::TcpListener::bind((config.http_host, port))
        .await
        .with_context(|| {
            format!(
                "Failed to bind HTTP server on {}:{}",
                config.http_host, port
            )
        })?;

    tracing::info!(host = %config.http_host, port, "HTTP dashboard server listening");

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(async move { cancel.cancelled().await })
    .await
    .context("HTTP server error")?;

    Ok(())
}

fn build_cors_layer(config: &crate::config::AppConfig) -> anyhow::Result<CorsLayer> {
    #[cfg(debug_assertions)]
    {
        let _ = config;
        Ok(CorsLayer::permissive())
    }

    #[cfg(not(debug_assertions))]
    {
        let origins = release_cors_origins(config)?;

        Ok(CorsLayer::new()
            .allow_origin(origins)
            .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
            .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE, header::ACCEPT]))
    }
}

#[cfg(not(debug_assertions))]
fn release_cors_origins(config: &crate::config::AppConfig) -> anyhow::Result<Vec<HeaderValue>> {
    let mut origins = Vec::new();

    let bind_hosts = if config.http_host.is_unspecified() {
        let mut hosts = vec![config.http_host];
        let interfaces = local_ip_address::list_afinet_netifas()
            .context("Failed to enumerate local network interfaces for release CORS")?;
        hosts.extend(
            interfaces
                .into_iter()
                .map(|(_, ip)| ip)
                .filter(|ip| !ip.is_unspecified()),
        );
        hosts
    } else {
        vec![config.http_host]
    };

    for host in bind_hosts {
        if host.is_unspecified() {
            continue;
        }
        origins.push(origin_header(&host.to_string(), config.http_port)?);
    }

    for domain in &config.cors_domains {
        let domain = domain.trim().trim_end_matches('.');
        if domain.is_empty() || domain.contains("://") || domain.contains('/') {
            anyhow::bail!(
                "Invalid cors_domains entry '{}': expected a hostname without scheme or path",
                domain
            );
        }
        origins.push(origin_header(domain, config.http_port)?);
    }

    origins.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
    origins.dedup_by(|a, b| a == b);

    if origins.is_empty() {
        anyhow::bail!("Release CORS origin allowlist is empty");
    }

    Ok(origins)
}

#[cfg(not(debug_assertions))]
fn origin_header(host: &str, port: u16) -> anyhow::Result<HeaderValue> {
    let origin = if host.parse::<IpAddr>().is_ok() && host.contains(':') {
        if port == 80 {
            format!("http://[{}]", host)
        } else {
            format!("http://[{}]:{}", host, port)
        }
    } else if port == 80 {
        format!("http://{}", host)
    } else {
        format!("http://{}:{}", host, port)
    };

    HeaderValue::from_str(&origin)
        .map_err(|error| anyhow::anyhow!("Invalid CORS origin '{}': {}", origin, error))
}
