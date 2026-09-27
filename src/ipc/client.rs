use super::protocol::{ChangeNotice, Request, Response};
use crate::config::paths;
use crate::grants::{Decision, Grant, Scope};
use crate::settings::value::Value;
use std::io::BufReader;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Duration;

pub struct IpcClient;

// The daemon's socket path lives in one place, `config::paths`
// (`settings::manager` builds on it too, for `set_via_daemon`) -- this
// just forwards to it instead of keeping its own copy that could drift
// from the real one.
fn default_socket_path() -> PathBuf {
    paths::daemon_socket_path()
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
    pub fn set_grant(
        sha256: &str,
        capability: &str,
        decision: Decision,
        scope: Scope,
    ) -> Result<(), String> {
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

    /// Opens a dedicated, long-lived connection to `socket` and asks the
    /// daemon to push every future setting change over it (see
    /// `protocol::Request::Subscribe`), so a long-running client can
    /// reflect a change made by *another* process -- another
    /// `mitos-settings` invocation, another instance of the same GUI, or
    /// mitos-service -- without polling `GET`/`LIST`. Spawns a
    /// background thread that blocks reading the socket and forwards
    /// each parsed `(key, value)` pair into the returned channel; the
    /// thread ends (closing the channel) once the daemon connection
    /// drops, so callers should treat a closed `Receiver` as "not
    /// subscribed anymore," not as an error to propagate. This doesn't
    /// use `send`/`send_with_timeout`: those expect exactly one
    /// `Response`, and a subscription is an open-ended stream of
    /// `ChangeNotice`s instead. See `gui/src/main.rs` for how the GUI
    /// bridges this into GTK's own main loop.
    pub fn subscribe(socket: &Path) -> std::io::Result<Receiver<(String, Value)>> {
        let stream = UnixStream::connect(socket)?;
        Request::Subscribe.write_to(&stream)?;

        let (tx, rx) = mpsc::channel();
        let read_stream = stream.try_clone()?;
        thread::spawn(move || {
            let mut reader = BufReader::new(read_stream);
            while let Ok(Some(notice)) = ChangeNotice::read_from(&mut reader) {
                if tx.send((notice.key, notice.value)).is_err() {
                    break; // receiving end dropped -- caller stopped listening
                }
            }
        });

        Ok(rx)
    }
}
