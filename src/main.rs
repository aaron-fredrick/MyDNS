use std::sync::Arc;

use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;

use mydns::{cache, config, db, dns, observability, privileges, state, web};

use config::AppConfig;
use dns::blocklist::BlocklistIndex;
use dns::record_index::RecordIndex;
use dns::upstream::UpstreamResolver;
use dns::zone_trie::ZoneTrie;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    #[cfg(debug_assertions)]
    dotenvy::dotenv().ok();

    let logging_config = observability::telemetry::logging::LoggingConfig::default();
    let _logging_guard = observability::telemetry::pipeline::init(logging_config)?;

    tracing::info!(log_file = %_logging_guard.log_filename, "MyDNS starting");

    let (log_tx, _) = broadcast::channel::<String>(1024);

    let mut cfg = AppConfig::from_config_file()?;
    tracing::info!(
        bind_host = %cfg.bind_host,
        dns_port = cfg.dns_port,
        http_host = %cfg.http_host,
        http_port = cfg.http_port,
        timezone = %cfg.timezone,
        "Configuration loaded"
    );

    let pool = db::init(&cfg.db_path).await?;

    if let Some(prio) = db::settings::get_setting(&pool, "resolver_priority").await? {
        if let Ok(p) = prio.parse::<config::ResolverPriority>() {
            cfg.resolver_priority = p;
        }
    }
    if let Some(cf) = db::settings::get_setting(&pool, "cloudflare_dns").await? {
        if let Ok(a) = cf.parse::<std::net::SocketAddr>() {
            cfg.cloudflare_dns = a;
        }
    }
    if let Some(rt) = db::settings::get_setting(&pool, "router_dns").await? {
        cfg.router_dns = rt.parse::<std::net::SocketAddr>().ok();
    }

    if cfg.jwt_secret.is_empty() {
        if let Some(saved_secret) = db::settings::get_setting(&pool, "jwt_secret").await? {
            cfg.jwt_secret = saved_secret;
        } else {
            cfg.jwt_secret = config::generate_secret(64);
            db::settings::set_setting(&pool, "jwt_secret", &cfg.jwt_secret).await?;
            tracing::info!("Generated and persisted new JWT secret");
        }
    }

    privileges::check_and_exit_if_insufficient(cfg.dns_port, cfg.http_port);

    if db::users::find_user_hash(&pool, &cfg.admin_username)
        .await?
        .is_none()
    {
        let hash = web::auth::hash_password(&cfg.admin_password)?;
        db::users::seed_admin(&pool, &cfg.admin_username, &hash).await?;
        tracing::info!(username = %cfg.admin_username, "Admin user seeded");
    }
    cfg.admin_password.clear();

    let upstream = UpstreamResolver::from_config(
        cfg.resolver_mode.clone(),
        cfg.resolver_priority.clone(),
        cfg.cloudflare_dns,
        cfg.router_dns,
        cfg.root_hints.clone(),
    )?;

    let purged = db::records::delete_dev_records(&pool).await?;
    if purged > 0 {
        tracing::info!(count = purged, "Purged ephemeral dev records on startup");
    }

    db::zones::seed_zones(&pool, &cfg.allowed_zones).await?;

    let zone_names = db::zones::list_zone_names(&pool).await?;
    tracing::info!(zones = ?zone_names, "Local DNS zones loaded from DB");
    let zone_trie = ZoneTrie::from_zones(&zone_names);
    let record_index = RecordIndex::load_from_db(&pool).await?;

    let enabled_domains = db::blocklist::list_enabled_domains(&pool).await?;
    tracing::info!(count = enabled_domains.len(), "Blocklist loaded from DB");
    let blocklist_index = BlocklistIndex::from_domains(&enabled_domains);

    let timezone = cfg
        .timezone
        .parse::<chrono_tz::Tz>()
        .map_err(|error| anyhow::anyhow!("Invalid configured timezone '{}': {error}", cfg.timezone))?;

    let observability_db = Arc::new(
        observability::database::ObservabilityDatabase::init(&cfg.observability_db_path).await?,
    );
    let telemetry_metrics =
        observability::telemetry::metrics::aggregator::MetricsAggregator::new(timezone);

    let cancel = CancellationToken::new();
    let state = state::AppState::new(
        pool.clone(),
        cfg,
        upstream,
        log_tx,
        cancel.clone(),
        record_index,
        zone_trie,
        blocklist_index,
        Arc::clone(&telemetry_metrics),
        Arc::clone(&observability_db),
    );

    let metrics_persistence = observability::telemetry::metrics::persistence::spawn_persistence(
        Arc::clone(&telemetry_metrics),
        Arc::clone(&observability_db),
        cancel.clone(),
    );

    {
        let metrics = Arc::clone(&state.metrics);
        state.upstream.write().await.attach_metrics(metrics);
    }

    cache::spawn_pruner(Arc::clone(&state.cache), pool.clone(), cancel.clone());

    {
        let signal_cancel = cancel.clone();
        tokio::spawn(async move {
            await_shutdown_signal().await;
            tracing::info!("Shutdown signal received — stopping MyDNS");
            signal_cancel.cancel();
        });
    }

    let dns_state = Arc::clone(&state);
    let dns_cancel = cancel.clone();
    let dns_handle = tokio::spawn(async move {
        if let Err(e) = dns::server::run(dns_state, dns_cancel.clone()).await {
            tracing::error!(error = %e, "DNS server terminated with error");
        }
        tracing::warn!("DNS server exited — triggering shutdown");
        dns_cancel.cancel();
    });

    let http_cancel = cancel.clone();
    let http_handle = tokio::spawn(async move {
        if let Err(e) = web::server::run(Arc::clone(&state), http_cancel.clone()).await {
            tracing::error!(error = %e, "HTTP server terminated with error");
        }
        http_cancel.cancel();
    });

    let _ = tokio::join!(dns_handle, http_handle);
    let _ = metrics_persistence.await;
    tracing::info!("MyDNS shutdown complete");
    Ok(())
}

async fn await_shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        let mut sigint =
            signal(SignalKind::interrupt()).expect("Failed to register SIGINT handler");
        let mut sigterm =
            signal(SignalKind::terminate()).expect("Failed to register SIGTERM handler");
        tokio::select! { _ = sigint.recv() => {}, _ = sigterm.recv() => {} }
    }
    #[cfg(not(unix))]
    {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to register Ctrl+C");
    }
}
