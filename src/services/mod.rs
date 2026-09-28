//! Mid-level system interaction: reads live state (often via `hardware`)
//! and, unlike `hardware`, can also mutate it — flipping Wi-Fi on, changing
//! volume, setting the timezone, and so on.
//!
//! `apply()` is the single place `settings::manager::SettingsManager` calls
//! into after a value has been validated and persisted, so adding a new
//! "live" setting means: register it in the right `categories::*` file,
//! then add one match arm here.

pub mod accounts;
pub mod audio;
pub mod battery;
pub mod bluetooth;
pub mod dbus;
pub mod display;
pub mod home_conf;
pub mod locale;
pub mod network;
pub mod power;
pub mod storage;
pub mod time;
pub mod updates;

use crate::settings::value::Value;

/// Best-effort: failures are logged, never propagated. The persisted value
/// is the source of truth; live application is an optimistic extra that
/// should never make a `set` call fail just because, say, `amixer` isn't
/// installed in a minimal container.
pub fn apply(key: &str, value: &Value) {
    let result: Option<std::io::Result<()>> = match key {
        "display.brightness" => value
            .as_int()
            .map(|v| display::set_brightness(v.clamp(0, 100) as u8)),
        "display.night_light" => value.as_bool().map(display::set_night_light),
        "sound.volume" => value
            .as_int()
            .map(|v| audio::set_volume(v.clamp(0, 100) as u8)),
        "sound.output_muted" => value.as_bool().map(audio::set_muted),
        "network.wifi_enabled" => value.as_bool().map(network::set_wifi_enabled),
        "network.proxy_mode" => value.as_str().map(network::set_proxy_mode),
        "network.vpn_active_profile" => value.as_str().map(network::set_vpn_active_profile),
        "bluetooth.enabled" => value.as_bool().map(bluetooth::set_powered),
        "bluetooth.auto_scan" => value.as_bool().map(bluetooth::set_auto_scan),
        "power.profile" => value.as_str().map(power::set_profile),
        "power.screen_timeout_minutes" => value.as_int().map(power::set_screen_timeout),
        "date_time.timezone" => value.as_str().map(time::set_timezone),
        "date_time.automatic_time" => value.as_bool().map(time::set_ntp_enabled),
        "language.system_language" => value.as_str().map(locale::set_language),
        _ => None,
    };

    if let Some(Err(err)) = result {
        eprintln!("mitos-settings: '{key}' was saved, but applying it to the running system failed: {err}");
    }
}

/// Settings whose *live* effect gets overwritten by another setting's
/// `apply()`, and so need pushing again right after it -- called by
/// `SettingsManager` straight after `apply` (same spot, and same
/// "consult the manager for current values" shape, as
/// `home_conf::sync_if_relevant`). Currently one pair:
/// `power.profile` -> `power.screen_timeout_minutes`, because
/// mitos-power's `set_profile` replaces the whole idle-timeout set with
/// the new profile's own (`idle::policy::effective_timeouts`), silently
/// discarding whatever `SetIdleTimeout` had set before -- without this,
/// changing the profile would quietly undo the person's chosen screen
/// timeout. Best-effort like `apply`: a failure here is logged, never
/// propagated.
pub fn reapply_dependents(key: &str, manager: &crate::settings::manager::SettingsManager) {
    if key == "power.profile" {
        if let Ok(value) = manager.get("power.screen_timeout_minutes") {
            apply("power.screen_timeout_minutes", value);
        }
    }
}
