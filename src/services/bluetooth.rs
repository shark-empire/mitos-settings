//! Bluetooth actions mitos-settings itself needs -- power, discovery,
//! and the device list for `categories::bluetooth`'s `live_info()`. All
//! three now go through `network::client` (mitos-network's control
//! socket) instead of shelling out to `bluetoothctl`: this machine's
//! Bluetooth adapter is mitos-network's to manage, the same reasoning
//! `grants`' own doc comment gives for why that module stopped keeping
//! its own local state once mitos-service existed to ask instead.
//!
//! This is the corrected version of that change: the first pass (before
//! mitos-network's actual source was available, working only from its
//! descriptive integration guide) sent `BluetoothPower`'s field as
//! `powered` -- the real field, confirmed against
//! `mitos-network/src/ipc/messages.rs`, is `on`. `network::client` has
//! that fix now; see its own doc comment for this and the other thing
//! the guide alone didn't make clear (every reply is wrapped in a
//! `ServerMessage::Response`, not sent bare).
//!
//! **Left unconnected, deliberately:** `bluetooth.discoverable` --
//! confirmed against the real `Request` enum, not just absent from the
//! guide's prose: there is no discoverability request anywhere in it
//! (`BluetoothPower`/`BluetoothScan`/`Pair`/`Trust`/`Connect`/
//! `Disconnect`/`Remove`/`ListBluetoothDevices` is the complete list).
//! It still persists and displays normally; it just doesn't do anything
//! live.

use crate::network::client;

#[derive(Debug, Clone)]
pub struct PairedDevice {
    pub mac: String,
    pub name: String,
}

pub fn set_powered(on: bool) -> std::io::Result<()> {
    client::set_bluetooth_power(on)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
}

/// Backs `bluetooth.auto_scan` via `BluetoothScan { on }`. Worth
/// knowing: this sets the daemon's scan state directly on every
/// `apply()` call for that setting, with no tie to whether a Bluetooth
/// settings page happens to be open -- see `network::client::
/// set_bluetooth_scan`'s doc comment.
pub fn set_auto_scan(on: bool) -> std::io::Result<()> {
    client::set_bluetooth_scan(on).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
}

pub fn list_devices() -> Vec<PairedDevice> {
    client::list_bluetooth_devices()
        .map(|devices| {
            devices
                .into_iter()
                .map(|d| PairedDevice {
                    mac: d.mac,
                    name: d.name,
                })
                .collect()
        })
        // Best-effort, like the old bluetoothctl-backed version: a
        // live_info() row is a nice-to-have, not something a failed
        // daemon call should turn into a page that won't render.
        .unwrap_or_default()
}
