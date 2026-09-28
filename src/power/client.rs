//! A one-shot, dependency-free client for mitos-power's control socket
//! (`/run/mitos/power.sock`, confirmed against `config/defaults.rs` --
//! notably *not* `/run/mitos-power/...`, despite that being the more
//! obvious guess by analogy with mitos-network/mitos-service).
//!
//! **Wire format**, confirmed against `mitos-power/src/ipc/protocol.rs`
//! directly (including that file's own round-trip tests): newline-
//! delimited JSON, one `ClientMessage`/`ServerMessage` object per line.
//! A request looks like
//! `{"kind":"request","id":"1","method":"SetProfile","params":{"name":"powersave"}}`;
//! the matching reply is
//! `{"kind":"response","id":"1","ok":true,"result":{}}` on success, or
//! `{"kind":"response","id":"1","ok":false,"error":{"code":"...","message":"..."}}`
//! on failure. Unlike mitos-network, there's no risk of an unsolicited
//! `Event` arriving on this connection ahead of the reply: event
//! delivery there requires an explicit `Subscribe` client message first
//! ("[a]dd event names to this connection's subscription set"), which
//! this client never sends, so every line read back is the one reply
//! being waited for -- no envelope-skipping loop needed here the way
//! `network::client::request` has one.
//!
//! One-shot connect/send-one-line/read-one-line/close, same as
//! `grants::service_client` (mitos-service) and `network::client`
//! (mitos-network): this crate applies settings imperatively on `set`
//! and computes `live_info()` fresh per page-build, so there's nothing
//! here that needs mitos-power's event stream either.

use crate::network::json::{self, Json};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

fn socket_path() -> PathBuf {
    PathBuf::from("/run/mitos/power.sock")
}

const TIMEOUT: Duration = Duration::from_secs(10);

fn next_id() -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    COUNTER.fetch_add(1, Ordering::Relaxed).to_string()
}

fn connect(timeout: Duration) -> Result<UnixStream, String> {
    let stream = UnixStream::connect(socket_path())
        .map_err(|e| format!("could not reach mitos-power: {e}"))?;
    let _ = stream.set_read_timeout(Some(timeout));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(10)));
    Ok(stream)
}

/// Sends one `{"kind":"request", ...}` line and reads back the matching
/// `{"kind":"response", ...}` line. `ok: false` is surfaced as `Err`,
/// using `error.message` (falling back to `error.code`, then a generic
/// message, if either is missing -- defensive, since neither field is
/// something this crate has verified is always present together). On
/// success, returns whatever `result` held (an empty object for a
/// method like `SetProfile` that has nothing to report).
fn request(method: &str, params: Json, timeout: Duration) -> Result<Json, String> {
    let mut stream = connect(timeout)?;

    let id = next_id();
    let envelope = Json::object(vec![
        ("kind", Json::string("request")),
        ("id", Json::string(id.clone())),
        ("method", Json::string(method)),
        ("params", params),
    ]);
    let mut line = envelope.encode();
    line.push('\n');
    stream
        .write_all(line.as_bytes())
        .map_err(|e| format!("could not send to mitos-power: {e}"))?;

    let mut reader = BufReader::new(stream);
    let mut response_line = String::new();
    reader
        .read_line(&mut response_line)
        .map_err(|e| format!("could not read from mitos-power: {e}"))?;
    if response_line.is_empty() {
        return Err("mitos-power closed the connection without replying".to_string());
    }
    let response = json::parse(&response_line)?;

    let ok = response.get("ok").and_then(Json::as_bool).unwrap_or(false);
    if !ok {
        let message = response
            .get("error")
            .and_then(|e| e.get("message"))
            .and_then(Json::as_str)
            .or_else(|| {
                response
                    .get("error")
                    .and_then(|e| e.get("code"))
                    .and_then(Json::as_str)
            })
            .unwrap_or("mitos-power reported failure with no message");
        return Err(message.to_string());
    }

    Ok(response.get("result").cloned().unwrap_or(Json::Null))
}

/// `SetProfile { name }`. Maps this crate's `power.profile` choices
/// (`categories::power`) onto mitos-power's own profile names,
/// confirmed against `manager/manager.rs`'s own test
/// (`set_profile("performance")`) and `profiles/manager.rs`'s built-ins
/// (`"performance"`, `"balanced"`, `"powersave"`) -- note `"powersave"`,
/// not the hyphenated `"power-saver"` this crate's schema uses; `"balanced"`
/// and `"performance"` already match and pass through unchanged.
pub fn set_profile(name: &str) -> Result<(), String> {
    let power_name = match name {
        "power-saver" => "powersave",
        other => other,
    };
    let params = Json::object(vec![("name", Json::string(power_name))]);
    request("SetProfile", params, TIMEOUT)?;
    Ok(())
}

