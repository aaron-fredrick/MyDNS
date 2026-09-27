use std::net::IpAddr;
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

    let udp = bind_udp_socket(bind_host, port)
        .await
        .with_context(|| format!("Failed to bind UDP socket on {}:{}", bind_host, port))?;

    let tcp = TcpListener::bind((bind_host, port))
        .await
        .with_context(|| format!("Failed to bind TCP socket on {}:{}", bind_host, port))?;

    run_with_sockets(state, cancel, udp, tcp).await
}

async fn bind_udp_socket(bind_host: IpAddr, port: u16) -> std::io::Result<UdpSocket> {
    #[cfg(windows)]
    {
        use std::mem::size_of;
        use std::os::windows::io::AsRawSocket;
        use std::ptr::null_mut;
        use winapi::um::mswsock::SIO_UDP_CONNRESET;
        use winapi::um::winsock2::{WSAIoctl, SOCKET};

        let socket = std::net::UdpSocket::bind((bind_host, port))?;
        let mut disable_connreset: u32 = 0;
        let mut bytes_returned: u32 = 0;

        let result = unsafe {
            WSAIoctl(
                socket.as_raw_socket() as SOCKET,
                SIO_UDP_CONNRESET,
                &mut disable_connreset as *mut u32 as *mut _,
                size_of::<u32>() as u32,
                null_mut(),
                0,
                &mut bytes_returned,
                null_mut(),
                None,
            )
        };

        if result != 0 {
            return Err(std::io::Error::last_os_error());
        }

        socket.set_nonblocking(true)?;
        UdpSocket::from_std(socket)
    }

    #[cfg(not(windows))]
    {
        UdpSocket::bind((bind_host, port)).await
    }
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
