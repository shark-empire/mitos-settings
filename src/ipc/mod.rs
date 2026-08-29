//! Client/server IPC over a Unix domain socket, used so unprivileged
//! callers (the CLI, the interactive app) can ask the privileged daemon to
//! apply Admin/Root-level settings on their behalf. See docs/security.md
//! for the trust model.

pub mod client;
pub mod permissions;
pub mod protocol;
pub mod server;

pub use client::IpcClient;
pub use protocol::{Request, Response};
pub use server::IpcServer;
