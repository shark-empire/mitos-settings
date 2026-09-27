//! A one-shot, dependency-free client for mitos-network's control
//! socket (`/run/mitos-network/network.sock`) -- see
//! `docs/network-integration.md` for the general shape, and
//! mitos-network's own `src/ipc/{messages,protocol}.rs` (copied
//! alongside it as `docs/network-messages.rs` for reference) for the
//! actual schema every call below is now checked against directly,
//! rather than inferred from prose.
//!
//! Two things the descriptive guide didn't make clear, discovered only
//! by reading the real source, and worth flagging since they'd both be
//! silent, confusing failures otherwise:
//!
//! 1. **Every reply is wrapped.** The wire-level reply to a `Request`
//!    isn't a bare `Response` -- it's a `ServerMessage::Response(..)`,
//!    because the same connection can *also* receive an unsolicited
//!    `ServerMessage::Event(..)` at any point once it's sent one
//!    request (mitos-network has no separate `SUBSCRIBE` step; event
//!    delivery is implicit for as long as the socket's open). `request`
//!    below unwraps this and skips past any `Event` it happens to read
//!    before the real `Response` arrives.
//! 2. **`BluetoothPower`'s field is `on`, not `powered`.** The
//!    integration guide never showed this request's fields, so this
//!    crate's first pass guessed `powered` (a reasonable-sounding name
//!    that was simply wrong). Fixed below now that the real definition
//!    is available -- a concrete example of exactly the kind of mistake
//!    that guide's own closing note warned this crate hadn't ruled out.
//!
//! This still deliberately doesn't follow the integration guide's
//! recommended shape for a full network panel (two long-lived
//! connections: one draining `Event`s in a loop, one for on-demand
//! requests). This crate applies a setting imperatively the moment it's
//! set (see `services::apply`) and computes `live_info()` fresh every
//! time a category page is built -- nothing here needs to *react* to a
//! change it didn't itself make, so there's still no use for the event
//! stream. One-shot connect/send/read/close instead, the same pattern
//! `grants::service_client` already uses for mitos-service.

use crate::network::json::{self, Json};
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::time::Duration;

fn socket_path() -> PathBuf {
    PathBuf::from("/run/mitos-network/network.sock")
}

const TIMEOUT: Duration = Duration::from_secs(10);

fn connect(timeout: Duration) -> Result<UnixStream, String> {
    let stream = UnixStream::connect(socket_path())
        .map_err(|e| format!("could not reach mitos-network: {e}"))?;
    let _ = stream.set_read_timeout(Some(timeout));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(10)));
    Ok(stream)
}

/// Sends `req`, unwraps the matching `ServerMessage::Response`
/// (skipping past any `ServerMessage::Event` read first -- see this
/// file's top doc comment), and surfaces a `Response::Error(String)` as
/// `Err`. Anything else is returned as the raw `Response`-shaped `Json`
/// for the caller to pull fields out of.
fn request(req: &Json, timeout: Duration) -> Result<Json, String> {
    let mut stream = connect(timeout)?;

    let body = req.encode();
    let len =
        u32::try_from(body.len()).map_err(|_| "request body too large to frame".to_string())?;
    stream
        .write_all(&len.to_le_bytes())
        .map_err(|e| format!("could not send to mitos-network: {e}"))?;
    stream
        .write_all(body.as_bytes())
        .map_err(|e| format!("could not send to mitos-network: {e}"))?;

    // A handful of frames, not an unbounded loop: this connection was
    // just opened for this one request, so an Event flood here would
    // mean something is very wrong -- better to give up with a clear
    // error than hang reading forever.
    for _ in 0..8 {
        let mut len_bytes = [0u8; 4];
        stream
            .read_exact(&mut len_bytes)
            .map_err(|e| format!("could not read from mitos-network: {e}"))?;
        let frame_len = u32::from_le_bytes(len_bytes) as usize;

        let mut frame_bytes = vec![0u8; frame_len];
        stream
            .read_exact(&mut frame_bytes)
            .map_err(|e| format!("could not read from mitos-network: {e}"))?;
        let frame_text = String::from_utf8(frame_bytes)
            .map_err(|_| "mitos-network sent a non-UTF-8 message".to_string())?;
        let message = json::parse(&frame_text)?;

        match message.as_single_variant() {
            Some(("Response", response)) => {
                if let Some(("Error", err_msg)) = response.as_single_variant() {
                    return Err(err_msg.as_str().unwrap_or("(no message)").to_string());
                }
                return Ok(response.clone());
            }
            Some(("Event", _)) => continue, // not our reply -- keep reading
            _ => return Err(format!("unexpected message from mitos-network: {frame_text}")),
        }
    }

    Err("mitos-network kept sending Events without ever replying".to_string())
}

