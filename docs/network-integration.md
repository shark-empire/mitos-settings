<!--
Copied here from mitos-network's own repo (originally
"How to connnect network to gui and settings.txt") so the reasoning in
`src/network/`, `services::network`, and `services::bluetooth` has
something in this repo to point at. This file's own relative links
("INTEGRATION.md", "docs/ipc.md", "src/ipc/messages.rs") refer to
mitos-network's repo, not this one -- they're left as-is rather than
rewritten, since resolving them means going and looking at that project,
same as reading it in place there would.

UPDATE: `mitos-network-reference/` (sibling to this file) now holds the
real `messages.rs`/`protocol.rs` this guide was describing secondhand.
Two things this guide's prose left ambiguous turned out to matter once
the real source was available: `BluetoothPower`'s field is `on`, not a
guessable name like `powered`, and every reply is wrapped in a
`ServerMessage::Response(..)` envelope the guide never mentions. Prefer
`mitos-network-reference/` for exact field names; keep reading this file
for the narrative "how to build a panel" context the raw schema doesn't
carry on its own.
-->

# Connecting a GUI or Settings app to mitos-network

This goes one level deeper than [`INTEGRATION.md`](../INTEGRATION.md) and
[`docs/ipc.md`](ipc.md): those cover the transport and protocol in general,
this is specifically about building the thing INTEGRATION.md calls out as
not yet built -- a network settings panel, status applet, or first-run
Wi-Fi picker. Everything here is grounded in the actual `Request`/`Response`/
`Event` definitions in `src/ipc/messages.rs` and how `src/manager/manager.rs`
handles each one; nothing below is aspirational.

## The short version

A settings GUI is an IPC client like `mitos-netctl`, just long-running and
reactive instead of one-shot. Concretely:

1. Open **two** connections to the socket at `general.socket-path`
   (default `/run/mitos-network/network.sock`): one that only ever calls
   `Client::next_event()` in a loop to drive live UI updates, and one that
   only ever calls `Client::request()` for user-initiated actions. This is
   INTEGRATION.md's own recommendation -- repeated here because it matters
   more for a settings app than for anything else that talks to this
   daemon.
2. Issue requests from a background thread, never the UI thread. Several
   requests block the daemon's single manager thread for real, observable
   time (`ScanWifi` up to ~8s, `ConnectWifi` for however long association +
   4-way handshake takes) -- see "Rough edges" below.
3. Render initial state from a handful of `Get*`/`List*` requests at
   startup, then keep it current from the `Event` stream rather than
   polling -- except for the two areas that don't emit events yet
   (Bluetooth, proxy), also covered below.

## Three ways to connect

- **Rust** (the expected path for `mitos-gui`): add `mitos-network` as a
  path dependency and use `ipc::client::Client` directly. See
  INTEGRATION.md section 1 for the exact snippet.
- **Any other language/toolkit** (a GTK+Python panel, a Qt/C++ applet, an
  Electron settings window): speak the wire protocol directly -- a 4-byte
  little-endian length prefix plus that many bytes of JSON, documented in
  full in `docs/ipc.md`. `Request`/`Response`/`Event` all serialize as
  single-key JSON objects (serde's default enum representation), e.g.
  `{"ConnectWifi":{"device":"wlan0","ssid":"Home","security":"Wpa2Psk",
  "passphrase":"..."}}`. No code generation or shared schema needed --
  `src/ipc/messages.rs` is the schema, read directly.
- **Shell out to `mitos-netctl`**: fine for a single fire-and-forget
  action (a right-click "Forget this network" menu item), wrong for
  anything that needs to stay current -- you'd be polling a subprocess
  instead of reading a socket, with none of the event stream's benefit.

## Permissions: what needs elevation

Every request maps to a `security::policy::Capability`, checked against
the connecting process's `uid`/`gid` (resolved via `SO_PEERCRED`, not
anything the client asserts about itself):

| Capability | Who's allowed | Requests |
|---|---|---|
| `ViewState` | anyone | every `Get*`/`List*` query, `GetConnectivity`, `GetProxyConfig`, `ResolveProxy`, `Diagnose` |
| `ManageConnections`, `ManageWifi`, `ManageFirewall`, `ManageHotspot`, `ManageBluetooth`, `ManageProxy` | root or `netdev` group | `Add`/`Delete`/`Activate`/`DeactivateConnection`, `ScanWifi`/`ConnectWifi`/`ForgetWifi`, firewall zone/rule changes, hotspot start/stop, all `Bluetooth*` actions, `SetProxyConfig` |
| `Admin` | root only | `Reload` |

For UI purposes, that means: read-only screens (status overview, device
list, "what network am I on") never need a permission story at all. Any
screen with a mutating control (Wi-Fi connect button, VPN toggle, proxy
save, hotspot switch) is gated the same way every mainstream desktop
network manager already gates these -- a logged-in session in `netdev`
just works, anyone else gets `Response::Error(...)`.