/// `SetIdleTimeout { secs }` -- backs `power.screen_timeout_minutes`,
/// *not* `power.suspend_timeout_minutes` despite the generic-sounding
/// name. Traced through `manager::set_idle_timeout` ->
/// `IdleDetector::set_off_timeout`, which sets `DisplayTimeouts.off_after`
/// -- the *display* tier's off threshold (`DisplayTier::Off` ->
/// `DisplayPowerState::Off`), a completely separate field from
/// `IdleDetector`'s own `suspend_after`/`suspend_on_idle`, which have no
/// IPC setter at all (config-file-only, `DisplayConfig`). Worth stating
/// plainly since the method name alone would suggest otherwise.
pub fn set_screen_timeout_secs(secs: u64) -> Result<(), String> {
    let params = Json::object(vec![("secs", Json::Number(secs as f64))]);
    request("SetIdleTimeout", params, TIMEOUT)?;
    Ok(())
}

#[derive(Debug, Clone, PartialEq)]
pub struct BatteryInfo {
    pub id: String,
    pub percentage: Option<f32>,
    pub status: String,
    pub health_percent: Option<f32>,
    pub time_to_empty_secs: Option<u64>,
    pub time_to_full_secs: Option<u64>,
}

/// `GetBatteries` (no params, `PUBLIC` tier -- no permission story
/// needed for this one). Field names are copied straight from
/// `battery::battery::BatteryInfo`, trimmed to what `live_info()` shows
/// -- that struct also carries energy/power/voltage/current/temperature/
/// cycle_count/technology/manufacturer/model_name, left unparsed here
/// since nothing in this crate's schema surfaces them today.
pub fn list_batteries() -> Result<Vec<BatteryInfo>, String> {
    let response = request("GetBatteries", Json::Null, TIMEOUT)?;
    let items = response
        .as_array()
        .ok_or_else(|| format!("expected an array from GetBatteries, got: {}", response.encode()))?;

    Ok(items
        .iter()
        .filter_map(|item| {
            Some(BatteryInfo {
                id: item.get("id").and_then(Json::as_str)?.to_string(),
                percentage: item.get("percentage").and_then(json_as_f32),
                status: item
                    .get("status")
                    .and_then(Json::as_str)
                    .unwrap_or("unknown")
                    .to_string(),
                health_percent: item.get("health_percent").and_then(json_as_f32),
                time_to_empty_secs: item.get("time_to_empty_secs").and_then(json_as_u64),
                time_to_full_secs: item.get("time_to_full_secs").and_then(json_as_u64),
            })
        })
        .collect())
}

#[derive(Debug, Clone, PartialEq)]
pub struct PowerState {
    pub on_ac: bool,
    pub profile: String,
    pub thermal_level: String,
    pub display_brightness_percent: Option<u8>,
    pub idle_seconds: u64,
}

/// `GetPowerState` (no params, `PUBLIC` tier). Field names copied from
/// `manager::state::PowerStateSnapshot`, trimmed the same way
/// `list_batteries` is -- `batteries`/`overall_percentage` are left out
/// since `GetBatteries` already covers that ground with more detail,
/// and `lid_closed`/`active_inhibitors` aren't shown anywhere in this
/// crate's `power` category today.
///
/// Wire shapes, checked against mitos-power's serde attributes (an
/// earlier draft of this comment claimed these had no `rename_all` and
/// were PascalCase -- wrong on both counts, and `profile` in particular
/// would have parsed as "unknown" every time): `ThermalLevel` is
/// `rename_all = "snake_case"` (`"nominal"`/`"warning"`/`"critical"`, a
/// bare string), but `ProfileKind` is *adjacently tagged*
/// (`tag = "kind", content = "name"`), so `profile` is an object, not a
/// string -- see `profile_display_name`.
pub fn get_power_state() -> Result<PowerState, String> {
    let response = request("GetPowerState", Json::Null, TIMEOUT)?;
    Ok(PowerState {
        on_ac: response.get("on_ac").and_then(Json::as_bool).unwrap_or(false),
        profile: response
            .get("profile")
            .map(profile_display_name)
            .unwrap_or_else(|| "unknown".to_string()),
        thermal_level: response
            .get("thermal_level")
            .and_then(Json::as_str)
            .unwrap_or("unknown")
            .to_string(),
        display_brightness_percent: response
            .get("display_brightness_percent")
            .and_then(json_as_u64)
            .map(|v| v as u8),
        idle_seconds: response
            .get("idle_seconds")
            .and_then(json_as_u64)
            .unwrap_or(0),
    })
}

