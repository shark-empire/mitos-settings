use std::fs;

#[derive(Debug, Clone)]
pub struct GpuInfo {
    pub name: String,
    pub vendor_id: Option<String>,
    pub device_id: Option<String>,
}

/// Best-effort GPU enumeration via sysfs alone. This reports raw PCI
/// vendor/device IDs rather than a friendly name like "NVIDIA RTX 4070" —
/// resolving those needs the pci.ids database, which is out of scope for a
/// dependency-free reference implementation. `about::AboutCategory` shows
/// the IDs as-is.
pub fn list() -> Vec<GpuInfo> {
    let mut gpus = Vec::new();
    let Ok(entries) = fs::read_dir("/sys/class/drm") else { return gpus };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.starts_with("card") || name.contains('-') {
            continue; // skip connector pseudo-nodes like "card0-HDMI-A-1"
        }
        let device_dir = entry.path().join("device");
        let vendor_id = fs::read_to_string(device_dir.join("vendor")).ok().map(|s| s.trim().to_string());
        let device_id = fs::read_to_string(device_dir.join("device")).ok().map(|s| s.trim().to_string());
        if vendor_id.is_some() || device_id.is_some() {
            gpus.push(GpuInfo { name, vendor_id, device_id });
        }
    }
    gpus
}
