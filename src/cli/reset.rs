use crate::settings::manager::SettingsManager;

pub fn execute(manager: &mut SettingsManager, args: &[String]) -> Result<String, String> {
    match args.first().map(String::as_str) {
        None => Err("usage: mitos-settings reset <key> | --all".to_string()),
        Some("--all") => manager.reset_all().map(|()| "Reset every setting to its default.".to_string()).map_err(|e| e.to_string()),
        Some(key) => manager.reset(key).map(|()| format!("Reset {key} to its default.")).map_err(|e| e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::manager::test_support::isolated_manager;
    use crate::settings::manager::Mode;
    use crate::settings::value::Value;

    #[test]
    fn resets_a_single_key() {
        let (mut manager, dir) = isolated_manager(Mode::Standalone);
        manager.set("sound.volume", Value::Int(5)).unwrap();
        let result = execute(&mut manager, &["sound.volume".to_string()]);
        assert!(result.is_ok());
        assert_eq!(manager.get("sound.volume").unwrap(), &Value::Int(50));
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn resets_everything_with_all_flag() {
        let (mut manager, dir) = isolated_manager(Mode::Standalone);
        manager.set("sound.volume", Value::Int(5)).unwrap();
        manager.set("display.brightness", Value::Int(5)).unwrap();
        let result = execute(&mut manager, &["--all".to_string()]);
        assert!(result.is_ok());
        assert_eq!(manager.get("sound.volume").unwrap(), &Value::Int(50));
        assert_eq!(manager.get("display.brightness").unwrap(), &Value::Int(80));
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn missing_argument_is_a_usage_error() {
        let (mut manager, dir) = isolated_manager(Mode::Standalone);
        assert!(execute(&mut manager, &[]).is_err());
        std::fs::remove_dir_all(dir).ok();
    }
}
