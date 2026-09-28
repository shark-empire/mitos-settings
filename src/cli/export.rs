//! `mitos-settings export [file]` -- the write counterpart to `import`,
//! and, functionally, just a named, file-writing wrapper around
//! `list --json`'s existing output. `import` already documents itself
//! as reading "a JSON document shaped like `list --json`'s output," so
//! that equivalence already existed -- this exists mainly so someone
//! reaching for "how do I export my settings" finds a command named
//! that, rather than needing to already know `list --json > file.json`
//! does the same thing. With no path, prints the JSON to stdout, same
//! as `list --json`.
//!
//! Live info (`Category::live_info()` -- disk usage, paired devices,
//! and the like) is deliberately not included: none of it is a stored
//! value `import` could apply back, so including it here would stop
//! this export from round-tripping through `import` cleanly, the
//! opposite of the point (see this file's own test for that round trip
//! actually working).

use crate::settings::json;
use crate::settings::manager::SettingsManager;

pub fn execute(manager: &SettingsManager, args: &[String]) -> Result<String, String> {
    let contents = json::values_to_json(manager, None);

    match args.first() {
        Some(path) => {
            std::fs::write(path, format!("{contents}\n"))
                .map_err(|e| format!("couldn't write {path}: {e}"))?;
            Ok(format!("Exported settings to {path}.\n"))
        }
        None => Ok(contents),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::manager::test_support::isolated_manager;
    use crate::settings::manager::Mode;
    use crate::settings::value::Value;

    #[test]
    fn with_no_path_prints_json_to_stdout() {
        let (manager, dir) = isolated_manager(Mode::Standalone);
        let out = execute(&manager, &[]).unwrap();
        assert!(out.contains("\"sound.volume\""));
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn with_a_path_writes_the_file_instead_of_printing() {
        let (manager, dir) = isolated_manager(Mode::Standalone);
        let file = dir.join("export-test.json");
        let result = execute(&manager, &[file.display().to_string()]);
        assert!(result.unwrap().contains("Exported"));
        let written = std::fs::read_to_string(&file).unwrap();
        assert!(written.contains("\"sound.volume\""));
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn round_trips_through_import() {
        let (mut manager, dir) = isolated_manager(Mode::Standalone);
        manager.set("sound.volume", Value::Int(77)).unwrap();
        let file = dir.join("round-trip.json");
        execute(&manager, &[file.display().to_string()]).unwrap();

        let (mut fresh, dir2) = isolated_manager(Mode::Standalone);
        let contents = std::fs::read_to_string(&file).unwrap();
        let pairs = json::values_from_json(fresh.schema(), &contents).unwrap();
        fresh.import_values(pairs).unwrap();
        assert_eq!(fresh.get("sound.volume").unwrap(), &Value::Int(77));

        std::fs::remove_dir_all(dir).ok();
        std::fs::remove_dir_all(dir2).ok();
    }
}
