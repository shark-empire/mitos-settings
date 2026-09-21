use super::protocol::{Request, Response};
use crate::grants::{Decision, Grant, Scope};
use std::io::BufReader;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub struct IpcClient;

// Helper to get the default socket path.
// IMPORTANT: Adjust this string if your daemon binds to a different location!
fn default_socket_path() -> PathBuf {
    PathBuf::from("/run/mitos-settings/daemon.sock")
}

impl IpcClient {
    /// Connects to `socket`, sends `request`, and waits for a response.
    /// A 5-second read timeout keeps a wedged daemon from hanging the CLI
    /// forever.
    pub fn send(socket: &Path, request: &Request) -> std::io::Result<Response> {
        Self::send_with_timeout(socket, request, Duration::from_secs(5))
    }

    /// Like `send`, but with an explicit timeout instead of the default
    /// 5 seconds. Needed for `set_grant` below: a `SetGrant` on a
    /// dangerous capability doesn't reply until a person answers a real
    /// mitos-session elevation prompt (relayed through mitos-service),
    /// which can legitimately take up to that project's own
    /// `[elevation].prompt_timeout_secs` (a couple of minutes by
    /// default) -- 5 seconds would time out a perfectly healthy
    /// exchange the moment someone pauses to read what they're
    /// approving.
    pub fn send_with_timeout(
        socket: &Path,
        request: &Request,
        timeout: Duration,
    ) -> std::io::Result<Response> {
        let stream = UnixStream::connect(socket)?;
        stream.set_read_timeout(Some(timeout))?;
        request.write_to(&stream)?;
        let reader = BufReader::new(stream);
        Response::read_from(reader)
    }

    /// Asks the privileged daemon to change `username`'s password.
    /// Returns the daemon's error message verbatim on failure so the GUI
    /// can display it (e.g. "Permission denied: can only change your own
    /// password").
    pub fn change_password(username: &str, new_password: &str) -> Result<(), String> {
        let socket = default_socket_path();
        let request = Request::ChangePassword {
            username: username.to_string(),
            new_password: new_password.to_string(),
        };

        match Self::send(&socket, &request) {
            Ok(Response::Ok(_)) => Ok(()),
            Ok(Response::Err(e)) => Err(e),
            Ok(other) => Err(format!("unexpected daemon response: {other:?}")),
            Err(e) => Err(format!("could not communicate with daemon: {e}")),
        }
    }

    /// Every grant currently held by mitos-service, the single
    /// rulebook owner -- see `crate::grants`.
    pub fn list_grants() -> Result<Vec<Grant>, String> {
        let socket = default_socket_path();
        match Self::send(&socket, &Request::ListGrants) {
            Ok(Response::Grants(rows)) => Ok(rows),
            Ok(Response::Err(e)) => Err(e),
            Ok(other) => Err(format!("unexpected daemon response: {other:?}")),
            Err(e) => Err(format!("could not communicate with daemon: {e}")),
        }
    }

    /// Records `decision` for `sha256`/`capability` with the given
    /// `scope`, against *the calling user's own* mitos-session session
    /// -- there's no way to grant on someone else's behalf through
    /// this client, see `protocol::Request::SetGrant`'s doc comment.
    /// **Callers on a GUI's main/event thread must run this on a
    /// background thread instead** -- for a dangerous capability this
    /// blocks for as long as it takes the person to answer
    /// mitos-session's elevation prompt (see `send_with_timeout`'s doc
    /// comment), and a frozen, unresponsive window for up to three
    /// minutes is a worse outcome than a moment of extra plumbing. See
    /// `gui/src/permissions_page.rs` for the pattern this crate's own
    /// GUI uses.
    pub fn set_grant(sha256: &str, capability: &str, decision: Decision, scope: Scope) -> Result<(), String> {
        let socket = default_socket_path();
        let request = Request::SetGrant {
            sha256: sha256.to_string(),
            capability: capability.to_string(),
            decision,
            scope,
        };
        // Comfortably past mitos-session's own prompt-timeout default
        // (120s) -- see mitos-service's `session_client::connect` for
        // the matching choice further down this same wait.
        match Self::send_with_timeout(&socket, &request, Duration::from_secs(180)) {
            Ok(Response::Ok(_)) => Ok(()),
            Ok(Response::Err(e)) => Err(e),
            Ok(other) => Err(format!("unexpected daemon response: {other:?}")),
            Err(e) => Err(format!("could not communicate with daemon: {e}")),
        }
    }

    /// Removes whatever grant exists for `sha256`/`capability`,
    /// whatever its scope. Never needs elevating (see mitos-service's
    /// own `ipc.rs`), so this always uses the short default timeout.
    pub fn revoke_grant(sha256: &str, capability: &str) -> Result<(), String> {
        let socket = default_socket_path();
        let request = Request::RevokeGrant {
            sha256: sha256.to_string(),
            capability: capability.to_string(),
        };
        match Self::send(&socket, &request) {
            Ok(Response::Ok(_)) => Ok(()),
            Ok(Response::Err(e)) => Err(e),
            Ok(other) => Err(format!("unexpected daemon response: {other:?}")),
            Err(e) => Err(format!("could not communicate with daemon: {e}")),
        }
    }
}
