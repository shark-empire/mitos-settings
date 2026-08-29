use crate::categories::Category;
use crate::settings::schema::Schema;
use std::fs;
use std::process::Command;

pub struct AboutCategory;

impl Category for AboutCategory {
    fn id(&self) -> &'static str {
        "about"
    }
    fn name(&self) -> &'static str {
        "About MITOS"
    }
    fn icon(&self) -> &'static str {
        "help-about"
    }
    fn subitems(&self) -> &'static [&'static str] {
        &["Version", "Kernel", "Hardware", "License", "System information"]
    }

    /// About has nothing to configure — it's a read-only snapshot of the
    /// system — so there's nothing to add to the schema.
    fn register(&self, _schema: &mut Schema) {}

    fn live_info(&self) -> Vec<(&'static str, String)> {
        let summary = crate::hardware::summary();
        let distro = crate::platform::os_release()
            .get("PRETTY_NAME")
            .cloned()
            .unwrap_or_else(|| "MITOS".to_string());
        let mut rows = vec![
            ("distribution", distro),
            ("version", env!("CARGO_PKG_VERSION").to_string()),
            ("kernel", kernel_version()),
            ("cpu", summary.cpu_model.unwrap_or_else(|| "unknown".to_string())),
            ("cpu_cores", summary.cpu_cores.to_string()),
            ("license", "MIT".to_string()),
        ];
        if let Some(mem_kb) = summary.mem_total_kb {
            rows.push(("memory", format!("{:.1} GiB", mem_kb as f64 / 1024.0 / 1024.0)));
        }
        for gpu in summary.gpu_names {
            rows.push(("gpu", gpu));
        }
        rows
    }
}

fn kernel_version() -> String {
    Command::new("uname")
        .arg("-r")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .or_else(|| fs::read_to_string("/proc/version").ok().map(|s| s.trim().to_string()))
        .unwrap_or_else(|| "unknown".to_string())
}
