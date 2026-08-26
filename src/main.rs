use std::sync::Arc;

use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use mydns::{cache, config, db, dns, privileges, state, web};

use config::AppConfig;
use dns::upstream::UpstreamResolver;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // `.env` is a developer convenience only. Release builds never load it.
    #[cfg(debug_assertions)]
    dotenvy::dotenv().ok();

    // ── 2. Logging setup ──────────────────────────────────────────────────────
    let log_filename = {
        let now = chrono::Local::now();
        format!("mydns_{}.log", now.format("%Y-%m-%d_%H-%M-%S"))
    };
    std::fs::create_dir_all("logs")?;
    let file_appender = tracing_appender::rolling::never("logs", &log_filename);
    let (non_blocking_file, _file_guard) = tracing_appender::non_blocking(file_appender);

    // Broadcast channel used to stream log events specifically for the dashboard.
    let (log_tx, _) = broadcast::channel::<String>(1024);

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(non_blocking_file)
                .with_ansi(false),
        )
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stdout))
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    tracing::info!(log_file = %log_filename, "MyDNS starting");

    // ── 3. Configuration ──────────────────────────────────────────────────────
    // Production configuration is explicit and fail-fast. In particular,
    // admin credentials must be present in config.ini; there are no defaults.
    let mut cfg = AppConfig::fromConfigFile()?;
    tracing::info!(
        bind_host = %cfg.bind_host,
        dns_port = cfg.dns_port,
        http_host = %cfg.http_host,
        http_port = cfg.http_port,
        "Configuration loaded"
    );

    // ── 4. Database ───────────────────────────────────────────────────────────
    let pool = db::init(&cfg.db_path).await?;

    // Load persisted settings (resolver priority, upstream IPs, etc.) and apply.
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

    // ── 5. JWT secret persistence ─────────────────────────────────────────────
    // If no secret was explicitly configured, persist the generated secret so
    // existing sessions remain valid across restarts.
    if cfg.jwt_secret.is_empty() {
        if let Some(saved_secret) = db::getSetting(&pool, "jwt_secret").await? {
            cfg.jwt_secret = saved_secret;
        } else {
            cfg.jwt_secret = config::generateSecret(64);
            db::setSetting(&pool, "jwt_secret", &cfg.jwt_secret).await?;
            tracing::info!("Generated and persisted new JWT secret");
        }
    }

    // ── 6. Privilege check (post-config) ──────────────────────────────────────
    privileges::checkAndExitIfInsufficient(cfg.dns_port, cfg.http_port);

    // ── 6. Seed admin user ────────────────────────────────────────────────────
    if db::records::findUserHash(&pool, &cfg.admin_username)
        .await?
        .is_none()
    {
        let hash = web::auth::hashPassword(&cfg.admin_password)?;
        db::records::seedAdmin(&pool, &cfg.admin_username, &hash).await?;
        tracing::info!(username = %cfg.admin_username, "Admin user seeded");
    }

    // ── 6. Build upstream resolver ────────────────────────────────────────────
    let upstream = UpstreamResolver::fromConfig(
        cfg.resolver_priority.clone(),
        cfg.cloudflare_dns,
        cfg.router_dns,
    )?;

    // ── 7. Shared state ───────────────────────────────────────────────────────
    let cancel = CancellationToken::new();
    let state = state::AppState::new(pool.clone(), cfg, upstream, log_tx, cancel.clone());

    // ── 8. Background cache pruner ────────────────────────────────────────────
    cache::spawnPruner(Arc::clone(&state.cache), pool.clone(), cancel.clone());

    // ── 9. Spawn DNS and HTTP servers with fate-sharing ───────────────────────
    // The DNS server binds its privileged sockets before dropping Unix
    // privileges. The process then continues with reduced privileges.
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
        if let Err(e) = web::server::run(Arc::clone(&state), http_cancel).await {
            tracing::error!(error = %e, "HTTP server terminated with error");
        }
    });

    // Cancel propagates: whichever of DNS/HTTP exits first signals the other.
    let _ = tokio::join!(dns_handle, http_handle);

    tracing::info!("MyDNS shutdown complete");
    Ok(())
}
