//! Helper for the rare case where an already-running, non-daemon process
//! needs to escalate a single external command to root (rather than going
//! through the IPC daemon). Prefers a polkit-style prompt, falls back to
//! non-interactive sudo.

use crate::platform;
use std::io;
use std::process::{Command, Output};

pub fn run_as_root(cmd: &str, args: &[&str]) -> io::Result<Output> {
    if platform::command_exists("pkexec") {
        return Command::new("pkexec").arg(cmd).args(args).output();
    }
    if platform::command_exists("sudo") {
        let mut full = vec!["-n", cmd];
        full.extend_from_slice(args);
        return Command::new("sudo").args(full).output();
    }
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "no privilege escalation helper (pkexec or sudo) is available on this system",
    ))
}