**Don't parse that error string to detect "permission denied" specifically.**
Right now every failure -- wrong Wi-Fi password, a device that doesn't
exist, insufficient permission, a timeout talking to wpa_supplicant --
arrives as the same `Response::Error(String)` shape (currently something
like `"uid 1000 lacks ManageWifi"` for the permission case specifically,
straight from `Debug`-formatting a `Capability`). That's a reasonable
message for `mitos-netctl`'s stderr, not something to string-match for UI
branching, and the exact wording isn't a stable contract. If your UI wants
to *pre-emptively* gray out a control rather than let the user hit the
error, check group membership through your own platform's APIs (e.g. is
the current user in `netdev`) rather than asking the daemon "would this be
allowed" -- there's no such query today. Treating this as a real gap:
distinguishing "permission" from "not found" from "genuinely failed" with a
structured error (an error *kind* alongside the message, or a dedicated
`Response::PermissionDenied` variant) would be a good, small follow-up if
you find yourself wanting it.

## Screen-by-screen: mapping a settings UI to the API

**Overview / status page**
`GetState` and `ListDevices` for the initial render; after that, update
from `StateChanged` and `DeviceStateChanged` events rather than re-polling.
`GetConnectivity` / `ConnectivityChanged` for the "online / limited /
captive portal" indicator -- on `ConnectivityState::Portal`, this daemon
deliberately does *not* try to open a browser itself (see
`docs/networking.md`); that action belongs to the GUI.

**Wi-Fi panel**
`ScanWifi { device }` triggers a scan and returns the results in the same
response (it blocks until wpa_supplicant reports the scan complete, or a
~8s safety timeout) -- show a spinner/skeleton list for that window, don't
treat the wait as an error state. `ListWifiNetworks { device }` returns the
last scan's results instantly, without triggering a new one -- use it for
"switch back to this tab" where a full rescan isn't warranted.
`ConnectWifi { device, ssid, security, passphrase }` for the connect
action (a wrong password surfaces as `Response::Error`, not a distinct
variant -- there's no structured "auth failed" signal to key retry-prompt
UI off of yet). `ForgetWifi { device, ssid }` for "forget this network."
For an Enterprise (802.1X) network, the picker needs to collect identity/
CA cert/client cert path and optionally a pinned EAP method before this is
reachable through IPC at all -- `Request::ConnectWifi` doesn't currently
carry `EapConfig`; that has to go through `AddConnection` with a full
`ConnectionProfile` instead (see below).

**Known networks / saved connections list**
`ListConnections` for the list, `GetConnection { id }` for a detail pane,
`ActivateConnection`/`DeactivateConnection` for a connect/disconnect
toggle, `DeleteConnection` for removal. This list mixes Wi-Fi, wired, and
VPN profiles together (they're all just `ConnectionProfile`s distinguished
by their settings) -- filter/group by which settings variant is populated
if your UI wants separate Wi-Fi/VPN sections rather than one flat list.

