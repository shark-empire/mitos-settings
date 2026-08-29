use crate::hardware;
use std::io;
use std::process::Command;

pub fn list_interfaces() -> Vec<hardware::network::NetIface> {
    hardware::network::list()
}

pub fn set_wifi_enabled(enabled: bool) -> io::Result<()> {
    let state = if enabled { "on" } else { "off" };
    let status = Command::new("nmcli").args(["radio", "wifi", state]).status()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::new(io::ErrorKind::Other, "nmcli reported failure (is NetworkManager running?)"))
    }
}

pub fn set_interface_up(name: &str, up: bool) -> io::Result<()> {
    let state = if up { "up" } else { "down" };
    let status = Command::new("ip").args(["link", "set", name, state]).status()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::new(io::ErrorKind::Other, "ip link set failed (needs root)"))
    }
}
