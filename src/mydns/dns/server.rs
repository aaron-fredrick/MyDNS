use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use hickory_server::server::Server;
use tokio::net::{TcpListener, UdpSocket};
use tokio_util::sync::CancellationToken;

use crate::dns::metrics_handler::MetricsHandler;
use crate::state::AppState;

/// Binds the DNS server on UDP and TCP and runs until the cancellation token
/// fires or an unrecoverable error occurs.
pub async fn run(state: Arc<AppState>, cancel: CancellationToken) -> anyhow::Result<()> {
    let (bind_host, port) = {
        let cfg = state.config.read().await;
        (cfg.bind_host, cfg.dns_port)
    };

    let udp = UdpSocket::bind((bind_host, port))
        .await
        .with_context(|| format!("Failed to bind UDP socket on {}:{}", bind_host, port))?;

    let tcp = TcpListener::bind((bind_host, port))
        .await
        .with_context(|| format!("Failed to bind TCP socket on {}:{}", bind_host, port))?;

    run_with_sockets(state, cancel, udp, tcp).await
}

/// Runs the DNS server using sockets that are already bound.
///
/// Keeping listener ownership with the caller until this function takes
/// ownership eliminates the bind/release/rebind race used by integration
/// test fixtures and also makes socket startup failures observable.
pub async fn run_with_sockets(
    state: Arc<AppState>,
    cancel: CancellationToken,
    udp: UdpSocket,
    tcp: TcpListener,
) -> anyhow::Result<()> {
    let (bind_host, port) = {
        let cfg = state.config.read().await;
        (cfg.bind_host, cfg.dns_port)
    };

    tracing::info!(%bind_host, port, "DNS server bound (UDP + TCP)");

    #[cfg(unix)]
    {
        if nix::unistd::getuid().is_root() {
            let cfg = state.config.read().await;
            crate::privileges::drop_privileges(&cfg.run_as_user, &cfg.run_as_group)
                .context("Failed to drop Unix privileges after binding DNS sockets")?;
        } else {
            tracing::debug!("Running as non-root user; skipping Unix privilege drop");
        }
    }

    let handler = MetricsHandler::new(state);
    let mut server = Server::new(handler);
    server.register_socket(udp);
    server.register_listener(tcp, Duration::from_secs(30), 4096);

    tokio::select! {
        result = server.block_until_done() => {
            result.context("DNS server exited unexpectedly")?;
        }
        _ = cancel.cancelled() => {
            tracing::info!("DNS server received shutdown signal");
        }
    }

    Ok(())
}
