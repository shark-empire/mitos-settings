use super::keyboard::{parse_input_devices, InputDevice};

pub fn list() -> Vec<InputDevice> {
    parse_input_devices()
        .into_iter()
        .filter(|d| {
            let n = d.name.to_lowercase();
            n.contains("touchpad") || n.contains("trackpad")
        })
        .collect()
}
