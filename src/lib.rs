//! # mitos-settings
//!
//! System settings manager and daemon for MITOS. See `README.md` for an
//! overview and `docs/architecture.md` for how the pieces fit together.
//!
//! This crate is split into a library (this file) and a thin binary
//! (`main.rs`) so that `tests/*.rs` integration tests — and anything else
//! that wants to embed this logic — can exercise `SettingsManager` and
//! friends directly, without going through a subprocess.

pub mod app;
pub mod categories;
pub mod cli;
pub mod config;
pub mod hardware;
pub mod ipc;
pub mod notifications;
pub mod permissions;
pub mod platform;
pub mod services;
pub mod settings;
