use std::fs;

#[derive(Debug, Clone)]
pub struct ConnectorInfo {
    pub name: String,
    pub connected: bool,
}

pub fn list_connectors() -> Vec<ConnectorInfo> {
    let mut out = Vec::new();
    let Ok(entries) = fs::read_dir("/sys/class/drm") else { return out };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        let status_path = entry.path().join("status");
        let Ok(status) = fs::read_to_string(&status_path) else { continue };
        out.push(ConnectorInfo { name, connected: status.trim() == "connected" });
    }
    out
}
