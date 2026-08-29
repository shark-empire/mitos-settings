use std::fs;

pub fn adapters() -> Vec<String> {
    fs::read_dir("/sys/class/bluetooth")
        .map(|entries| entries.flatten().map(|e| e.file_name().to_string_lossy().to_string()).collect())
        .unwrap_or_default()
}

pub fn is_present() -> bool {
    !adapters().is_empty()
}
