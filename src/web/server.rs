use std::sync::Arc;

use anyhow::Context;
use axum::{
    extract::DefaultBodyLimit,
    http::{header as http_header, HeaderName, HeaderValue as HV, StatusCode},
    routing::{delete, get, post, put},
    Router,
};
use rust_embed::Embed;
use tokio_util::sync::CancellationToken;
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use tower_http::set_header::SetResponseHeaderLayer;

use crate::state::AppState;
use crate::web::{auth, cache_api, records_api, settings_api, stats_api, ws, zones_api};

const MAX_BODY_BYTES: usize = 64 * 1024;

/// Production frontend assets are embedded into the MyDNS binary after the
/// Vite build. `allow_missing` keeps ordinary Rust-only development builds
/// possible; release builds must produce `out/web` before packaging.
#[derive(Embed)]
#[folder = "out/web/"]
#[allow_missing = true]
struct FrontendAssets;

pub async fn run(state: Arc<AppState>, cancel: CancellationToken) -> anyhow::Result<()> {
    let config = state.config.read().await.clone();
    let port = config.http_port;
    let cors = build_cors_layer(&config)?;

    // Keep the existing API surface isolated from the SPA fallback. Unknown
    // API routes must remain 404 instead of receiving index.html.
    let api_routes = Router::new()
        .route("/auth/login", post(auth::login))
        .route(
            "/records",
            get(records_api::list_records).post(records_api::create_record),
        )
        .route(
            "/records/:id",
            put(records_api::update_record).delete(records_api::delete_record),
        )
        .route("/stats", get(stats_api::get_stats))
        .route(
            "/settings",
            get(settings_api::get_settings).put(settings_api::update_settings),
        )
        .route(
            "/cache",
            get(cache_api::list_cache).delete(cache_api::clear_cache),
        )
        .route("/cache/:name/:rtype", delete(cache_api::delete_cache_entry))
        .route(
            "/zones",
            get(zones_api::list_zones).post(zones_api::add_zone),
        )
        .route("/zones/:name", delete(zones_api::remove_zone))
        .fallback(|| async { StatusCode::NOT_FOUND });

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
            HV::from_static(
                "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; connect-src 'self' ws: wss:; img-src 'self' data:; font-src 'self' data:;",
            ),
        ));

    let app = Router::new()
        .nest("/api/v1", api_routes)
        .route("/ws", get(ws::ws_handler))
        .route("/", get(serve_frontend_root))
        .route("/*path", get(serve_frontend))
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

/// Serve a concrete Vite asset when it exists, otherwise fall back to the SPA
/// entry point so client-side routes such as `/records` work on refresh.
async fn serve_frontend(
    axum::extract::Path(path): axum::extract::Path<String>,
) -> axum::response::Response {
    serve_asset(&path)
}

async fn serve_frontend_root() -> axum::response::Response {
    serve_asset("")
}

fn serve_asset(path: &str) -> axum::response::Response {
    use axum::body::Body;
    use axum::http::Response;
    use axum::response::IntoResponse;

    let normalized = path.trim_start_matches('/');
    let asset = FrontendAssets::get(normalized).or_else(|| FrontendAssets::get("index.html"));
    let Some(asset) = asset else {
        return StatusCode::NOT_FOUND.into_response();
    };

    let mime = if normalized.is_empty() {
        "text/html; charset=utf-8".to_string()
    } else {
        mime_guess::from_path(normalized)
            .first_or_octet_stream()
            .essence_str()
            .to_string()
    };

    Response::builder()
        .status(StatusCode::OK)
        .header(http_header::CONTENT_TYPE, mime)
        .body(Body::from(asset.data.into_owned()))
        .expect("valid frontend response")
}

fn build_cors_layer(config: &crate::config::AppConfig) -> anyhow::Result<CorsLayer> {
    #[cfg(debug_assertions)]
    {
        let _ = config;
        Ok(CorsLayer::permissive())
    }

    #[cfg(not(debug_assertions))]
    {
        use axum::http::{header, Method};

        let mut origins = Vec::new();
        let bind_hosts = if config.http_host.is_unspecified() {
            let mut hosts = vec![config.http_host];
            let interfaces = local_ip_address::list_afinet_netifas()
                .context("Failed to enumerate local network interfaces")?;
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
            if !host.is_unspecified() {
                origins.push(origin_header(&host.to_string(), config.http_port)?);
            }
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

        Ok(CorsLayer::new()
            .allow_origin(origins)
            .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
            .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE, header::ACCEPT]))
    }
}

#[cfg(not(debug_assertions))]
fn origin_header(host: &str, port: u16) -> anyhow::Result<axum::http::HeaderValue> {
    use std::net::IpAddr;

    let origin = if host.parse::<IpAddr>().is_ok() && host.contains(':') {
        if port == 80 {
            format!("http://[{host}]")
        } else {
            format!("http://[{host}]:{port}")
        }
    } else if port == 80 {
        format!("http://{host}")
    } else {
        format!("http://{host}:{port}")
    };

    axum::http::HeaderValue::from_str(&origin)
        .map_err(|error| anyhow::anyhow!("Invalid CORS origin '{}': {}", origin, error))
}
