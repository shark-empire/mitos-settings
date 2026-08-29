//! Read-only hardware introspection, straight from `/proc` and `/sys`. No
//! mutation lives here — that's `services`, which uses these modules for
//! its own read side and layers command execution on top for writes.

pub mod audio;
pub mod battery;
pub mod bluetooth;
pub mod cpu;
pub mod displays;
pub mod gpu;
pub mod keyboard;
pub mod memory;
pub mod mouse;
pub mod network;
pub mod touchpad;

#[derive(Debug, Clone)]
pub struct SystemSummary {
    pub cpu_model: Option<String>,
    pub cpu_cores: usize,
    pub mem_total_kb: Option<u64>,
    pub gpu_names: Vec<String>,
}

/// Aggregates a handful of the modules above into the snapshot
/// `categories::about` shows.
pub fn summary() -> SystemSummary {
    SystemSummary {
        cpu_model: cpu::model_name(),
        cpu_cores: cpu::core_count(),
        mem_total_kb: memory::info().map(|m| m.total_kb),
        gpu_names: gpu::list().into_iter().map(|g| g.name).collect(),
    }
}
