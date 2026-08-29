use std::io;
use std::process::{Command, Stdio};

#[derive(Debug, Clone)]
pub struct PairedDevice {
    pub mac: String,
    pub name: String,
}

pub fn set_powered(on: bool) -> io::Result<()> {
    let state = if on { "on" } else { "off" };
    run(&["power", state])
}

pub fn list_devices() -> Vec<PairedDevice> {
    let Ok(output) = Command::new("bluetoothctl").arg("devices").output() else { return Vec::new() };
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| {
            // Format: "Device AA:BB:CC:DD:EE:FF Device Name"
            let mut parts = line.splitn(3, ' ');
            let _literal_device = parts.next()?;
            let mac = parts.next()?.to_string();
            let name = parts.next().unwrap_or("").to_string();
            Some(PairedDevice { mac, name })
        })
        .collect()
}

fn run(args: &[&str]) -> io::Result<()> {
    let status = Command::new("bluetoothctl").args(args).stdout(Stdio::null()).status()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::new(io::ErrorKind::Other, "bluetoothctl reported failure"))
    }
}
