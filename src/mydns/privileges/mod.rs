//! Cross-platform privilege helpers.
//!
//! MyDNS must bind to port 53, which requires Administrator rights on Windows
//! and appropriate port-binding privileges on Unix. This module provides:
//! - [`check_and_exit_if_insufficient`] – called once at startup.
//! - [`drop_privileges`] – called on Unix after the socket is bound when MyDNS
//!   was started as root.

// `drop_privileges` and `drop_privileges_impl` are compiled out on Windows
// (they are called only inside a `#[cfg(unix)]` block in dns/server.rs).
#[allow(dead_code)]
pub fn check_and_exit_if_insufficient(dns_port: u16, http_port: u16) {
    if (dns_port < 1024 || http_port < 1024) && !has_required_privileges() {
        eprintln!(
            "[CRITICAL] MyDNS requires permission to bind privileged ports (DNS: {}, HTTP: {}).\n\\
             {}",
            dns_port,
            http_port,
            elevation_hint()
        );
        std::process::exit(1);
    }
}

/// Drops to a non-privileged user after the DNS socket has been bound.
///
/// On Unix this is required for a production deployment when running as root:
/// failure to drop privileges is returned to the caller so startup fails closed.
#[allow(dead_code)]
pub fn drop_privileges(user: &str, group: &str) -> anyhow::Result<()> {
    drop_privileges_impl(user, group)
}

// ── platform implementations ──────────────────────────────────────────────────

#[cfg(windows)]
fn has_required_privileges() -> bool {
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

#[cfg(target_os = "linux")]
fn has_required_privileges() -> bool {
    if nix::unistd::getuid().is_root() {
        return true;
    }

    has_cap_net_bind_service()
}

#[cfg(target_os = "linux")]
fn has_cap_net_bind_service() -> bool {
    const CAP_NET_BIND_SERVICE: u32 = 10;

    let Ok(status) = std::fs::read_to_string("/proc/self/status") else {
        return false;
    };

    let Some(cap_eff) = status
        .lines()
        .find_map(|line| line.strip_prefix("CapEff:\t"))
    else {
        return false;
    };

    let Ok(cap_eff) = u64::from_str_radix(cap_eff.trim(), 16) else {
        return false;
    };

    (cap_eff & (1u64 << CAP_NET_BIND_SERVICE)) != 0
}

#[cfg(all(unix, not(target_os = "linux")))]
fn has_required_privileges() -> bool {
    nix::unistd::getuid().is_root()
}

#[cfg(not(any(windows, unix)))]
fn has_required_privileges() -> bool {
    // Conservative fallback: assume no privileges and let bind fail loudly.
    false
}

#[cfg(windows)]
fn elevation_hint() -> &'static str {
    "  → Right-click the terminal and choose 'Run as Administrator', then retry."
}

#[cfg(unix)]
fn elevation_hint() -> &'static str {
    "  → Re-run with: sudo ./mydns\n\\
     \n\\
     Alternatively, on Linux grant the binary CAP_NET_BIND_SERVICE:\n\\
     \n\\
       sudo setcap cap_net_bind_service=ep ./target/release/mydns"
}

#[cfg(not(any(windows, unix)))]
fn elevation_hint() -> &'static str {
    "  → Run with sufficient OS privileges to bind privileged ports."
}

// ── privilege dropping ────────────────────────────────────────────────────────

#[cfg(unix)]
fn drop_privileges_impl(user_name: &str, group_name: &str) -> anyhow::Result<()> {
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

#[cfg(not(unix))]
#[allow(dead_code)]
fn drop_privileges_impl(_user: &str, _group: &str) -> anyhow::Result<()> {
    tracing::warn!("Privilege dropping is not supported on this platform");
    Ok(())
}

#[cfg(test)]
mod tests {
    #[cfg(target_os = "linux")]
    #[test]
    fn detects_cap_net_bind_service_bit() {
        const CAP_NET_BIND_SERVICE: u32 = 10;
        let cap_eff = 1u64 << CAP_NET_BIND_SERVICE;
        assert_ne!(cap_eff & (1u64 << CAP_NET_BIND_SERVICE), 0);
        assert_eq!(cap_eff & (1u64 << 9), 0);
    }
}
