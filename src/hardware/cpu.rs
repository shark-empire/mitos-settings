use std::fs;

pub fn model_name() -> Option<String> {
    let content = fs::read_to_string("/proc/cpuinfo").ok()?;
    content
        .lines()
        .find(|l| l.starts_with("model name"))
        .and_then(|l| l.split_once(':'))
        .map(|(_, v)| v.trim().to_string())
}

pub fn core_count() -> usize {
    fs::read_to_string("/proc/cpuinfo")
        .map(|c| c.lines().filter(|l| l.starts_with("processor")).count())
        .unwrap_or(0)
}

/// (1, 5, 15) minute load averages, if `/proc/loadavg` is readable.
pub fn load_average() -> Option<(f32, f32, f32)> {
    let content = fs::read_to_string("/proc/loadavg").ok()?;
    let mut parts = content.split_whitespace();
    let one = parts.next()?.parse().ok()?;
    let five = parts.next()?.parse().ok()?;
    let fifteen = parts.next()?.parse().ok()?;
    Some((one, five, fifteen))
}
