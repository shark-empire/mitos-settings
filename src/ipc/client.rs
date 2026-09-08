use super::protocol::{Request, Response};
use std::io::BufReader;
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::Duration;

pub struct IpcClient;

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
        let mut stream = connect()?; // your existing socket-connect helper
        Request::ChangePassword {
            username: username.to_string(),
            new_password: new_password.to_string(),
        }
        .write_to(&stream)
        .map_err(|e| format!("could not send request: {e}"))?;

        match Response::read_from(std::io::BufReader::new(&stream))
            .map_err(|e| format!("could not read response: {e}"))?
        {
            Response::Ok(_) => Ok(()),
            Response::Err(e) => Err(e),
            other => Err(format!("unexpected daemon response: {other:?}")),
        }
    }
}
