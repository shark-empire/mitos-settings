//! Tests for `permissions`: privilege-level ordering and `AuthContext`
//! derivation. Doesn't assert a specific uid/username (that depends on
//! whatever environment the test happens to run in) — just that the
//! machinery behaves consistently.

use mitos_settings::permissions::{self, current_context, satisfies, AuthContext, PrivilegeLevel};

#[test]
fn privilege_levels_are_totally_ordered() {
    assert!(PrivilegeLevel::Root > PrivilegeLevel::Admin);
    assert!(PrivilegeLevel::Admin > PrivilegeLevel::User);
    assert!(PrivilegeLevel::Root > PrivilegeLevel::User);
}

#[test]
fn satisfies_matches_ordering() {
    assert!(satisfies(PrivilegeLevel::Root, PrivilegeLevel::Admin));
    assert!(satisfies(PrivilegeLevel::Admin, PrivilegeLevel::Admin));
    assert!(!satisfies(PrivilegeLevel::User, PrivilegeLevel::Admin));
}

#[test]
fn auth_context_level_is_consistent_with_its_fields() {
    let root = AuthContext { uid: 0, username: "root".into(), is_admin: true };
    assert_eq!(root.level(), PrivilegeLevel::Root);

    let admin = AuthContext { uid: 1000, username: "amy".into(), is_admin: true };
    assert_eq!(admin.level(), PrivilegeLevel::Admin);

    let user = AuthContext { uid: 1000, username: "amy".into(), is_admin: false };
    assert_eq!(user.level(), PrivilegeLevel::User);
}

#[test]
fn current_context_never_panics_and_produces_a_level() {
    let ctx = current_context();
    // Whatever this sandbox's uid actually is, level() should resolve
    // without panicking, and describe() should have something to say
    // about it.
    let level = ctx.level();
    assert!(!permissions::describe(level).is_empty());
}
