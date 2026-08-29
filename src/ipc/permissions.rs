//! The daemon's Unix socket is the actual access-control boundary: only
//! processes whose user can read/write `daemon.sock` (mode 0660 — see
//! `ipc::server::bind` and docs/security.md) can reach these handlers at
//! all. There is no per-request peer-credential check here, which is a
//! known, documented simplification for a reference implementation (see
//! docs/security.md for the hardening path via `SO_PEERCRED`).
//!
//! What this module *does* add is a defense-in-depth check: even a client
//! that reached the socket cannot have the daemon apply a `Root`-level
//! setting unless the daemon process itself is actually running as root.

use crate::permissions::{current_context, PrivilegeLevel};
use crate::settings::schema::SettingSpec;

pub fn ensure_daemon_may_apply(spec: &SettingSpec) -> Result<(), String> {
    if spec.privilege == PrivilegeLevel::Root && current_context().uid != 0 {
        return Err(format!(
            "'{}' requires the daemon to run as root; start it via systemd or `sudo mitos-settings --daemon`",
            spec.key
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::schema::SettingSpec;
    use crate::settings::value::{Value, ValueKind};

    #[test]
    fn admin_level_settings_are_never_blocked_here() {
        let spec = SettingSpec::new(
            "sharing.file_sharing_enabled",
            "sharing",
            "File sharing",
            "desc",
            ValueKind::Bool,
            Value::Bool(false),
            PrivilegeLevel::Admin,
        );
        assert!(ensure_daemon_may_apply(&spec).is_ok());
    }
}
