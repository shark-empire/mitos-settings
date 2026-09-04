use crate::categories::{self, Category};
use crate::settings::json;
use crate::settings::manager::SettingsManager;

/// With no argument, lists every category. With a category id, lists that
/// category's settings (current value included) plus any live info. Add
/// `--json` (with or without a category) to get current values as JSON
/// instead of the human-readable table — see INTEGRATION.md.
pub fn execute(manager: &SettingsManager, args: &[String]) -> Result<String, String> {
    let as_json = args.iter().any(|a| a == "--json");
    let category_id = args.iter().find(|a| a.as_str() != "--json");

    if as_json {
        match category_id {
            Some(id) if categories::find(id).is_none() => Err(format!("unknown category '{id}'")),
            Some(id) => Ok(json::values_to_json(manager, Some(id.as_str()))),
            None => Ok(json::values_to_json(manager, None)),
        }
    } else {
        match category_id {
            None => Ok(list_categories()),
            Some(id) => list_category(manager, id),
        }
    }
}

fn list_categories() -> String {
    let mut out = String::new();
    for cat in categories::all() {
        out.push_str(&format!("{:<16} {}\n", cat.id(), cat.name()));
    }
    out
}

fn list_category(manager: &SettingsManager, category_id: &str) -> Result<String, String> {
    let category = categories::find(category_id).ok_or_else(|| format!("unknown category '{category_id}'"))?;

    let mut out = String::new();
    for spec in manager.schema().by_category(category.id()) {
        let value = manager.get(spec.key).map(|v| v.to_string()).unwrap_or_default();
        out.push_str(&format!("{:<32} {}\n", spec.key, value));
    }
    for (label, value) in category.live_info() {
        out.push_str(&format!("{label:<32} {value} (live)\n"));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::manager::test_support::isolated_manager;
    use crate::settings::manager::Mode;

    #[test]
    fn no_args_lists_every_category() {
        let (manager, dir) = isolated_manager(Mode::Standalone);
        let out = execute(&manager, &[]).unwrap();
        assert!(out.contains("display"));
        assert!(out.contains("about"));
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn category_arg_lists_its_keys() {
        let (manager, dir) = isolated_manager(Mode::Standalone);
        let out = execute(&manager, &["sound".to_string()]).unwrap();
        assert!(out.contains("sound.volume"));
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn unknown_category_is_an_error() {
        let (manager, dir) = isolated_manager(Mode::Standalone);
        assert!(execute(&manager, &["not-a-category".to_string()]).is_err());
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn json_flag_with_no_category_dumps_every_value() {
        let (manager, dir) = isolated_manager(Mode::Standalone);
        let out = execute(&manager, &["--json".to_string()]).unwrap();
        assert!(out.contains("\"sound.volume\""));
        assert!(out.contains("\"display.brightness\""));
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn json_flag_with_category_filters_to_it() {
        let (manager, dir) = isolated_manager(Mode::Standalone);
        let out = execute(&manager, &["sound".to_string(), "--json".to_string()]).unwrap();
        assert!(out.contains("\"sound.volume\""));
        assert!(!out.contains("\"display.brightness\""));
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn json_flag_order_does_not_matter() {
        let (manager, dir) = isolated_manager(Mode::Standalone);
        let a = execute(&manager, &["--json".to_string(), "sound".to_string()]).unwrap();
        let b = execute(&manager, &["sound".to_string(), "--json".to_string()]).unwrap();
        assert_eq!(a, b);
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn json_flag_with_unknown_category_is_still_an_error() {
        let (manager, dir) = isolated_manager(Mode::Standalone);
        assert!(execute(&manager, &["not-a-category".to_string(), "--json".to_string()]).is_err());
        std::fs::remove_dir_all(dir).ok();
    }
}
