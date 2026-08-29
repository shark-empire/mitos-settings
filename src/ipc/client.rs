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
}
