//! A deliberately simple, line-oriented text protocol between CLI/app
//! clients and the privileged daemon. No external serialization crate: the
//! request/response space is small enough that hand-rolled framing is
//! easier to audit than pulling in serde+a wire format for five message
//! kinds.
//!
//! Requests: `GET <key>` / `SET <key> <encoded-value>` / `LIST [<category>]`
//! / `RESET <key|--all>` / `PING` / `WHOAMI`
//!
//! Responses: `OK <message>` / `ERR <message>` / a multi-line `OK` header
//! followed by `DATA <key>=<value>` rows and a terminating `END` (used for
//! `LIST`).

use crate::settings::value::Value;
use std::io::{self, BufRead, Write};

#[derive(Debug, Clone)]
pub enum Request {
    Get { key: String },
    Set { key: String, value: Value },
    List { category: Option<String> },
    /// `None` means "reset every setting".
    Reset { key: Option<String> },
    Ping,
    /// Diagnostic: ask the daemon who it thinks is asking, per
    /// `SO_PEERCRED`. Mostly useful for confirming peer-credential
    /// resolution actually works end to end — see `ipc::permissions`.
    WhoAmI,
}

#[derive(Debug, Clone)]
pub enum Response {
    Ok(String),
    Err(String),
    Data(Vec<(String, String)>),
}

impl Request {
    pub fn write_to<W: Write>(&self, mut w: W) -> io::Result<()> {
        match self {
            Request::Get { key } => writeln!(w, "GET {key}"),
            Request::Set { key, value } => writeln!(w, "SET {key} {}", value.encode()),
            Request::List { category } => match category {
                Some(c) => writeln!(w, "LIST {c}"),
                None => writeln!(w, "LIST"),
            },
            Request::Reset { key } => match key {
                Some(k) => writeln!(w, "RESET {k}"),
                None => writeln!(w, "RESET --all"),
            },
            Request::Ping => writeln!(w, "PING"),
            Request::WhoAmI => writeln!(w, "WHOAMI"),
        }
    }

    pub fn read_from<R: BufRead>(mut r: R) -> io::Result<Request> {
        let mut line = String::new();
        r.read_line(&mut line)?;
        let line = line.trim();
        let mut parts = line.splitn(3, ' ');
        let verb = parts.next().unwrap_or("");
        match verb {
            "GET" => Ok(Request::Get { key: parts.next().unwrap_or("").to_string() }),
            "SET" => {
                let key = parts.next().unwrap_or("").to_string();
                let value_raw = parts.next().unwrap_or("");
                let value = Value::decode(value_raw).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
                Ok(Request::Set { key, value })
            }
            "LIST" => Ok(Request::List { category: parts.next().map(str::to_string).filter(|s| !s.is_empty()) }),
            "RESET" => {
                let arg = parts.next().unwrap_or("");
                if arg.is_empty() || arg == "--all" {
                    Ok(Request::Reset { key: None })
                } else {
                    Ok(Request::Reset { key: Some(arg.to_string()) })
                }
            }
            "PING" => Ok(Request::Ping),
            "WHOAMI" => Ok(Request::WhoAmI),
            other => Err(io::Error::new(io::ErrorKind::InvalidData, format!("unknown verb '{other}'"))),
        }
    }
}

impl Response {
    pub fn write_to<W: Write>(&self, mut w: W) -> io::Result<()> {
        match self {
            Response::Ok(msg) => writeln!(w, "OK {msg}"),
            Response::Err(msg) => writeln!(w, "ERR {msg}"),
            Response::Data(rows) => {
                writeln!(w, "OK")?;
                for (k, v) in rows {
                    writeln!(w, "DATA {k}={v}")?;
                }
                writeln!(w, "END")
            }
        }
    }

    pub fn read_from<R: BufRead>(mut r: R) -> io::Result<Response> {
        let mut first = String::new();
        r.read_line(&mut first)?;
        let first = first.trim_end();

        if let Some(rest) = first.strip_prefix("OK ") {
            return Ok(Response::Ok(rest.to_string()));
        }
        if let Some(rest) = first.strip_prefix("ERR ") {
            return Ok(Response::Err(rest.to_string()));
        }
        if first == "OK" {
            let mut rows = Vec::new();
            loop {
                let mut line = String::new();
                if r.read_line(&mut line)? == 0 || line.trim_end() == "END" {
                    break;
                }
                if let Some(rest) = line.trim_end().strip_prefix("DATA ") {
                    if let Some((k, v)) = rest.split_once('=') {
                        rows.push((k.to_string(), v.to_string()));
                    }
                }
            }
            return Ok(Response::Data(rows));
        }
        Err(io::Error::new(io::ErrorKind::InvalidData, format!("malformed response '{first}'")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn request_round_trips() {
        let requests = vec![
            Request::Get { key: "display.brightness".into() },
            Request::Set { key: "sound.volume".into(), value: Value::Int(50) },
            Request::List { category: Some("network".into()) },
            Request::List { category: None },
            Request::Reset { key: Some("sound.volume".into()) },
            Request::Reset { key: None },
            Request::Ping,
            Request::WhoAmI,
        ];
        for req in requests {
            let mut buf = Vec::new();
            req.write_to(&mut buf).unwrap();
            let parsed = Request::read_from(Cursor::new(buf)).unwrap();
            // Compare via debug formatting since Request has no PartialEq.
            assert_eq!(format!("{req:?}"), format!("{parsed:?}"));
        }
    }

    #[test]
    fn ok_response_round_trips() {
        let mut buf = Vec::new();
        Response::Ok("applied".into()).write_to(&mut buf).unwrap();
        let parsed = Response::read_from(Cursor::new(buf)).unwrap();
        assert!(matches!(parsed, Response::Ok(s) if s == "applied"));
    }

    #[test]
    fn data_response_round_trips() {
        let rows = vec![("a".to_string(), "int:1".to_string()), ("b".to_string(), "bool:true".to_string())];
        let mut buf = Vec::new();
        Response::Data(rows.clone()).write_to(&mut buf).unwrap();
        let parsed = Response::read_from(Cursor::new(buf)).unwrap();
        match parsed {
            Response::Data(got) => assert_eq!(got, rows),
            other => panic!("expected Data, got {other:?}"),
        }
    }
}
