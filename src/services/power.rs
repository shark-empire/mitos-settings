//! Power actions mitos-settings itself needs, for `categories::power`'s
//! `apply()` wiring and (new) `live_info()`. Checked against
//! mitos-power's actual `src/ipc/messages.rs`/`manager/manager.rs`, not
//! just guessed at -- see `power::client`'s doc comment for the wire
//! protocol (notably different from `network::client`'s: newline-
//! delimited JSON with an explicit method name, not mitos-network's
//! length-prefixed tagged-enum shape) and each function's own doc
//! comment for exactly what it maps to.
//!
//! **Connected:** `power.profile` (`SetProfile`, now going through
//! mitos-power instead of the generic `powerprofilesctl` it used
//! before -- same reasoning `services::bluetooth`'s doc comment gives
//! for dropping `bluetoothctl`: this machine's power profile is
//! mitos-power's to own once it exists, and having two tools manage the
//! same knob risks them fighting each other) and
//! `power.screen_timeout_minutes` (`SetIdleTimeout`, converted to
//! seconds -- see `power::client::set_screen_timeout_secs`'s doc
//! comment for why *this* is the one that maps there despite the
//! generic-sounding method name, and why `power.suspend_timeout_minutes`
//! does not, below).
//!
//! **Still not connected**, confirmed against the real source rather
//! than just absent from a guess:
//!
//! - `power.suspend_timeout_minutes` -- `IdleDetector` has a completely
//!   separate `suspend_after`/`suspend_on_idle` pair from the
//!   `off_after` field `SetIdleTimeout` actually sets (see
//!   `power::client::set_screen_timeout_secs`), and neither has an IPC
//!   setter -- both come from `DisplayConfig`, config-file-only.
//! - `power.suspend_on_battery_low` -- `policy::battery_policy`'s
//!   critical-battery action is driven entirely by `battery.toml`
//!   (`Config.battery.critical_action`), no IPC method touches it.
//! - `power.lid_close_action` -- same shape again: `policy::lid_policy`
//!   reads `LidConfig` (`close_action_ac`/`close_action_battery`/
//!   `open_action`), config-file-only, no `SetLidAction`-style method
//!   exists in `Request`.
//!
//! For all three: mitos-settings writing directly to mitos-power's own
//! config file (rather than calling an IPC method) was considered and
//! rejected here, the same call `services::network` made for
//! `network.firewall_enabled` -- it's a different, more invasive kind of
//! coupling than an IPC call (racing mitos-power's own config
//! read/reload timing, with no method to ask it to pick up the change)
//! and risks silently fighting whatever's already in that file, for
//! settings this crate can't even fully verify the reload semantics of
//! without mitos-power's config-loading source in front of it too.

use crate::power::client;
use std::io;

/// Backs `power.profile`.
pub fn set_profile(name: &str) -> io::Result<()> {
    client::set_profile(name).map_err(|e| io::Error::new(io::ErrorKind::Other, e))
}

/// How long "never" is, in seconds, when this crate's `0 = never` has to
/// be expressed to mitos-power: ten years. Passing `0` through would do
/// the exact opposite -- `DisplayTimeouts::classify` treats
/// `idle_for >= off_after` as "off", every duration is `>= 0`, so a
/// zero timeout means the screen is off *immediately and permanently*
/// (mitos-power's own test, `tick_with_zero_timeouts_immediately_reports_off`,
/// says as much: "0s timeouts means 'instantly idle'"). `SetIdleTimeout`
/// has no separate "disabled" flag, so a very long timeout is the only
/// way to say it.
const NEVER_SECS: u64 = 10 * 365 * 24 * 60 * 60;

/// Backs `power.screen_timeout_minutes`. `0` ("never") becomes
/// `NEVER_SECS`, not `0` -- see that constant.
///
/// Two limits worth knowing, both from mitos-power's own design rather
/// than anything fixable from here:
/// - "never" means the screen never turns *off*. `SetIdleTimeout` only
///   sets `off_after`; the *dim* threshold (`dim_timeout_secs`, from
///   `display.toml`) has no IPC setter, so the screen can still dim
///   after that long.
/// - The value doesn't stick on mitos-power's side: `SetIdleTimeout`
///   isn't persisted there, and every `SetProfile` replaces the whole
///   timeout set with the new profile's own. `services::
///   reapply_dependents` covers the second half (re-pushing this after
///   a profile change); the first (a mitos-power restart) isn't
///   something a one-shot client can detect.
pub fn set_screen_timeout(minutes: i64) -> io::Result<()> {
    let secs = if minutes <= 0 {
        NEVER_SECS
    } else {
        minutes as u64 * 60
    };
    client::set_screen_timeout_secs(secs).map_err(|e| io::Error::new(io::ErrorKind::Other, e))
}

/// Backs `power`'s (new) `live_info()`. `GetPowerState` is `PUBLIC`
/// tier, so this doesn't need any of the session-vs-privileged
/// reasoning the two `set_*` functions above do. Returns nothing (not
/// an error string) when mitos-power isn't reachable -- this is a
/// nice-to-have summary line, not something that should make a
/// settings page render an error over.
pub fn live_status() -> Vec<(String, String)> {
    let Ok(state) = client::get_power_state() else {
        return Vec::new();
    };

    let power_source = if state.on_ac { "AC power" } else { "battery" };
    let mut line = format!("{power_source}, {} profile, {} thermal", state.profile, state.thermal_level);
    if let Some(pct) = state.display_brightness_percent {
        line.push_str(&format!(", display {pct}%"));
    }
    vec![("status".to_string(), line)]
}
