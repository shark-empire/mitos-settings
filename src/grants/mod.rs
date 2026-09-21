//! Per-app permission grants: which apps (identified by the SHA-256 of
//! their binary) may use which capabilities, each Allow/Deny with a
//! scope of Once/Session/Always -- the piece the original MITOS
//! permission design describes as this project's job: "see every app,
//! see its permissions, and flip toggles."
//!
//! **This module is now a thin client of mitos-service, and nothing
//! more.** An earlier version of this feature kept its own local copy
//! of the rulebook (a file this daemon wrote and read itself) with its
//! own guessed risk-classification table, built before mitos-service
//! existed as a reachable project. That's gone. mitos-service is the
//! single rulebook every MITOS component looks to -- see that
//! project's own README -- and this module exists only to talk to it
//! (`service_client`) and hold just enough shape (`model`) to parse
//! what it says back. If mitos-service is unreachable, this feature
//! simply doesn't work right now (a clear error, not a silent local
//! fallback that could drift from the real rulebook) -- see
//! `docs/security.md`'s Permission grants section.
//!
//! One consequence worth knowing up front: mitos-service has no
//! concept of an app's display name, only its binary hash. There is no
//! "Video Editor" anywhere in this data -- see `model`'s doc comment on
//! `Sha256Hex`.

pub mod model;
pub mod service_client;

pub use model::{Decision, Grant, Risk, Scope, Sha256Hex};
pub use service_client::{CheckResult, GrantResult};
