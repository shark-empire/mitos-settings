//! Typed persistence for a settings store. Wraps `config::loader`/`writer`
//! (which know nothing about `Value`) and `config::migration`, converting
//! to and from `Value::encode`/`decode`.
//!
//! There are two `Store`s in play at runtime — see `settings::manager` —
//! one for user-scope settings, one for the system-wide (root-owned) store.

use crate::config::{loader, migration, paths, writer};
use crate::settings::value::Value;
use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};

pub struct Store {
    path: PathBuf,
}

impl Store {
    pub fn user() -> Self {
        Store { path: paths::user_config_path() }
    }

    pub fn system() -> Self {
        Store { path: paths::system_config_path() }
    }

    pub fn at(path: impl Into<PathBuf>) -> Self {
        Store { path: path.into() }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn load(&self) -> io::Result<HashMap<String, Value>> {
        let raw = loader::load(&self.path)?;
        let raw = migration::migrate(raw);
        let mut values = HashMap::with_capacity(raw.entries.len());
        for (key, encoded) in raw.entries {
            match Value::decode(&encoded) {
                Ok(v) => {
                    values.insert(key, v);
                }
                Err(err) => {
                    eprintln!(
                        "mitos-settings: skipping unreadable entry '{key}' in {}: {err}",
                        self.path.display()
                    );
                }
            }
        }
        Ok(values)
    }

    pub fn save(&self, values: &HashMap<String, Value>) -> io::Result<()> {
        let mut doc = loader::RawDocument { version: loader::CURRENT_VERSION, entries: Default::default() };
        for (key, value) in values {
            doc.entries.insert(key.clone(), value.encode());
        }
        writer::write(&self.path, &doc)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_then_load_round_trips_values() {
        let dir = std::env::temp_dir().join(format!("mitos-persistence-test-{}", std::process::id()));
        let store = Store::at(dir.join("settings.conf"));

        let mut values = HashMap::new();
        values.insert("display.brightness".to_string(), Value::Int(80));
        values.insert("network.wifi_enabled".to_string(), Value::Bool(true));
        store.save(&values).unwrap();

        let reloaded = store.load().unwrap();
        assert_eq!(reloaded.get("display.brightness"), Some(&Value::Int(80)));
        assert_eq!(reloaded.get("network.wifi_enabled"), Some(&Value::Bool(true)));

        std::fs::remove_dir_all(dir).ok();
    }
}
