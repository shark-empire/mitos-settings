use crate::settings::manager::SettingsManager;
use crate::settings::value::Value;

pub fn execute(manager: &mut SettingsManager, args: &[String]) -> Result<String, String> {
    let key = args.first().ok_or_else(|| "usage: mitos-settings set <key> <value>".to_string())?;
    let raw_value = args.get(1).ok_or_else(|| "usage: mitos-settings set <key> <value>".to_string())?;

    let spec = manager.schema().get(key).ok_or_else(|| format!("unknown setting '{key}'"))?;
    let kind = spec.kind;

    let value = Value::parse(kind, raw_value).map_err(|e| format!("invalid value for '{key}': {e}"))?;
    manager.set(key, value).map(|()| format!("Updated {key}.")).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::manager::test_support::isolated_manager;
    use crate::settings::manager::Mode;

    #[test]
    fn sets_a_valid_value() {
        let (mut manager, dir) = isolated_manager(Mode::Standalone);
        let result = execute(&mut manager, &["sound.volume".to_string(), "70".to_string()]);
        assert!(result.is_ok());
        assert_eq!(manager.get("sound.volume").unwrap().to_string(), "70");
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn rejects_invalid_value_before_touching_the_manager() {
        let (mut manager, dir) = isolated_manager(Mode::Standalone);
        let result = execute(&mut manager, &["sound.volume".to_string(), "not-a-number".to_string()]);
        assert!(result.is_err());
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn rejects_unknown_key() {
        let (mut manager, dir) = isolated_manager(Mode::Standalone);
        let result = execute(&mut manager, &["nonexistent.key".to_string(), "1".to_string()]);
        assert!(result.is_err());
        std::fs::remove_dir_all(dir).ok();
    }
}
