//! Tests the on-disk story: `settings::persistence::Store` round-tripping
//! every `Value` kind, and `config::migration` carrying a v1 file forward.

use mitos_settings::config::loader::{self, RawDocument};
use mitos_settings::config::writer;
use mitos_settings::settings::persistence::Store;
use mitos_settings::settings::value::Value;
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

fn temp_path(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("mitos-persistence-itest-{label}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir.join("settings.conf")
}

#[test]
fn store_round_trips_every_value_kind() {
    let path = temp_path("round-trip");
    let store = Store::at(&path);

    let mut values: HashMap<String, Value> = HashMap::new();
    values.insert("a.bool".to_string(), Value::Bool(true));
    values.insert("a.int".to_string(), Value::Int(-7));
    values.insert("a.float".to_string(), Value::Float(1.5));
    values.insert("a.str".to_string(), Value::Str("hello, world".to_string()));
    values.insert("a.strlist".to_string(), Value::StrList(vec!["x".into(), "y".into()]));

    store.save(&values).unwrap();
    let reloaded = store.load().unwrap();

    for (k, v) in &values {
        assert_eq!(reloaded.get(k), Some(v), "mismatch for key {k}");
    }
    std::fs::remove_dir_all(path.parent().unwrap()).ok();
}

#[test]
fn missing_file_loads_as_empty_rather_than_erroring() {
    let path = temp_path("missing");
    std::fs::remove_dir_all(path.parent().unwrap()).ok(); // make sure it really doesn't exist
    let store = Store::at(&path);
    let values = store.load().unwrap();
    assert!(values.is_empty());
}

#[test]
fn legacy_v1_wifi_key_is_migrated_on_load() {
    let path = temp_path("migration");
    let mut entries = BTreeMap::new();
    entries.insert("wifi.enabled".to_string(), "bool:true".to_string());
    writer::write(&path, &RawDocument { version: 1, entries }).unwrap();

    let store = Store::at(&path);
    let values = store.load().unwrap();
    assert_eq!(values.get("network.wifi_enabled"), Some(&Value::Bool(true)));
    assert!(!values.contains_key("wifi.enabled"));

    // The raw file on disk is untouched by a read-only `load()` -- only a
    // `Store::save()` after this would rewrite it at the current version.
    let raw_on_disk = loader::load(&path).unwrap();
    assert_eq!(raw_on_disk.version, 1);

    std::fs::remove_dir_all(path.parent().unwrap()).ok();
}

#[test]
fn writer_produces_atomic_no_partial_files() {
    let path = temp_path("atomic");
    let doc = RawDocument { version: loader::CURRENT_VERSION, entries: Default::default() };
    writer::write(&path, &doc).unwrap();
    assert!(path.exists());
    assert!(!path.with_extension("tmp").exists(), "temp file should be renamed away, not left behind");
    std::fs::remove_dir_all(path.parent().unwrap()).ok();
}
