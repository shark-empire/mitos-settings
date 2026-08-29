//! Determines *who* is asking, so `policy::PrivilegeLevel` checks have
//! something to compare against.
//!
//! This deliberately shells out to the `id` coreutil rather than calling
//! `getuid(2)`/`getgroups(2)` through an FFI crate. `id` is present on every
//! Linux system (it's part of coreutils) and keeps this whole project
//! free of unsafe code and external dependencies.

use super::policy::PrivilegeLevel;
use crate::platform;

#[derive(Debug, Clone)]
pub struct AuthContext {
    pub uid: u32,
    pub username: String,
    pub is_admin: bool,
}

impl AuthContext {
    pub fn level(&self) -> PrivilegeLevel {
        if self.uid == 0 {
            PrivilegeLevel::Root
        } else if self.is_admin {
            PrivilegeLevel::Admin
        } else {
            PrivilegeLevel::User
        }
    }

    /// Fallback context used only if identity lookup fails outright (e.g.
    /// `id` is missing). Treated as the least-privileged possible caller.
    fn unknown() -> Self {
        AuthContext { uid: u32::MAX, username: "unknown".to_string(), is_admin: false }
    }
}

/// Inspects the identity of the process currently running mitos-settings.
pub fn current_context() -> AuthContext {
    let Some(uid) = run("id", &["-u"]).and_then(|s| s.trim().parse::<u32>().ok()) else {
        return AuthContext::unknown();
    };
    let username = run("id", &["-un"]).map(|s| s.trim().to_string()).unwrap_or_else(|| "unknown".into());
    let groups = run("id", &["-Gn"]).unwrap_or_default();
    let is_admin = uid == 0
        || groups
            .split_whitespace()
            .any(|g| matches!(g, "sudo" | "wheel" | "admin" | "mitos-admin"));

    AuthContext { uid, username, is_admin }
}

fn run(cmd: &str, args: &[&str]) -> Option<String> {
    platform::run_command(cmd, args).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_matches_uid_and_group() {
        let ctx = AuthContext { uid: 0, username: "root".into(), is_admin: true };
        assert_eq!(ctx.level(), PrivilegeLevel::Root);

        let ctx = AuthContext { uid: 1000, username: "amy".into(), is_admin: true };
        assert_eq!(ctx.level(), PrivilegeLevel::Admin);

        let ctx = AuthContext { uid: 1000, username: "amy".into(), is_admin: false };
        assert_eq!(ctx.level(), PrivilegeLevel::User);
    }

    #[test]
    fn current_context_returns_something_usable() {
        // We can't assert a specific uid in CI/sandbox environments, but the
        // lookup should never panic and should always produce a level.
        let ctx = current_context();
        let _ = ctx.level();
    }
}