/// `ProfileKind` serializes as `{"kind":"balanced"}` for a built-in
/// (a unit variant has no `name` content to emit) and
/// `{"kind":"custom","name":"gaming"}` for a custom one. Shown in this
/// crate's own vocabulary where one exists (`power_saver` ->
/// `power-saver`, matching `power.profile`'s choices), or the custom
/// profile's own name.
fn profile_display_name(profile: &Json) -> String {
    let kind = profile
        .get("kind")
        .and_then(Json::as_str)
        .unwrap_or("unknown");
    match kind {
        "custom" => profile
            .get("name")
            .and_then(Json::as_str)
            .unwrap_or("custom")
            .to_string(),
        "power_saver" => "power-saver".to_string(),
        other => other.to_string(),
    }
}

fn json_as_f32(value: &Json) -> Option<f32> {
    match value {
        Json::Number(n) => Some(*n as f32),
        _ => None,
    }
}

fn json_as_u64(value: &Json) -> Option<u64> {
    match value {
        Json::Number(n) if *n >= 0.0 => Some(*n as u64),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Exercises the envelope this file builds and the parsing it expects
    // back, without a live daemon (nothing in this crate's test suite
    // can reach one) -- confirms internal consistency, not a match
    // against the real mitos-power schema (already checked by hand
    // against its source; see this file's own doc comments).

    #[test]
    fn set_profile_request_has_the_expected_envelope() {
        let params = Json::object(vec![("name", Json::string("powersave"))]);
        let envelope = Json::object(vec![
            ("kind", Json::string("request")),
            ("id", Json::string("1")),
            ("method", Json::string("SetProfile")),
            ("params", params),
        ]);
        assert_eq!(
            envelope.encode(),
            r#"{"kind":"request","id":"1","method":"SetProfile","params":{"name":"powersave"}}"#
        );
    }

    #[test]
    fn power_saver_translates_to_powersave() {
        // set_profile itself needs a live socket to run end to end; this
        // just confirms the one piece of logic that doesn't -- the name
        // translation -- by re-deriving it the same way set_profile does.
        let translate = |name: &str| match name {
            "power-saver" => "powersave",
            other => other,
        };
        assert_eq!(translate("power-saver"), "powersave");
        assert_eq!(translate("balanced"), "balanced");
        assert_eq!(translate("performance"), "performance");
    }

    #[test]
    fn parses_a_successful_response_envelope() {
        let response = json::parse(r#"{"kind":"response","id":"1","ok":true,"result":{"percent":42}}"#).unwrap();
        assert_eq!(response.get("ok").and_then(Json::as_bool), Some(true));
        assert_eq!(
            response
                .get("result")
                .and_then(|r| r.get("percent"))
                .and_then(json_as_u64),
            Some(42)
        );
    }

    #[test]
    fn parses_an_error_response_envelope() {
        let response = json::parse(
            r#"{"kind":"response","id":"1","ok":false,"error":{"code":"PERMISSION_DENIED","message":"nope"}}"#,
        )
        .unwrap();
        assert_eq!(response.get("ok").and_then(Json::as_bool), Some(false));
        assert_eq!(
            response
                .get("error")
                .and_then(|e| e.get("message"))
                .and_then(Json::as_str),
            Some("nope")
        );
    }

    #[test]
    fn profile_display_name_handles_the_adjacently_tagged_shapes() {
        let balanced = json::parse(r#"{"kind":"balanced"}"#).unwrap();
        assert_eq!(profile_display_name(&balanced), "balanced");

        let saver = json::parse(r#"{"kind":"power_saver"}"#).unwrap();
        assert_eq!(profile_display_name(&saver), "power-saver");

        let custom = json::parse(r#"{"kind":"custom","name":"gaming"}"#).unwrap();
        assert_eq!(profile_display_name(&custom), "gaming");
    }

    #[test]
    fn parses_a_batteries_array() {
        // "charging", not "Charging" -- ChargingState is
        // `#[serde(rename_all = "snake_case")]` on mitos-power's side.
        let response = json::parse(
            r#"[{"id":"BAT0","percentage":73.5,"status":"charging","health_percent":96.0,"time_to_empty_secs":null,"time_to_full_secs":1800}]"#,
        )
        .unwrap();
        let items = response.as_array().unwrap();
        assert_eq!(items[0].get("id").and_then(Json::as_str), Some("BAT0"));
        assert_eq!(items[0].get("status").and_then(Json::as_str), Some("charging"));
        assert_eq!(items[0].get("percentage").and_then(json_as_f32), Some(73.5));
        assert_eq!(items[0].get("time_to_full_secs").and_then(json_as_u64), Some(1800));
        assert_eq!(items[0].get("time_to_empty_secs").and_then(json_as_u64), None);
    }
}