**VPN section**
No VPN-specific requests exist -- a VPN is a `ConnectionProfile` like any
other, so `AddConnection` (with `VpnSettings` populated: kind, config path,
credentials via the same secrets flow as Wi-Fi) creates one, and
`Activate`/`DeactivateConnection` connects/disconnects it. For live status
beyond up/down (OpenVPN's real interface name and connection state, e.g.
`CONNECTING`/`RECONNECTING`), `vpn::openvpn::status(ifname)` exists in the
library but **isn't wired through IPC yet** -- today a GUI can only see
"active or not" via `GetState`/`GetConnection`, not OpenVPN's own
management-interface state string. Exposing that as a `Request`/`Response`
pair (or folding it into `GetConnection`'s response) is a natural, fairly
small next step if a VPN status indicator needs it.

**Proxy settings pane**
`GetProxyConfig` on load, `SetProxyConfig { config }` on save (`config.mode`
is `None`/`Manual`/`Auto`; `Manual` wants `http`/`https`/`ftp`/`socks`/
`no_proxy`, `Auto` wants `pac_url`). For a "test this URL" affordance --
genuinely worth having, since PAC scripts are easy to get subtly wrong --
`ResolveProxy { url }` runs the *actual* currently-active config (evaluating
the real PAC script for `Auto` mode) and returns the directive string a
client would get, e.g. `"PROXY proxy.example.com:8080; DIRECT"` or
`"DIRECT"`. There's no `ProxyConfigChanged` event (see below): after a
successful `SetProxyConfig`, update your own displayed state from the
config you just sent rather than waiting for a broadcast that won't come.

**Hotspot toggle**
`StartHotspot { device, ssid, passphrase, uplink }` / `StopHotspot { device }`.

**Firewall / "advanced" section**
`SetFirewallZone { interface, zone }` for the per-interface trust level,
`AddFirewallRule`/`RemoveFirewallRule` for custom rules. This is the one
area where a wrong setting has real security consequences -- consider a
confirmation step in the UI even though the daemon doesn't require one.

**Bluetooth panel**
`ListBluetoothDevices` for the list; `BluetoothPower`/`BluetoothScan`/
`PairBluetooth`/`TrustBluetooth`/`ConnectBluetooth`/`DisconnectBluetooth`/
`RemoveBluetooth` for the actions, each keyed by MAC address. Like proxy,
**there's no live Bluetooth event** -- `ListBluetoothDevices` doesn't
update in the background, so a panel that's currently open should re-poll
it after every action you take (and probably on a short interval while
visible, the way most Bluetooth settings panels already do, given
discovery/pairing state changes asynchronously on the other end too).

**Diagnostics / "why isn't this working" page**
`Diagnose` returns a `DiagnosticReport` -- reasonable to put behind an
explicit button/expander rather than running it proactively, since it's
one of the heavier requests (it inspects live state across every
subsystem, not just returning cached daemon state).

## Live updates: the `Event` stream

Every open connection receives every broadcast event -- there's no
subscribe/filter step. The full enum:

| Event | Fires when | What to update |
|---|---|---|
| `StateChanged(NetworkState)` | overall daemon state changes | overview page |
| `DeviceAdded(NetworkDevice)` / `DeviceRemoved(String)` | an interface appears/disappears (USB NIC plugged in, etc.) | device list |
| `DeviceStateChanged { device, state }` | a specific device's link/connection state changes | that device's row/detail |
| `ConnectionActivated(String)` / `ConnectionDeactivated(String)` | a profile (Wi-Fi, wired, or VPN) connects/disconnects | connections list, VPN toggle state |
| `ConnectivityChanged(ConnectivityState)` | internet reachability / captive-portal status changes | the online/offline/portal indicator |

**Gaps worth knowing about, not working around silently:** proxy config
changes and Bluetooth device/state changes don't broadcast anything.
`SetProxyConfig`/Bluetooth actions succeeding is only visible via that
request's own `Response`, or a subsequent `Get`/`List` call. If you're
extending this daemon rather than just consuming it, adding
`ProxyConfigChanged`/`BluetoothDeviceChanged` variants and the two
`EventBus::broadcast` calls to go with them would close this cleanly --
the `EventBus` itself is already generic over any `Event` variant.

## A minimal worked example (Rust)

```rust
use mitos_network::config;
use mitos_network::ipc::client::Client;
use mitos_network::ipc::messages::{Event, Request, Response};
use std::sync::mpsc;

// Connection 1: events only, feeding UI updates.
fn spawn_event_listener(on_event: mpsc::Sender<Event>) {
    std::thread::spawn(move || {
        let mut client = match Client::connect(config::defaults_socket_path()) {
            Ok(c) => c,
            Err(e) => { eprintln!("daemon not reachable yet: {e}"); return; }
        };
        loop {
            match client.next_event() {
                Ok(ev) => { if on_event.send(ev).is_err() { return; } }
                Err(_) => return, // connection dropped; a real app would reconnect with backoff
            }
        }
    });
}

// Connection 2: request/response, driven by user actions -- called from
// a background thread/task, never the UI thread, since e.g. ScanWifi can
// take several seconds.
fn scan_wifi(client: &mut Client, device: &str) -> Result<Vec<mitos_network::wifi::WifiNetwork>, String> {
    match client.request(Request::ScanWifi { device: device.to_string() }) {
        Ok(Response::WifiNetworks(nets)) => Ok(nets),
        Ok(Response::Error(e)) => Err(e),
        Ok(_) => Err("unexpected response".into()),
        Err(e) => Err(e.to_string()),
    }
}
```

## Rough edges to design around

- **Requests can block for real time.** The whole daemon is one manager
  thread processing one `Command` at a time; `ScanWifi`/`ConnectWifi` in
  particular can take seconds. A slow request from any client delays
  request handling *and* event delivery for every other connection too
  (events are broadcast from the same thread) -- there's no per-connection
  isolation. Design for "the daemon can be briefly unresponsive," not "every
  request is instant."
- **Errors are one unstructured string**, covering permission denial,
  not-found, protocol-level failures, and genuine runtime errors alike --
  see the permissions section above.
- **No live events for proxy or Bluetooth state** -- poll after actions in
  those two areas specifically; everything else has a real event.
- **`ConnectWifi` has no path for Enterprise (802.1X) networks** -- that
  needs a full `ConnectionProfile` via `AddConnection` instead, so an
  Enterprise connect dialog is a heavier flow than a PSK one.
- **Nothing here has been compiled** (see the top-level README) -- treat
  exact request/response field names as very likely correct (they're
  copied straight from `src/ipc/messages.rs`) but worth a final diff
  against that file once this crate actually builds.
