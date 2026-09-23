//! `mitos-settings history [count]` -- shows the most recent setting
//! changes (user-scope and system-scope merged, newest first), from
//! `SettingsManager::recent_history`. Defaults to the last 20; pass a
//! number to see more or fewer. Timestamps are raw Unix seconds -- pipe
//! through `date -d @<seconds>` (or `date -r <seconds>` on macOS/BSD) for
//! a calendar date; this crate stays dependency-free (see
//! INTEGRATION.md), so there's no bundled date-formatting logic here.

use crate::settings::manager::SettingsManager;

pub fn execute(manager: &SettingsManager, args: &[String]) -> Result<String, String> {
    let count: usize = match args.first() {
        Some(arg) => arg.parse().map_err(|_| {
            format!("'{arg}' isn't a number; usage: mitos-settings history [count]")
        })?,
        None => 20,
    };

    let Some(entries) = manager.recent_history(count) else {
        return Ok("History isn't available for this manager instance.\n".to_string());
    };
    if entries.is_empty() {
        return Ok("No changes recorded yet.\n".to_string());
    }

    let mut out = String::new();
    out.push_str(&format!("{:<12} {:<32} {}\n", "WHEN (unix)", "KEY", "VALUE"));
    for entry in entries {
        out.push_str(&format!(
            "{:<12} {:<32} {}\n",
            entry.timestamp_unix, entry.key, entry.value
        ));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::manager::test_support::isolated_manager;
    use crate::settings::manager::Mode;
    use crate::settings::value::Value;

    #[test]
    fn reports_unavailable_without_history_paths_configured() {
        let (manager, dir) = isolated_manager(Mode::Standalone);
        let output = execute(&manager, &[]).unwrap();
        assert!(output.contains("isn't available"));
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn shows_a_recorded_change() {
        let (manager, dir) = isolated_manager(Mode::Standalone);
        let mut manager = manager
            .with_history_paths(dir.join("user-history.log"), dir.join("system-history.log"));
        manager.set("sound.volume", Value::Int(11)).unwrap();

        let output = execute(&manager, &[]).unwrap();
        assert!(output.contains("sound.volume"));
        assert!(output.contains("11"));
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn rejects_a_non_numeric_count() {
        let (manager, dir) = isolated_manager(Mode::Standalone);
        assert!(execute(&manager, &["not-a-number".to_string()]).is_err());
        std::fs::remove_dir_all(dir).ok();
    }
}
