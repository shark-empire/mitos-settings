use crate::categories::{self, Category};
use crate::settings::manager::SettingsManager;

/// With no argument, lists every category. With a category id, lists that
/// category's settings (current value included) plus any live info.
pub fn execute(manager: &SettingsManager, args: &[String]) -> Result<String, String> {
    match args.first() {
        None => Ok(list_categories()),
        Some(category_id) => list_category(manager, category_id),
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
}
