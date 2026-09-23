//! `mitos-settings import <file>` -- reads a JSON document shaped like
//! `list --json`'s output and applies it via
//! `SettingsManager::import_values`, which validates every value before
//! writing any of them (see that method's doc comment for exactly what
//! guarantee that is and isn't).

use crate::settings::json;
use crate::settings::manager::SettingsManager;

pub fn execute(manager: &mut SettingsManager, args: &[String]) -> Result<String, String> {
    let Some(path) = args.first() else {
        return Err("usage: mitos-settings import <file.json>".to_string());
    };

    let contents =
        std::fs::read_to_string(path).map_err(|e| format!("couldn't read {path}: {e}"))?;
    let pairs = json::values_from_json(manager.schema(), &contents)?;
    let count = manager.import_values(pairs).map_err(|e| e.to_string())?;

    Ok(format!("Imported {count} setting(s) from {path}.\n"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::manager::test_support::isolated_manager;
    use crate::settings::manager::Mode;
    use crate::settings::value::Value;

    #[test]
    fn rejects_missing_path_argument() {
        let (mut manager, dir) = isolated_manager(Mode::Standalone);
        assert!(execute(&mut manager, &[]).is_err());
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn rejects_a_file_that_does_not_exist() {
        let (mut manager, dir) = isolated_manager(Mode::Standalone);
        let result = execute(
            &mut manager,
            &["/nonexistent/path/does-not-exist.json".to_string()],
        );
        assert!(result.is_err());
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn imports_a_well_formed_file() {
        let (mut manager, dir) = isolated_manager(Mode::Standalone);
        let file = dir.join("import-test.json");
        std::fs::write(&file, "{\"sound.volume\": 33}\n").unwrap();

        let result = execute(&mut manager, &[file.display().to_string()]);
        assert!(result.unwrap().contains("Imported 1"));
        assert_eq!(manager.get("sound.volume").unwrap(), &Value::Int(33));
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn a_malformed_file_changes_nothing() {
        let (mut manager, dir) = isolated_manager(Mode::Standalone);
        let file = dir.join("bad-import.json");
        // display.brightness's range is 0..=100 -- 999 fails validation,
        // so sound.volume alongside it should never get applied either.
        std::fs::write(
            &file,
            "{\"sound.volume\": 33, \"display.brightness\": 999}\n",
        )
        .unwrap();

        let result = execute(&mut manager, &[file.display().to_string()]);
        assert!(result.is_err());
        assert_eq!(manager.get("sound.volume").unwrap(), &Value::Int(50));
        std::fs::remove_dir_all(dir).ok();
    }
}
