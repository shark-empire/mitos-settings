use std::collections::HashMap;
use std::fs;

#[derive(Debug, Clone, Copy, Default)]
pub struct MemInfo {
    pub total_kb: u64,
    pub available_kb: u64,
    pub swap_total_kb: u64,
    pub swap_free_kb: u64,
}

impl MemInfo {
    pub fn used_kb(&self) -> u64 {
        self.total_kb.saturating_sub(self.available_kb)
    }
}

pub fn info() -> Option<MemInfo> {
    let content = fs::read_to_string("/proc/meminfo").ok()?;
    let mut fields: HashMap<&str, u64> = HashMap::new();
    for line in content.lines() {
        let Some((key, rest)) = line.split_once(':') else { continue };
        if let Some(value) = rest.trim().split_whitespace().next().and_then(|v| v.parse().ok()) {
            fields.insert(key, value);
        }
    }
    Some(MemInfo {
        total_kb: *fields.get("MemTotal").unwrap_or(&0),
        available_kb: *fields.get("MemAvailable").unwrap_or(&0),
        swap_total_kb: *fields.get("SwapTotal").unwrap_or(&0),
        swap_free_kb: *fields.get("SwapFree").unwrap_or(&0),
    })
}
