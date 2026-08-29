use std::io;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct MountInfo {
    pub device: String,
    pub target: String,
    pub fstype: String,
}

pub fn list_mounts() -> Vec<MountInfo> {
    let Ok(content) = std::fs::read_to_string("/proc/mounts") else { return Vec::new() };
    content
        .lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let device = parts.next()?.to_string();
            let target = parts.next()?.to_string();
            let fstype = parts.next()?.to_string();
            (device.starts_with('/') || device == "tmpfs").then_some(MountInfo { device, target, fstype })
        })
        .collect()
}

#[derive(Debug, Clone)]
pub struct DiskUsage {
    pub filesystem: String,
    pub used_percent: u8,
    pub mount: String,
}

/// Shells out to `df` rather than calling `statvfs(2)` directly, so this
/// module stays dependency-free (no libc/FFI needed for one syscall).
pub fn disk_usage() -> io::Result<Vec<DiskUsage>> {
    let output = Command::new("df").arg("-h").output()?;
    let text = String::from_utf8_lossy(&output.stdout);
    Ok(text
        .lines()
        .skip(1) // header row
        .filter_map(|line| {
            let cols: Vec<&str> = line.split_whitespace().collect();
            if cols.len() < 6 {
                return None;
            }
            let used_percent = cols[4].trim_end_matches('%').parse().ok()?;
            Some(DiskUsage { filesystem: cols[0].to_string(), used_percent, mount: cols[5].to_string() })
        })
        .collect())
}
