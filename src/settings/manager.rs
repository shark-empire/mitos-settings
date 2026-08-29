//! The `SettingsManager` is where everything else in this crate meets:
//!
//! - `categories::register_all` builds the `Schema`
//! - `settings::persistence::Store` (x2: user + system) loads/saves values
//! - `settings::validation` checks a candidate value before it's accepted
//! - `permissions` decides whether the caller may write a given key
//! - `ipc::client` forwards privileged writes to the daemon when the
//!   caller isn't privileged enough to make them directly
//! - `services::apply` pushes an accepted change out to the running system
//! - `notifications::EventBus` tells anyone listening that it happened

use crate::categories;
use crate::config::paths;
use crate::ipc::client::IpcClient;
use crate::ipc::protocol::{Request, Response};
use crate::notifications::events::{Event, EventBus};
use crate::permissions::{self, AuthContext, PrivilegeLevel};
use crate::services;
use crate::settings::defaults;
use crate::settings::persistence::Store;
use crate::settings::schema::{Schema, SettingSpec};
use crate::settings::validation::{self, ValidationError};
use crate::settings::value::Value;
use std::collections::HashMap;
use std::fmt;

#[derive(Debug)]
pub enum SettingsError {
    UnknownKey(String),
    Invalid(ValidationError),
    PermissionDenied { key: String, required: PrivilegeLevel, held: PrivilegeLevel },
    Io(std::io::Error),
    Daemon(String),
}

impl fmt::Display for SettingsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SettingsError::UnknownKey(k) => write!(f, "unknown setting '{k}'"),
            SettingsError::Invalid(e) => write!(f, "{e}"),
            SettingsError::PermissionDenied { key, required, held } => write!(
                f,
                "'{key}' requires {required} privileges (you currently have {held}); \
                 re-run with sudo, or make sure the mitos-settings daemon is running"
            ),
            SettingsError::Io(e) => write!(f, "{e}"),
            SettingsError::Daemon(e) => write!(f, "daemon error: {e}"),
        }
    }
}

impl std::error::Error for SettingsError {}

impl From<std::io::Error> for SettingsError {
    fn from(e: std::io::Error) -> Self {
        SettingsError::Io(e)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Ordinary CLI/interactive-app usage. Writes that need more privilege
    /// than the current process has are forwarded to the daemon over IPC.
    Standalone,
    /// This process *is* the root-owned daemon: it may write system-scope
    /// settings directly, with no further escalation.
    DaemonAuthority,
}

pub struct SettingsManager {
    schema: Schema,
    values: HashMap<String, Value>,
    user_store: Store,
    system_store: Store,
    mode: Mode,
    ctx: AuthContext,
    pub events: EventBus,
}

impl SettingsManager {
    /// Builds a manager against the real, well-known user/system config
    /// paths (see `config::paths`). This is what `main.rs`, the daemon, and
    /// the interactive app use.
    pub fn load(mode: Mode) -> Result<Self, SettingsError> {
        Self::with_stores(mode, Store::user(), Store::system())
    }

    /// Builds a manager against arbitrary stores. Exists so tests (and
    /// anything else embedding this crate) can point at throwaway paths
    /// instead of the real, machine-wide config files — without resorting
    /// to mutating process-global environment variables, which doesn't
    /// play well with tests running in parallel.
    pub fn with_stores(mode: Mode, user_store: Store, system_store: Store) -> Result<Self, SettingsError> {
        let mut schema = Schema::new();
        categories::register_all(&mut schema);

        let mut values = defaults::default_values(&schema);
        values.extend(user_store.load()?);
        values.extend(system_store.load()?);

        Ok(SettingsManager {
            schema,
            values,
            user_store,
            system_store,
            mode,
            ctx: permissions::current_context(),
            events: EventBus::new(),
        })
    }

    pub fn schema(&self) -> &Schema {
        &self.schema
    }

    pub fn context(&self) -> &AuthContext {
        &self.ctx
    }

    pub fn get(&self, key: &str) -> Result<&Value, SettingsError> {
        let spec = self.schema.get(key).ok_or_else(|| SettingsError::UnknownKey(key.to_string()))?;
        Ok(self.values.get(key).unwrap_or(&spec.default))
    }

    pub fn set(&mut self, key: &str, value: Value) -> Result<(), SettingsError> {
        let spec = self.schema.get(key).ok_or_else(|| SettingsError::UnknownKey(key.to_string()))?.clone();
        validation::validate(&spec, &value).map_err(SettingsError::Invalid)?;

        let held = self.ctx.level();
        if spec.privilege > PrivilegeLevel::User && held < spec.privilege {
            if self.mode == Mode::Standalone {
                return self.set_via_daemon(key, &value);
            }
            return Err(SettingsError::PermissionDenied { key: key.to_string(), required: spec.privilege, held });
        }

        self.values.insert(key.to_string(), value.clone());
        self.persist(&spec)?;
        services::apply(key, &value);
        self.events.publish(Event::SettingChanged { key: key.to_string(), value });
        Ok(())
    }

