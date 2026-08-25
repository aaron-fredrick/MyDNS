use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use hickory_server::ServerFuture;
use tokio::net::{TcpListener, UdpSocket};
use tokio_util::sync::CancellationToken;

use crate::dns::handler::DnsHandler;
use crate::state::AppState;

/// Binds the DNS server on UDP and TCP and runs until the cancellation token
/// fires or an unrecoverable error occurs.
///
/// On Unix the sockets are bound before privileges are dropped. This is
/// necessary when the configured DNS port is below 1024 (for example, 53).
pub async fn run(state: Arc<AppState>, cancel: CancellationToken) -> anyhow::Result<()> {
    let port = {
        let cfg = state.config.read().await;
        cfg.dns_port
    };

    let udp = UdpSocket::bind(("0.0.0.0", port))
        .await
        .with_context(|| format!("Failed to bind UDP socket on port {}", port))?;

    let tcp = TcpListener::bind(("0.0.0.0", port))
        .await
        .with_context(|| format!("Failed to bind TCP socket on port {}", port))?;

    tracing::info!(port, "DNS server bound (UDP + TCP)");

    // The privileged bind must happen before this process drops its Unix
    // privileges. The sockets remain usable by the unprivileged process.
    #[cfg(unix)]
    if let Err(e) = crate::privileges::dropPrivileges() {
        tracing::warn!(error = %e, "Privilege drop failed — continuing with current privileges");
    }

    let handler = DnsHandler::new(state);
    let mut server = ServerFuture::new(handler);
    server.register_socket(udp);
    server.register_listener(tcp, Duration::from_secs(30));

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
