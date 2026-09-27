//! A thin, dependency-free client for mitos-network, mitos-settings'
//! network/Wi-Fi/Bluetooth backend -- the same "talk to the daemon that
//! actually owns this state" shape `grants` already follows for
//! mitos-service. See `client`'s doc comment for the protocol and,
//! importantly, exactly how much of it this crate actually speaks: only
//! `services::bluetooth`'s two calls today (adapter power, device list)
//! have enough evidence behind their exact wire shape to implement with
//! any confidence -- see `services::network`'s and
//! `services::bluetooth`'s own doc comments for the settings left
//! unconnected, and why. `docs/network-integration.md` holds the full
//! integration guide this module was built from.

pub mod client;
pub mod json;
