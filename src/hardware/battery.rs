use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct BatteryInfo {
    pub name: String,
    pub capacity_percent: Option<u8>,
    pub status: Option<String>,
}

pub fn list() -> Vec<BatteryInfo> {
    let mut batteries = Vec::new();
    let Ok(entries) = fs::read_dir("/sys/class/power_supply") else { return batteries };
    for entry in entries.flatten() {
        let path: PathBuf = entry.path();
        let ty = fs::read_to_string(path.join("type")).unwrap_or_default();
        if ty.trim() != "Battery" {
            continue;
        }
        let capacity_percent = fs::read_to_string(path.join("capacity")).ok().and_then(|s| s.trim().parse().ok());
        let status = fs::read_to_string(path.join("status")).ok().map(|s| s.trim().to_string());
        batteries.push(BatteryInfo {
            name: entry.file_name().to_string_lossy().to_string(),
            capacity_percent,
            status,
        });
    }
    batteries
}
