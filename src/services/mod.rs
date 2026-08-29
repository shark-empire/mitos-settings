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
pub mod bluetooth;
pub mod display;
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
        "display.brightness" => value.as_int().map(|v| display::set_brightness(v.clamp(0, 100) as u8)),
        "display.night_light" => value.as_bool().map(display::set_night_light),
        "sound.volume" => value.as_int().map(|v| audio::set_volume(v.clamp(0, 100) as u8)),
        "sound.output_muted" => value.as_bool().map(audio::set_muted),
        "network.wifi_enabled" => value.as_bool().map(network::set_wifi_enabled),
        "bluetooth.enabled" => value.as_bool().map(bluetooth::set_powered),
        "power.profile" => value.as_str().map(power::set_profile),
        "date_time.timezone" => value.as_str().map(time::set_timezone),
        "date_time.automatic_time" => value.as_bool().map(time::set_ntp_enabled),
        "language.system_language" => value.as_str().map(locale::set_language),
        _ => None,
    };

    if let Some(Err(err)) = result {
        eprintln!("mitos-settings: '{key}' was saved, but applying it to the running system failed: {err}");
    }
}
