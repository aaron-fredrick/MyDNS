use std::sync::Arc;

use axum::{
    extract::DefaultBodyLimit,
    http::{header as http_header, HeaderName, HeaderValue as HV, StatusCode},
    routing::{delete, get, post, put},
    Router,
};
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use tower_http::set_header::SetResponseHeaderLayer;

use crate::api::v1::{
    blocklist as blocklist_api, cache as cache_api, records as records_api,
    settings as settings_api, stats as stats_api, zones as zones_api,
};
use crate::state::AppState;
use crate::web::{auth, frontend, ws};

const MAX_BODY_BYTES: usize = 64 * 1024;

/// Builds the complete Axum application router with all API routes, the
/// WebSocket endpoint, frontend asset serving, middleware, CORS, and security
/// headers applied.
pub(crate) fn build_app(state: Arc<AppState>, cors: CorsLayer) -> axum::Router {
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
        .route("/stats/history", get(stats_api::get_stats_history))
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
        .route(
            "/blocklist",
            get(blocklist_api::list_blocklist).post(blocklist_api::add_blocklist_entry),
        )
        .route(
            "/blocklist/:id",
            put(blocklist_api::update_blocklist_entry)
                .delete(blocklist_api::delete_blocklist_entry),
        )
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

    Router::new()
        .nest("/api/v1", api_routes)
        .route("/ws", get(ws::ws_handler))
        .route("/", get(frontend::serve_frontend_root))
        .route("/*path", get(frontend::serve_frontend))
        .layer(DefaultBodyLimit::max(MAX_BODY_BYTES))
        .layer(security_headers)
        .layer(cors)
        .with_state(state)
}

/// Constructs the CORS layer from the application configuration.
pub(crate) fn build_cors_layer(config: &crate::config::AppConfig) -> anyhow::Result<CorsLayer> {
    #[cfg(debug_assertions)]
    {
        let _ = config;
        Ok(CorsLayer::permissive())
    }

    #[cfg(not(debug_assertions))]
    {
        use axum::http::{header, Method};

        let origins = parse_cors_origins(config)?;

        Ok(CorsLayer::new()
            .allow_origin(origins)
            .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
            .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE, header::ACCEPT]))
    }
}

#[cfg(any(test, not(debug_assertions)))]
pub(crate) fn parse_cors_origins(
    config: &crate::config::AppConfig,
) -> anyhow::Result<Vec<axum::http::HeaderValue>> {
    use anyhow::Context;

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

    Ok(origins)
}

#[cfg(any(test, not(debug_assertions)))]
pub(crate) fn origin_header(host: &str, port: u16) -> anyhow::Result<axum::http::HeaderValue> {
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
