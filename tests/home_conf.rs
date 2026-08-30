//! End-to-end test of the home.conf projection: build a manager with an
//! explicit (throwaway) home.conf path, change a setting through the
//! normal public `set` API, and confirm the projected file reflects it —
//! without ever touching the real `~/.config/mitos/home.conf`.

use mitos_settings::settings::manager::{Mode, SettingsManager};
use mitos_settings::settings::persistence::Store;
use mitos_settings::settings::value::Value;
use std::path::PathBuf;

fn temp_dir(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("mitos-home-conf-itest-{label}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn manager_with_home_conf(dir: &std::path::Path) -> SettingsManager {
    SettingsManager::with_stores(Mode::Standalone, Store::at(dir.join("user.conf")), Store::at(dir.join("system.conf")))
        .unwrap()
        .with_home_conf_path(dir.join("home.conf"))
}

#[test]
fn changing_a_relevant_setting_regenerates_home_conf() {
    let dir = temp_dir("relevant");
    let mut manager = manager_with_home_conf(&dir);

    manager.set("appearance.accent_color", Value::Str("#00ff00".into())).unwrap();
    manager.set("appearance.glass_opacity", Value::Float(0.5)).unwrap();
    manager.set("appearance.dock_enabled", Value::Bool(false)).unwrap();

    let contents = std::fs::read_to_string(dir.join("home.conf")).unwrap();
    assert!(contents.contains("accent_color = #00ff00"));
    assert!(contents.contains("glass_opacity = 0.5"));
    assert!(contents.contains("dock = false"));

    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn changing_an_unrelated_setting_does_not_create_home_conf() {
    let dir = temp_dir("unrelated");
    let mut manager = manager_with_home_conf(&dir);

    manager.set("sound.volume", Value::Int(10)).unwrap();

    assert!(!dir.join("home.conf").exists());
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn manager_without_a_home_conf_path_never_writes_one() {
    let dir = temp_dir("no-path");
    let mut manager =
        SettingsManager::with_stores(Mode::Standalone, Store::at(dir.join("user.conf")), Store::at(dir.join("system.conf")))
            .unwrap();

    // No .with_home_conf_path() call -- this is exactly the shape every
    // other test in this suite uses, and it must never touch a real path.
    manager.set("appearance.accent_color", Value::Str("#123456".into())).unwrap();

    assert!(!dir.join("home.conf").exists());
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn invalid_hex_accent_color_is_rejected_before_it_ever_reaches_home_conf() {
    let dir = temp_dir("invalid-hex");
    let mut manager = manager_with_home_conf(&dir);

    let err = manager.set("appearance.accent_color", Value::Str("not-a-color".into())).unwrap_err();
    assert!(err.to_string().contains("not valid"));
    assert!(!dir.join("home.conf").exists());

    std::fs::remove_dir_all(dir).ok();
}
