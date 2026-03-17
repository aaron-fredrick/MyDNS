/// Cross-platform privilege helpers.
///
/// MyDNS must bind to port 53, which requires Administrator rights on Windows
/// and root (UID 0) on Linux/macOS. This module provides:
/// - [`check_and_exit_if_insufficient`] – called once at startup.
/// - [`drop_privileges`] – called on Unix after the socket is bound.

// `drop_privileges` and `drop_privileges_impl` are compiled out on Windows
// (they are called only inside a `#[cfg(unix)]` block in main.rs).
#[allow(dead_code)]

/// Checks that the process has sufficient OS privileges to bind privileged
/// ports (< 1024). Logs a structured critical error and terminates with exit
/// code 1 when the check fails.
pub fn check_and_exit_if_insufficient(dns_port: u16, http_port: u16) {
    if (dns_port < 1024 || http_port < 1024) && !is_running_elevated() {
        eprintln!(
            "[CRITICAL] MyDNS requires elevated privileges to bind to privileged ports (DNS: {}, HTTP: {}).\n\
             {}",
            dns_port,
            http_port,
            elevation_hint()
        );
        std::process::exit(1);
    }
}

/// Attempts to drop to a non-privileged user after the DNS socket has been
/// bound. This limits the blast radius of any future exploit.
///
/// On Windows, privilege dropping is not implemented; the function logs a
/// warning and returns normally.
#[cfg_attr(windows, allow(dead_code))]
pub fn drop_privileges() -> anyhow::Result<()> {
    drop_privileges_impl()
}

// ── platform implementations ──────────────────────────────────────────────────

#[cfg(windows)]
fn is_running_elevated() -> bool {
    use winapi::um::{
        processthreadsapi::{GetCurrentProcess, OpenProcessToken},
        securitybaseapi::GetTokenInformation,
        winnt::{TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY, HANDLE},
    };
    use std::ptr;

    unsafe {
        let mut token: HANDLE = ptr::null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
            return false;
        }
        let mut elevation = TOKEN_ELEVATION { TokenIsElevated: 0 };
        let mut size = std::mem::size_of::<TOKEN_ELEVATION>() as u32;
        let ok = GetTokenInformation(
            token,
            TokenElevation,
            &mut elevation as *mut _ as *mut _,
            size,
            &mut size,
        );
        ok != 0 && elevation.TokenIsElevated != 0
    }
}

#[cfg(unix)]
fn is_running_elevated() -> bool {
    nix::unistd::getuid().is_root()
}

#[cfg(not(any(windows, unix)))]
fn is_running_elevated() -> bool {
    // Conservative fallback: assume no privileges and let bind fail loudly.
    false
}

#[cfg(windows)]
fn elevation_hint() -> &'static str {
    "  → Right-click the terminal and choose 'Run as Administrator', then retry."
}

#[cfg(unix)]
fn elevation_hint() -> &'static str {
    "  → Re-run with: sudo ./mydns\n\
     \n\
     Alternatively, grant the binary CAP_NET_BIND_SERVICE:\n\
     \n\
       sudo setcap cap_net_bind_service=ep ./target/release/mydns"
}

#[cfg(not(any(windows, unix)))]
fn elevation_hint() -> &'static str {
    "  → Run with sufficient OS privileges to bind port 53."
}

// ── privilege dropping ────────────────────────────────────────────────────────

#[cfg(unix)]
fn drop_privileges_impl() -> anyhow::Result<()> {
    use nix::unistd::{User, setuid};

    match User::from_name("nobody")? {
        Some(user) => {
            setuid(user.uid)?;
            tracing::info!(uid = %user.uid, "Dropped privileges to 'nobody'");
        }
        None => {
            tracing::warn!("User 'nobody' not found — not dropping privileges");
        }
    }
    Ok(())
}

#[cfg(not(unix))]
#[allow(dead_code)]
fn drop_privileges_impl() -> anyhow::Result<()> {
    tracing::warn!("Privilege dropping is not supported on this platform");
    Ok(())
}
