//! Defines *how much* privilege an action needs. This module has no
//! knowledge of settings at all — `settings::schema` depends on it, not the
//! other way around — so it can be reused anywhere an action needs a
//! coarse-grained privilege check (IPC requests, CLI commands, ...).

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PrivilegeLevel {
    /// Anything a signed-in user may change about their own session.
    User = 0,
    /// Requires administrator rights (sudo/wheel group membership, or a
    /// polkit-style prompt) because it affects the whole machine.
    Admin = 1,
    /// Requires the action to actually run as uid 0 — reserved for the
    /// handful of settings where "administrator" isn't a strong enough
    /// guarantee (e.g. writing files outside any user's control).
    Root = 2,
}

impl fmt::Display for PrivilegeLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            PrivilegeLevel::User => "user",
            PrivilegeLevel::Admin => "admin",
            PrivilegeLevel::Root => "root",
        };
        write!(f, "{s}")
    }
}

/// True if someone authorized at `held` may perform an action that requires
/// `required`. Levels are totally ordered, so this is just `>=`.
pub fn satisfies(held: PrivilegeLevel, required: PrivilegeLevel) -> bool {
    held >= required
}

/// Human-readable explanation of what a level means, used in CLI/app error
/// messages when a permission check fails.
pub fn describe(level: PrivilegeLevel) -> &'static str {
    match level {
        PrivilegeLevel::User => "any signed-in user",
        PrivilegeLevel::Admin => "an administrator (sudo/wheel/mitos-admin group member)",
        PrivilegeLevel::Root => "root, or the mitos-settings daemon acting on root's behalf",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordering_is_transitive() {
        assert!(PrivilegeLevel::Root > PrivilegeLevel::Admin);
        assert!(PrivilegeLevel::Admin > PrivilegeLevel::User);
        assert!(satisfies(PrivilegeLevel::Root, PrivilegeLevel::Admin));
        assert!(!satisfies(PrivilegeLevel::User, PrivilegeLevel::Admin));
    }
}
