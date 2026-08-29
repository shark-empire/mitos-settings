use crate::settings::manager::SettingsManager;

pub fn execute(manager: &SettingsManager, args: &[String]) -> Result<String, String> {
    let key = args.first().ok_or_else(|| "usage: mitos-settings get <key>".to_string())?;
    manager.get(key).map(|v| v.to_string()).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::manager::test_support::isolated_manager;
    use crate::settings::manager::Mode;
    use crate::settings::value::Value;

    #[test]
    fn missing_key_argument_is_a_usage_error() {
        let (manager, dir) = isolated_manager(Mode::Standalone);
        assert!(execute(&manager, &[]).is_err());
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn returns_the_current_value_as_text() {
        let (mut manager, dir) = isolated_manager(Mode::Standalone);
        manager.set("sound.volume", Value::Int(33)).unwrap();
        let out = execute(&manager, &["sound.volume".to_string()]).unwrap();
        assert_eq!(out, "33");
        std::fs::remove_dir_all(dir).ok();
    }
}
