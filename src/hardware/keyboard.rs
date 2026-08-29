use std::fs;

#[derive(Debug, Clone)]
pub struct InputDevice {
    pub name: String,
    pub handlers: String,
}

/// Lists devices the kernel classifies as keyboards (anything exposing an
/// evdev "kbd" handler). Reading `/proc/bus/input/devices` needs no special
/// privileges.
pub fn list() -> Vec<InputDevice> {
    parse_input_devices().into_iter().filter(|d| d.handlers.contains("kbd")).collect()
}

/// Shared by `hardware::mouse` and `hardware::touchpad` too, since all three
/// read the same `/proc/bus/input/devices` table and just filter it
/// differently.
pub(crate) fn parse_input_devices() -> Vec<InputDevice> {
    let Ok(content) = fs::read_to_string("/proc/bus/input/devices") else { return Vec::new() };
    let mut devices = Vec::new();
    let mut name = String::new();
    let mut handlers = String::new();

    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("N: Name=") {
            name = rest.trim_matches('"').to_string();
        } else if let Some(rest) = line.strip_prefix("H: Handlers=") {
            handlers = rest.trim().to_string();
        } else if line.is_empty() && !name.is_empty() {
            devices.push(InputDevice { name: std::mem::take(&mut name), handlers: std::mem::take(&mut handlers) });
        }
    }
    if !name.is_empty() {
        devices.push(InputDevice { name, handlers });
    }
    devices
}
