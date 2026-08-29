use std::sync::Arc;

use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use mydns::{cache, config, db, dns, privileges, state, web};

use config::AppConfig;
use dns::record_index::RecordIndex;
use dns::upstream::UpstreamResolver;
use dns::zone_trie::ZoneTrie;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    #[cfg(debug_assertions)]
    dotenvy::dotenv().ok();

    let log_filename = {
        let now = chrono::Local::now();
        format!("mydns_{}.log", now.format("%Y-%m-%d_%H-%M-%S"))
    };
    std::fs::create_dir_all("logs")?;
    let file_appender = tracing_appender::rolling::never("logs", &log_filename);
    let (non_blocking_file, _file_guard) = tracing_appender::non_blocking(file_appender);
    let (log_tx, _) = broadcast::channel::<String>(1024);

    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(non_blocking_file)
                .with_ansi(false),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(std::io::stdout)
                .with_ansi(true),
        )
        .init();

    tracing::info!(log_file = %log_filename, "MyDNS starting");

    let mut cfg = AppConfig::from_config_file()?;
    tracing::info!(bind_host = %cfg.bind_host, dns_port = cfg.dns_port, http_host = %cfg.http_host, http_port = cfg.http_port, "Configuration loaded");

    let pool = db::init(&cfg.db_path).await?;

    if let Some(prio) = db::getSetting(&pool, "resolver_priority").await? {
        if let Ok(p) = prio.parse() {
            cfg.resolver_priority = p;
        }
    }
    if let Some(cf) = db::getSetting(&pool, "cloudflare_dns").await? {
        if let Ok(a) = cf.parse() {
            cfg.cloudflare_dns = a;
        }
    }
    if let Some(rt) = db::getSetting(&pool, "router_dns").await? {
        cfg.router_dns = rt.parse().ok();
    }

    if cfg.jwt_secret.is_empty() {
        if let Some(saved_secret) = db::getSetting(&pool, "jwt_secret").await? {
            cfg.jwt_secret = saved_secret;
        } else {
            cfg.jwt_secret = config::generate_secret(64);
            db::setSetting(&pool, "jwt_secret", &cfg.jwt_secret).await?;
            tracing::info!("Generated and persisted new JWT secret");
        }
    }

    privileges::checkAndExitIfInsufficient(cfg.dns_port, cfg.http_port);

    if db::records::findUserHash(&pool, &cfg.admin_username)
        .await?
        .is_none()
    {
        let hash = web::auth::hashPassword(&cfg.admin_password)?;
        db::records::seedAdmin(&pool, &cfg.admin_username, &hash).await?;
        tracing::info!(username = %cfg.admin_username, "Admin user seeded");
    }
    cfg.admin_password.clear();

    let upstream = UpstreamResolver::fromConfig(
        cfg.resolver_mode.clone(),
        cfg.resolver_priority.clone(),
        cfg.cloudflare_dns,
        cfg.router_dns,
        cfg.root_hints.clone(),
    )?;

    // Purge ephemeral dev records before building the index so they never
    // survive a restart. This must happen before RecordIndex::load_from_db.
    let purged = db::records::deleteDevRecords(&pool).await?;
    if purged > 0 {
        tracing::info!(count = purged, "Purged ephemeral dev records on startup");
    }

    // Seed DB zones from config (idempotent — skips duplicates).
    db::records::seedZones(&pool, &cfg.allowed_zones).await?;

    // Build the live trie from DB so zone changes made via the API persist
    // across restarts without requiring a config file edit.
    let zone_names = db::records::listZoneNames(&pool).await?;
    tracing::info!(zones = ?zone_names, "Authoritative zones loaded from DB");
    let zone_trie = ZoneTrie::from_zones(&zone_names);
    let record_index = RecordIndex::load_from_db(&pool).await?;

    let cancel = CancellationToken::new();
    let state = state::AppState::new(pool.clone(), cfg, upstream, log_tx, cancel.clone(), record_index, zone_trie);

    // Attach the shared collector after AppState owns the resolver. This keeps
    // upstream telemetry in the resolver implementation without coupling it to HTTP.
    {
        let metrics = Arc::clone(&state.metrics);
        state.upstream.write().await.attach_metrics(metrics);
    }

    cache::spawnPruner(Arc::clone(&state.cache), pool.clone(), cancel.clone());

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
            .expect("Failed to register Ctrl+C handler");
    }
}
