//! Battery info for `categories::battery`'s `live_info()`. No settings
//! to `apply()` here -- `battery.low_battery_threshold` has no
//! mitos-power IPC equivalent (config-file-only; see
//! `services::power`'s doc comment for the full reasoning, which
//! applies here too) -- just the live read, upgraded from the local
//! `hardware::battery::list()` sysfs read to mitos-power's own
//! `GetBatteries` when it's reachable, the same "prefer the daemon,
//! fall back to local hardware, never fail the page over it" choice
//! `services::network::live_status` already makes.

use crate::hardware;
use crate::power::client;

pub fn live_status() -> Vec<(String, String)> {
    match client::list_batteries() {
        Ok(batteries) if !batteries.is_empty() => batteries
            .into_iter()
            .map(|b| {
                let percent = b
                    .percentage
                    .map(|p| format!("{p:.0}%"))
                    .unwrap_or_else(|| "unknown".to_string());
                // `ChargingState` is snake_case on the wire
                // (`not_charging`) -- fine for a machine, not for a label.
                let mut line = format!("{}: {percent}, {}", b.id, b.status.replace('_', " "));
                if let Some(health) = b.health_percent {
                    line.push_str(&format!(", health {health:.0}%"));
                }
                if let Some(secs) = b.time_to_empty_secs {
                    line.push_str(&format!(", {} to empty", format_duration(secs)));
                } else if let Some(secs) = b.time_to_full_secs {
                    line.push_str(&format!(", {} to full", format_duration(secs)));
                }
                ("battery".to_string(), line)
            })
            .collect(),
        _ => {
            let batteries = hardware::battery::list();
            if batteries.is_empty() {
                return vec![(
                    "battery".to_string(),
                    "no battery detected (desktop or VM)".to_string(),
                )];
            }
            batteries
                .into_iter()
                .map(|b| {
                    let percent = b
                        .capacity_percent
                        .map(|p| format!("{p}%"))
                        .unwrap_or_else(|| "unknown".to_string());
                    let status = b.status.unwrap_or_else(|| "unknown".to_string());
                    (
                        "battery".to_string(),
                        format!("{}: {percent}, {status}", b.name),
                    )
                })
                .collect()
        }
    }
}

fn format_duration(secs: u64) -> String {
    let hours = secs / 3600;
    let minutes = (secs % 3600) / 60;
    if hours > 0 {
        format!("{hours}h {minutes}m")
    } else {
        format!("{minutes}m")
    }
}