/// `BluetoothPower { on: bool }`.
pub fn set_bluetooth_power(on: bool) -> Result<(), String> {
    let req = Json::variant("BluetoothPower", Json::object(vec![("on", Json::Bool(on))]));
    request(&req, TIMEOUT)?;
    Ok(())
}

/// `BluetoothScan { on: bool }` -- backs `bluetooth.auto_scan`. Worth
/// knowing: this sets the daemon's scan state directly and
/// unconditionally, on every `apply()` call for that setting -- there's
/// no notion of "while the Settings window is open" here (this crate
/// has no page-visibility hook to tie that to), so turning the setting
/// on genuinely leaves discovery running continuously, not just while a
/// Bluetooth page happens to be on screen, until something turns it
/// back off.
pub fn set_bluetooth_scan(on: bool) -> Result<(), String> {
    let req = Json::variant("BluetoothScan", Json::object(vec![("on", Json::Bool(on))]));
    request(&req, TIMEOUT)?;
    Ok(())
}

#[derive(Debug, Clone, PartialEq)]
pub struct BluetoothDevice {
    pub mac: String,
    pub name: String,
    pub paired: bool,
    pub connected: bool,
}

/// `ListBluetoothDevices` -> `Response::BluetoothDevices(Vec<BluetoothDevice>)`.
pub fn list_bluetooth_devices() -> Result<Vec<BluetoothDevice>, String> {
    let req = Json::unit_variant("ListBluetoothDevices");
    let response = request(&req, TIMEOUT)?;

    let Some(("BluetoothDevices", items)) = response.as_single_variant() else {
        return Err(format!(
            "expected a BluetoothDevices response, got: {}",
            response.encode()
        ));
    };
    let Some(items) = items.as_array() else {
        return Err("BluetoothDevices payload wasn't an array".to_string());
    };

    Ok(items
        .iter()
        .filter_map(|item| {
            Some(BluetoothDevice {
                mac: item.get("mac").and_then(Json::as_str)?.to_string(),
                name: item.get("name").and_then(Json::as_str).unwrap_or("").to_string(),
                paired: item.get("paired").and_then(Json::as_bool).unwrap_or(false),
                connected: item
                    .get("connected")
                    .and_then(Json::as_bool)
                    .unwrap_or(false),
            })
        })
        .collect())
}

/// `GetProxyConfig` -> `Response::ProxyConfig(Option<ProxyConfig>)`.
/// Returns the raw config object (all of `ProxyConfig`'s fields, not
/// just `mode`) so a caller that wants to change one field can send the
/// rest back unmodified -- see `set_proxy_mode`, which is the reason
/// this returns the whole `Json` object rather than a narrower type.
/// `Ok(None)` means no config has ever been set (mitos-network's own
/// `Option<ProxyConfig>`), in which case `ProxyConfig::default()`'s
/// shape (`mode: "none"`, everything else empty) is what a fresh
/// `SetProxyConfig` should start from.
fn get_proxy_config() -> Result<Option<Json>, String> {
    let req = Json::unit_variant("GetProxyConfig");
    let response = request(&req, TIMEOUT)?;

    let Some(("ProxyConfig", config)) = response.as_single_variant() else {
        return Err(format!(
            "expected a ProxyConfig response, got: {}",
            response.encode()
        ));
    };
    match config {
        Json::Null => Ok(None),
        other => Ok(Some(other.clone())),
    }
}

