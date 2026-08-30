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
    let is_admin = is_admin_group_list(&groups) || uid == 0;

    AuthContext { uid, username, is_admin }
}

/// Resolves the identity of an *arbitrary* uid — not the current process.
/// This is what backs real per-connection authorization in the IPC daemon
/// (see `ipc::permissions::peer_credentials`): the daemon itself runs as
/// root, but a request arriving over the socket is made on behalf of
/// whoever the kernel says is on the other end, and that's a different uid
/// almost every time.
pub fn context_for_uid(uid: u32) -> AuthContext {
    if uid == 0 {
        return AuthContext { uid: 0, username: "root".to_string(), is_admin: true };
    }
    let username = username_for_uid(uid).unwrap_or_else(|| uid.to_string());
    let groups = run("id", &["-Gn", &username]).unwrap_or_default();
    AuthContext { uid, is_admin: is_admin_group_list(&groups), username }
}

fn is_admin_group_list(groups: &str) -> bool {
    groups.split_whitespace().any(|g| matches!(g, "sudo" | "wheel" | "admin" | "mitos-admin"))
}

/// Looks up a username by uid via `/etc/passwd` directly, rather than
/// another `id` call -- `id` has no "reverse lookup a bare uid to a name"
/// mode short of `getent passwd <uid>`, which isn't available on every
/// minimal system, whereas `/etc/passwd` always is.
fn username_for_uid(uid: u32) -> Option<String> {
    let content = std::fs::read_to_string("/etc/passwd").ok()?;
    content.lines().find_map(|line| {
        let mut fields = line.split(':');
        let name = fields.next()?;
        let _password = fields.next()?;
        let line_uid: u32 = fields.next()?.parse().ok()?;
        (line_uid == uid).then(|| name.to_string())
    })
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
