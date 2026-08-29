//! Exercises `SettingsManager` the way a real caller would: through the
//! public crate API only, against the full, real schema built by
//! `categories::register_all` (not a hand-built toy schema).

use mitos_settings::categories;
use mitos_settings::settings::manager::{Mode, SettingsManager};
use mitos_settings::settings::persistence::Store;
use mitos_settings::settings::value::Value;
use std::path::{Path, PathBuf};

fn temp_dir(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("mitos-settings-itest-{label}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn manager_in(dir: &Path) -> SettingsManager {
    SettingsManager::with_stores(Mode::Standalone, Store::at(dir.join("user.conf")), Store::at(dir.join("system.conf")))
        .expect("manager should load against a fresh, empty store")
}

#[test]
fn every_category_registers_at_least_one_setting_except_about() {
    let dir = temp_dir("categories");
    let manager = manager_in(&dir);
    for cat in categories::all() {
        let count = manager.schema().by_category(cat.id()).count();
        assert!(count > 0 || cat.id() == "about", "category '{}' registered no settings", cat.id());
    }
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn set_get_reset_cycle_for_a_user_level_setting() {
    let dir = temp_dir("cycle");
    let mut manager = manager_in(&dir);
    assert_eq!(manager.get("sound.volume").unwrap(), &Value::Int(50));
    manager.set("sound.volume", Value::Int(20)).unwrap();
    assert_eq!(manager.get("sound.volume").unwrap(), &Value::Int(20));
    manager.reset("sound.volume").unwrap();
    assert_eq!(manager.get("sound.volume").unwrap(), &Value::Int(50));
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn choice_constraints_are_enforced_through_the_manager() {
    let dir = temp_dir("choices");
    let mut manager = manager_in(&dir);
    assert!(manager.set("power.profile", Value::Str("turbo".into())).is_err());
    assert!(manager.set("power.profile", Value::Str("performance".into())).is_ok());
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn range_constraints_are_enforced_through_the_manager() {
    let dir = temp_dir("ranges");
    let mut manager = manager_in(&dir);
    assert!(manager.set("display.brightness", Value::Int(150)).is_err());
    assert!(manager.set("display.brightness", Value::Int(60)).is_ok());
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn read_only_settings_reject_writes() {
    let dir = temp_dir("readonly");
    let mut manager = manager_in(&dir);
    let err = manager.set("security.firewall_status", Value::Str("on".into())).unwrap_err();
    assert!(err.to_string().contains("read-only"));
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn settings_persist_across_manager_reloads() {
    let dir = temp_dir("persist");
    {
        let mut manager = manager_in(&dir);
        manager.set("sound.volume", Value::Int(77)).unwrap();
    }
    let manager = manager_in(&dir);
    assert_eq!(manager.get("sound.volume").unwrap(), &Value::Int(77));
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn reset_all_restores_every_writable_default() {
    let dir = temp_dir("reset-all");
    let mut manager = manager_in(&dir);
    manager.set("sound.volume", Value::Int(1)).unwrap();
    manager.set("display.brightness", Value::Int(1)).unwrap();
    manager.reset_all().unwrap();
    assert_eq!(manager.get("sound.volume").unwrap(), &Value::Int(50));
    assert_eq!(manager.get("display.brightness").unwrap(), &Value::Int(80));
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn unknown_key_is_rejected_for_get_set_and_reset() {
    let dir = temp_dir("unknown-key");
    let mut manager = manager_in(&dir);
    assert!(manager.get("nonexistent.key").is_err());
    assert!(manager.set("nonexistent.key", Value::Bool(true)).is_err());
    assert!(manager.reset("nonexistent.key").is_err());
    std::fs::remove_dir_all(dir).ok();
}
