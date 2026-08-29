use std::io;
use std::process::Command;

pub fn current_timezone() -> Option<String> {
    let output = Command::new("timedatectl").args(["show", "-p", "Timezone", "--value"]).output().ok()?;
    let tz = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (!tz.is_empty()).then_some(tz)
}

pub fn set_timezone(tz: &str) -> io::Result<()> {
    let status = Command::new("timedatectl").args(["set-timezone", tz]).status()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::new(io::ErrorKind::Other, "timedatectl set-timezone failed (needs root)"))
    }
}

pub fn set_ntp_enabled(enabled: bool) -> io::Result<()> {
    let state = if enabled { "true" } else { "false" };
    let status = Command::new("timedatectl").args(["set-ntp", state]).status()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::new(io::ErrorKind::Other, "timedatectl set-ntp failed (needs root)"))
    }
}