/// Sets just the proxy *mode*, preserving whatever `http`/`https`/`ftp`/
/// `socks`/`no_proxy`/`pac_url` mitos-network already has -- this
/// crate's own `network.proxy_mode` setting has no fields for those
/// (see `categories::network`), and `SetProxyConfig` replaces the whole
/// config rather than merging it, so sending a mode-only object would
/// silently wipe them. Read-modify-write instead: fetch the current
/// config, change only `mode`, send the rest back untouched. Maps this
/// crate's `"automatic"` choice to mitos-network's `"auto"` --
/// `ProxyMode` is `#[serde(rename_all = "lowercase")]` there, so the
/// wire values are `"none"`/`"manual"`/`"auto"`, not the `None`/
/// `Manual`/`Auto` this crate's first pass (working from the
/// descriptive guide alone, before this project's source was
/// available) had assumed.
pub fn set_proxy_mode(mode: &str) -> Result<(), String> {
    let wire_mode = match mode {
        "none" => "none",
        "manual" => "manual",
        "automatic" => "auto",
        other => return Err(format!("unknown proxy mode '{other}'")),
    };

    let mut fields: Vec<(String, Json)> = match get_proxy_config()? {
        Some(Json::Object(existing_fields)) => existing_fields,
        _ => vec![
            ("http".to_string(), Json::Null),
            ("https".to_string(), Json::Null),
            ("ftp".to_string(), Json::Null),
            ("socks".to_string(), Json::Null),
            ("no_proxy".to_string(), Json::Array(Vec::new())),
            ("pac_url".to_string(), Json::Null),
        ],
    };
    match fields.iter_mut().find(|(k, _)| k.as_str() == "mode") {
        Some((_, v)) => *v = Json::string(wire_mode),
        None => fields.insert(0, ("mode".to_string(), Json::string(wire_mode))),
    }

    let req = Json::variant("SetProxyConfig", Json::variant("config", Json::Object(fields)));
    // NOTE: `SetProxyConfig { config: ProxyConfig }` is a struct
    // variant with one field named `config` -- so the request needs an
    // *extra* layer of nesting versus most other calls in this file
    // (`{"SetProxyConfig":{"config":{...the ProxyConfig object...}}}`),
    // which `Json::variant("SetProxyConfig", Json::variant("config", ..))`
    // builds correctly: the outer `variant` wraps the whole request,
    // the inner one supplies that single named field.
    request(&req, TIMEOUT)?;
    Ok(())
}

#[derive(Debug, Clone, PartialEq)]
pub struct Device {
    pub name: String,
    pub device_type: String,
    pub state: String,
    pub carrier: bool,
    pub ipv4_addresses: Vec<String>,
    pub active_connection: Option<String>,
}