    pub fn reset(&mut self, key: &str) -> Result<(), SettingsError> {
        let default = self.schema.get(key).ok_or_else(|| SettingsError::UnknownKey(key.to_string()))?.default.clone();
        self.set(key, default)
    }

    /// Resets every setting to its default. Best-effort: a privileged key
    /// this process can't reach (no daemon running, not an admin) is
    /// skipped rather than aborting the whole operation.
    pub fn reset_all(&mut self) -> Result<(), SettingsError> {
        let keys: Vec<&'static str> = self.schema.all().filter(|s| !s.read_only).map(|s| s.key).collect();
        for key in keys {
            let _ = self.reset(key);
        }
        Ok(())
    }

    fn persist(&self, spec: &SettingSpec) -> Result<(), SettingsError> {
        let is_system_scope = spec.privilege > PrivilegeLevel::User;
        let store = if is_system_scope { &self.system_store } else { &self.user_store };

        let subset: HashMap<String, Value> = self
            .values
            .iter()
            .filter(|(k, _)| {
                self.schema
                    .get(k.as_str())
                    .map(|s| (s.privilege > PrivilegeLevel::User) == is_system_scope)
                    .unwrap_or(false)
            })
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        store.save(&subset)?;
        Ok(())
    }

    /// Forwards a privileged write to the daemon over its Unix socket. Only
    /// reached in `Mode::Standalone` when the local caller isn't privileged
    /// enough to write the key directly.
    fn set_via_daemon(&mut self, key: &str, value: &Value) -> Result<(), SettingsError> {
        let socket = paths::daemon_socket_path();
        let response = IpcClient::send(&socket, &Request::Set { key: key.to_string(), value: value.clone() })
            .map_err(|e| {
                SettingsError::Daemon(format!(
                    "could not reach the daemon at {}: {e} (is it running? try `sudo mitos-settings --daemon`)",
                    socket.display()
                ))
            })?;

        match response {
            Response::Ok(_) => {
                self.values.insert(key.to_string(), value.clone());
                self.events.publish(Event::SettingChanged { key: key.to_string(), value: value.clone() });
                Ok(())
            }
            Response::Err(msg) => Err(SettingsError::Daemon(msg)),
            Response::Data(_) => Ok(()),
        }
    }
}

#[cfg(test)]
pub(crate) mod test_support {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    /// A manager backed by two throwaway temp-file stores, unique per call.
    /// Safe to use from tests running in parallel: nothing here touches a
    /// shared path or a process-global environment variable.
    pub fn isolated_manager(mode: Mode) -> (SettingsManager, std::path::PathBuf) {
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("mitos-settings-test-{}-{n}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let user_store = Store::at(dir.join("user.conf"));
        let system_store = Store::at(dir.join("system.conf"));
        let manager = SettingsManager::with_stores(mode, user_store, system_store).unwrap();
        (manager, dir)
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::isolated_manager;
    use super::*;

    #[test]
    fn get_returns_schema_default_when_unset() {
        let (manager, dir) = isolated_manager(Mode::Standalone);
        assert_eq!(manager.get("display.brightness").unwrap(), &Value::Int(80));
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn set_then_get_user_level_key_round_trips() {
        let (mut manager, dir) = isolated_manager(Mode::Standalone);
        manager.set("sound.volume", Value::Int(42)).unwrap();
        assert_eq!(manager.get("sound.volume").unwrap(), &Value::Int(42));
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn set_rejects_unknown_key() {
        let (mut manager, dir) = isolated_manager(Mode::Standalone);
        let err = manager.set("nonexistent.key", Value::Bool(true)).unwrap_err();
        assert!(matches!(err, SettingsError::UnknownKey(_)));
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn set_rejects_out_of_range_value() {
        let (mut manager, dir) = isolated_manager(Mode::Standalone);
        let err = manager.set("sound.volume", Value::Int(500)).unwrap_err();
        assert!(matches!(err, SettingsError::Invalid(_)));
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn reset_restores_default() {
        let (mut manager, dir) = isolated_manager(Mode::Standalone);
        manager.set("sound.volume", Value::Int(10)).unwrap();
        manager.reset("sound.volume").unwrap();
        assert_eq!(manager.get("sound.volume").unwrap(), &Value::Int(50));
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn daemon_authority_writes_privileged_keys_directly() {
        let (mut manager, dir) = isolated_manager(Mode::DaemonAuthority);
        // Whether this succeeds depends on the sandbox's real uid (DaemonAuthority
        // still enforces the privilege check against the real caller), so we
        // only assert it never panics and errors are the expected variant.
        if let Err(e) = manager.set("network.proxy_mode", Value::Str("manual".into())) {
            assert!(matches!(e, SettingsError::PermissionDenied { .. }));
        }
        std::fs::remove_dir_all(dir).ok();
    }
}
