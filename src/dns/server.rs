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

    tracing::info!(%bind_host, port, "DNS server bound (UDP + TCP)");

    #[cfg(unix)]
    {
        let cfg = state.config.read().await;
        crate::privileges::drop_privileges(&cfg.run_as_user, &cfg.run_as_group)
            .context("Failed to drop Unix privileges after binding DNS sockets")?;
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
