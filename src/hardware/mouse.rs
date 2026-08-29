use super::keyboard::{parse_input_devices, InputDevice};

pub fn list() -> Vec<InputDevice> {
    parse_input_devices()
        .into_iter()
        .filter(|d| d.handlers.contains("mouse") && !d.name.to_lowercase().contains("touchpad"))
        .collect()
}
