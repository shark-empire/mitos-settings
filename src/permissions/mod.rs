//! Everything to do with *who is allowed to do what*: privilege levels
//! (`policy`), identifying the caller (`authorization`), and escalating a
//! single command to root when running outside the daemon (`privileged`).

pub mod authorization;
pub mod policy;
pub mod privileged;

pub use authorization::{context_for_uid, current_context, AuthContext};
pub use policy::{describe, satisfies, PrivilegeLevel};
