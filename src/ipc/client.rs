use super::protocol::{Request, Response};
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
        let stream = UnixStream::connect(socket)?;
        stream.set_read_timeout(Some(Duration::from_secs(5)))?;
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
}
