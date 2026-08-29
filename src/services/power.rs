use std::io;
use std::process::Command;

pub fn set_profile(profile: &str) -> io::Result<()> {
    let status = Command::new("powerprofilesctl").args(["set", profile]).status()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::new(io::ErrorKind::Other, "powerprofilesctl reported failure"))
    }
}

pub fn suspend_now() -> io::Result<()> {
    let status = Command::new("systemctl").arg("suspend").status()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::new(io::ErrorKind::Other, "systemctl suspend failed"))
    }
}
