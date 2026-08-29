use std::fs;

#[derive(Debug, Clone)]
pub struct NetIface {
    pub name: String,
    pub operstate: String,
}

pub fn list() -> Vec<NetIface> {
    let mut ifaces = Vec::new();
    let Ok(entries) = fs::read_dir("/sys/class/net") else { return ifaces };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name == "lo" {
            continue;
        }
        let operstate = fs::read_to_string(entry.path().join("operstate"))
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|_| "unknown".to_string());
        ifaces.push(NetIface { name, operstate });
    }
    ifaces
}
