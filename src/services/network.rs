//! Network actions mitos-settings itself needs, for `categories::network`'s
//! `apply()` wiring and `live_info()`. Now checked against
//! mitos-network's actual `src/ipc/messages.rs` (its integration guide
//! alone, used for the first pass at this, turned out to have two real
//! gaps -- see `network::client`'s doc comment -- so this second pass
//! is worth trusting more than the first).
//!
//! **Still not connected**, now confirmed against the real `Request`
//! enum rather than just its absence from a descriptive guide:
//!
//! - `network.wifi_enabled` (`set_wifi_enabled`, below) -- still shells
//!   out to `nmcli radio wifi`. There is no radio on/off request
//!   anywhere in `Request` -- toggling the radio itself isn't a concept
//!   mitos-network's IPC exposes (it may be a kernel-level `rfkill`
//!   concern beneath it instead). Once mitos-network is actually
//!   managing this adapter, `nmcli` assuming NetworkManager owns it too
//!   is a real double-management risk worth fixing, but the fix isn't
//!   "call mitos-network instead" -- there's genuinely nothing to call.
//! - `network.ethernet_enabled` -- same: no such request exists.
//! - `network.dns_servers` -- confirmed per-connection
//!   (`ConnectionProfile.dns: Vec<String>`), not a global daemon
//!   setting `Request` exposes anywhere. A global override has nowhere
//!   to plug into this daemon's model as it stands.
//! - `network.firewall_enabled` -- confirmed per-interface zones
//!   (`firewall::zones::builtin_zones()`: `public`/`home`/`trusted`/
//!   `block`, each with its own `default_policy`), not one global
//!   on/off switch. Even mapping "disabled" to "every interface -> a
//!   permissive zone" would silently overwrite whatever zone
//!   assignments already exist per-interface -- left alone rather than
//!   risking that.
//!
//! **Now connected**, with the real schema available:
//! `network.proxy_mode` (`set_proxy_mode`) and `network.vpn_active_profile`
//! (`set_vpn_active_profile`) via `network::client`, and `live_status`
//! for `live_info()`'s device list, replacing the local
//! `hardware::network::list()` read with mitos-network's own
//! `ListDevices` when it's reachable (falling back to the local read
//! otherwise, the same "best-effort, don't fail the page over it" choice
//! `services::bluetooth::list_devices` already makes).

use crate::hardware;
use crate::network::client;
use std::io;
use std::process::Command;

pub fn list_interfaces() -> Vec<hardware::network::NetIface> {
    hardware::network::list()
}

/// `live_info()`'s device-list feed: mitos-network's own view when
/// reachable (name, kind, and state straight from `ListDevices` --
/// richer than raw sysfs, and authoritative now that mitos-network
/// manages these interfaces), falling back to the local
/// `hardware::network::list()` read if the daemon call fails for any
/// reason (not installed yet, not running, socket permission issue --
/// `live_info()` degrading to less detail beats a settings page that
/// won't render).
pub fn live_status() -> Vec<(String, String)> {
    match client::list_devices() {
        Ok(devices) if !devices.is_empty() => devices
            .into_iter()
            .map(|d| {
                let carrier = if d.carrier { "" } else { ", no carrier" };
                (
                    "interface".to_string(),
                    format!("{} ({}): {}{}", d.name, d.device_type, d.state, carrier),
                )
            })
            .collect(),
        _ => list_interfaces()
            .into_iter()
            .map(|i| {
                (
                    "interface".to_string(),
                    format!("{}: {}", i.name, i.operstate),
                )
            })
            .collect(),
    }
}

pub fn set_wifi_enabled(enabled: bool) -> io::Result<()> {
    let state = if enabled { "on" } else { "off" };
    let status = Command::new("nmcli")
        .args(["radio", "wifi", state])
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::Other,
            "nmcli reported failure (is NetworkManager running?)",
        ))
    }
}

pub fn set_interface_up(name: &str, up: bool) -> io::Result<()> {
    let state = if up { "up" } else { "down" };
    let status = Command::new("ip")
        .args(["link", "set", name, state])
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::Other,
            "ip link set failed (needs root)",
        ))
    }
}

/// Backs `network.proxy_mode`. See `network::client::set_proxy_mode`
/// for why this is a read-modify-write against mitos-network's current
/// `ProxyConfig` rather than a bare `{"mode": ...}` send.
pub fn set_proxy_mode(mode: &str) -> io::Result<()> {
    client::set_proxy_mode(mode).map_err(|e| io::Error::new(io::ErrorKind::Other, e))
}

/// Backs `network.vpn_active_profile`. `name` is this crate's schema
/// value directly (a plain display name, empty for none) --
/// `network::client::set_active_vpn` handles resolving that to
/// mitos-network's `id`-addressed `Activate`/`DeactivateConnection`
/// calls, including "empty name" meaning "deactivate whichever VPN is
/// currently active."
pub fn set_vpn_active_profile(name: &str) -> io::Result<()> {
    client::set_active_vpn(name).map_err(|e| io::Error::new(io::ErrorKind::Other, e))
}
