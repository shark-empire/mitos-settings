//! A thin, dependency-free client for mitos-power -- the same "talk to
//! the daemon that actually owns this state" shape `grants` (mitos-service)
//! and `network` (mitos-network) already follow. See `client`'s doc
//! comment for the protocol, which is *not* the same shape
//! `network::client` speaks to mitos-network, despite both existing for
//! an analogous reason: mitos-power uses newline-delimited JSON with an
//! explicit `"kind"` tag and string-named methods (closer to JSON-RPC),
//! not the length-prefixed, externally-tagged-enum wire format
//! mitos-network uses. `network::json`'s generic `Json` value/parser is
//! still reused here -- that part genuinely is protocol-agnostic -- only
//! the framing and envelope are different, so only those are
//! reimplemented in `client`.
//!
//! Only two settings had a real, IPC-reachable equivalent once
//! mitos-power's actual source was available: see `services::power`'s
//! doc comment for the full item-by-item accounting, including why
//! several settings that looked like they should map to something
//! (`power.suspend_timeout_minutes`, `power.lid_close_action`,
//! `battery.low_battery_threshold`) turned out to be config-file-only in
//! mitos-power, with no IPC method to set them at all.

pub mod client;
