//! Platform abstraction boundary. MITOS itself is Linux-based, so `linux`
//! is the only backend today — but call sites reach it through this module
//! rather than `std::process::Command` directly, so adding a second
//! backend later would mean adding a file here, not touching every
//! `hardware`/`services` module that shells out.

pub mod linux;

pub use linux::{command_exists, os_release, run_command};