/// `ListDevices` -> `Response::Devices(Vec<NetworkDevice>)`. `device_type`/
/// `state` are kept as their raw wire strings (`"Ethernet"`/`"WiFi"`/...,
/// `"Activated"`/`"Disconnected"`/...) rather than parsed into an enum
/// here -- `NetworkDevice`'s `DeviceType`/`DeviceState` have no
/// `#[serde(rename_all)]` in mitos-network's source, so these are the
/// exact, real wire values (serde's default: the Rust variant name
/// as-written), not a guess.
pub fn list_devices() -> Result<Vec<Device>, String> {
    let req = Json::unit_variant("ListDevices");
    let response = request(&req, TIMEOUT)?;

    let Some(("Devices", items)) = response.as_single_variant() else {
        return Err(format!(
            "expected a Devices response, got: {}",
            response.encode()
        ));
    };
    let Some(items) = items.as_array() else {
        return Err("Devices payload wasn't an array".to_string());
    };

    Ok(items
        .iter()
        .filter_map(|item| {
            Some(Device {
                name: item.get("name").and_then(Json::as_str)?.to_string(),
                device_type: item
                    .get("device_type")
                    .and_then(Json::as_str)
                    .unwrap_or("Unknown")
                    .to_string(),
                state: item
                    .get("state")
                    .and_then(Json::as_str)
                    .unwrap_or("Unavailable")
                    .to_string(),
                carrier: item.get("carrier").and_then(Json::as_bool).unwrap_or(false),
                ipv4_addresses: item
                    .get("ipv4_addresses")
                    .and_then(Json::as_array)
                    .map(|arr| arr.iter().filter_map(Json::as_str).map(String::from).collect())
                    .unwrap_or_default(),
                active_connection: item
                    .get("active_connection")
                    .and_then(Json::as_str)
                    .map(String::from),
            })
        })
        .collect())
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConnectionSummary {
    pub id: String,
    pub name: String,
    pub device_type: String,
}

/// `ListConnections` -> `Response::Connections(Vec<ConnectionProfile>)`,
/// trimmed to the three fields this crate actually needs (a full
/// `ConnectionProfile` also carries wifi/vpn secrets-adjacent settings
/// this crate has no use for and no schema fields to show).
pub fn list_connections() -> Result<Vec<ConnectionSummary>, String> {
    let req = Json::unit_variant("ListConnections");
    let response = request(&req, TIMEOUT)?;

    let Some(("Connections", items)) = response.as_single_variant() else {
        return Err(format!(
            "expected a Connections response, got: {}",
            response.encode()
        ));
    };
    let Some(items) = items.as_array() else {
        return Err("Connections payload wasn't an array".to_string());
    };

    Ok(items
        .iter()
        .filter_map(|item| {
            Some(ConnectionSummary {
                id: item.get("id").and_then(Json::as_str)?.to_string(),
                name: item.get("name").and_then(Json::as_str)?.to_string(),
                device_type: item
                    .get("device_type")
                    .and_then(Json::as_str)
                    .unwrap_or("Unknown")
                    .to_string(),
            })
        })
        .collect())
}

/// The active VPN connection's display `name`, if any -- resolved by
/// cross-referencing `ListConnections` (id -> name, filtered to
/// `device_type == "Vpn"`) against `ListDevices`' `active_connection`
/// (which device, if any, is currently using that id). `ConnectionProfile`
/// itself carries no "is this one active" field -- `active_connection`
/// on the *device* is the only place that's recorded, per
/// mitos-network's own source.
pub fn active_vpn_name() -> Result<Option<String>, String> {
    let connections = list_connections()?;
    let devices = list_devices()?;

    let active_ids: std::collections::HashSet<&str> = devices
        .iter()
        .filter_map(|d| d.active_connection.as_deref())
        .collect();

    Ok(connections
        .into_iter()
        .find(|c| c.device_type == "Vpn" && active_ids.contains(c.id.as_str()))
        .map(|c| c.name))
}

/// Activates the VPN connection profile named `name` (resolving name ->
/// `id` via `ListConnections` first, since `ActivateConnection` itself
/// only takes an `id`) -- backs `network.vpn_active_profile` being set
/// to a non-empty value. An empty `name` deactivates whichever VPN
/// profile is currently active instead (see `active_vpn_name`), mapping
/// this crate's single "one name field, empty means none" setting onto
/// mitos-network's separate activate/deactivate-by-id calls.
pub fn set_active_vpn(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return match active_vpn_name()? {
            None => Ok(()), // nothing active -- setting it to "none" is already true
            Some(_) => {
                let connections = list_connections()?;
                let devices = list_devices()?;
                let active_ids: std::collections::HashSet<&str> = devices
                    .iter()
                    .filter_map(|d| d.active_connection.as_deref())
                    .collect();
                let Some(active_vpn) = connections
                    .iter()
                    .find(|c| c.device_type == "Vpn" && active_ids.contains(c.id.as_str()))
                else {
                    return Ok(());
                };
                let req = Json::variant(
                    "DeactivateConnection",
                    Json::object(vec![("id", Json::string(active_vpn.id.clone()))]),
                );
                request(&req, TIMEOUT)?;
                Ok(())
            }
        };
    }

    let connections = list_connections()?;
    let Some(profile) = connections
        .iter()
        .find(|c| c.device_type == "Vpn" && c.name == name)
    else {
        return Err(format!("no VPN connection profile named '{name}'"));
    };

    let req = Json::variant(
        "ActivateConnection",
        Json::object(vec![("id", Json::string(profile.id.clone()))]),
    );
    request(&req, TIMEOUT)?;
    Ok(())
}
