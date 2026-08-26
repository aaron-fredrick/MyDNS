//! Cross-platform privilege helpers.
//!
//! MyDNS must bind to port 53, which requires Administrator rights on Windows
//! and root (UID 0) on Linux/macOS. This module provides:
//! - [`check_and_exit_if_insufficient`] – called once at startup.
//! - [`drop_privileges`] – called on Unix after the socket is bound.

// `drop_privileges` and `drop_privileges_impl` are compiled out on Windows
// (they are called only inside a `#[cfg(unix)]` block in main.rs).
#[allow(dead_code)]
#[allow(non_snake_case)]
pub fn checkAndExitIfInsufficient(dns_port: u16, http_port: u16) {
    if (dns_port < 1024 || http_port < 1024) && !isRunningElevated() {
        eprintln!(
            "[CRITICAL] MyDNS requires elevated privileges to bind to privileged ports (DNS: {}, HTTP: {}).\n\
             {}",
            dns_port,
            http_port,
            elevationHint()
        );
        std::process::exit(1);
    }
}

/// Drops to a non-privileged user after the DNS socket has been bound.
///
/// On Unix this is required for a production deployment when running as root:
/// failure to drop privileges is returned to the caller so startup fails closed.
#[allow(dead_code)]
#[allow(non_snake_case)]
pub fn dropPrivileges(user: &str, group: &str) -> anyhow::Result<()> {
    dropPrivilegesImpl(user, group)
}

// ── platform implementations ──────────────────────────────────────────────────

#[allow(non_snake_case)]
#[cfg(windows)]
fn isRunningElevated() -> bool {
    use std::ptr;
    use winapi::um::{
        processthreadsapi::{GetCurrentProcess, OpenProcessToken},
        securitybaseapi::GetTokenInformation,
        winnt::{TokenElevation, HANDLE, TOKEN_ELEVATION, TOKEN_QUERY},
    };

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

#[allow(non_snake_case)]
#[cfg(unix)]
fn isRunningElevated() -> bool {
    nix::unistd::getuid().is_root()
}

#[allow(non_snake_case)]
#[cfg(not(any(windows, unix)))]
fn isRunningElevated() -> bool {
    // Conservative fallback: assume no privileges and let bind fail loudly.
    false
}

#[allow(non_snake_case)]
#[cfg(windows)]
fn elevationHint() -> &'static str {
    "  → Right-click the terminal and choose 'Run as Administrator', then retry."
}

#[allow(non_snake_case)]
#[cfg(unix)]
fn elevationHint() -> &'static str {
    "  → Re-run with: sudo ./mydns\n\
     \n\
     Alternatively, grant the binary CAP_NET_BIND_SERVICE:\n\
     \n\
       sudo setcap cap_net_bind_service=ep ./target/release/mydns"
}

#[allow(non_snake_case)]
#[cfg(not(any(windows, unix)))]
fn elevationHint() -> &'static str {
    "  → Run with sufficient OS privileges to bind port 53."
}

// ── privilege dropping ────────────────────────────────────────────────────────

#[allow(non_snake_case)]
#[cfg(unix)]
fn dropPrivilegesImpl(user_name: &str, group_name: &str) -> anyhow::Result<()> {
    use nix::unistd::{setgroups, setresgid, setresuid, Group, User};

    let group = Group::from_name(group_name)
        .map_err(|e| anyhow::anyhow!("Error looking up Unix group '{}': {}", group_name, e))?
        .ok_or_else(|| anyhow::anyhow!("Required Unix group '{}' was not found", group_name))?;

    let user = User::from_name(user_name)
        .map_err(|e| anyhow::anyhow!("Error looking up Unix user '{}': {}", user_name, e))?
        .ok_or_else(|| anyhow::anyhow!("Required Unix user '{}' was not found", user_name))?;

    // Drop supplemental groups
    setgroups(&[group.gid])
        .map_err(|e| anyhow::anyhow!("Failed to drop supplemental groups: {}", e))?;

    // Drop GID first (because setuid might strip capability to change GID later)
    setresgid(group.gid, group.gid, group.gid)
        .map_err(|e| anyhow::anyhow!("Failed to drop group privileges to {}: {}", group_name, e))?;

    // Drop UID
    setresuid(user.uid, user.uid, user.uid)
        .map_err(|e| anyhow::anyhow!("Failed to drop user privileges to {}: {}", user_name, e))?;

    tracing::info!(uid = %user.uid, gid = %group.gid, "Dropped privileges to {}:{}", user_name, group_name);
    Ok(())
}

#[allow(non_snake_case)]
#[cfg(not(unix))]
#[allow(dead_code)]
fn dropPrivilegesImpl(_user: &str, _group: &str) -> anyhow::Result<()> {
    tracing::warn!("Privilege dropping is not supported on this platform");
    Ok(())
}
