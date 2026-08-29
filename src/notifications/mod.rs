//! In-process events (`events::EventBus`) plus a best-effort bridge to the
//! desktop notification daemon, for the rare setting change worth
//! surfacing as a toast (e.g. "Wi-Fi turned off").

pub mod events;

pub use events::{Event, EventBus};

/// Sends a desktop notification via `notify-send`, if it's installed.
/// Silently does nothing otherwise — a missing notification daemon should
/// never be a reason to fail a settings change.
pub fn toast(summary: &str, body: &str) {
    let _ = std::process::Command::new("notify-send").arg(summary).arg(body).status();
}
