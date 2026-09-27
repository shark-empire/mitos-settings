//! A deliberately simple, line-oriented text protocol between CLI/app
//! clients and the privileged daemon. No external serialization crate: the
//! request/response space is small enough that hand-rolled framing is
//! easier to audit than pulling in serde+a wire format for five message
//! kinds.
//!
//! Requests: `GET <key>` / `SET <key> <encoded-value>` / `LIST [<category>]`
//! / `RESET <key|--all>` / `PING` / `WHOAMI` / a multi-line `SETMANY`
//! (one `SET`-shaped row per pair, `END`-terminated) / `SUBSCRIBE`
//!
//! Responses: `OK <message>` / `ERR <message>` / a multi-line `OK` header
//! followed by `DATA <key>=<value>` rows and a terminating `END` (used for
//! `LIST`).
//!
//! `SUBSCRIBE` is different from every other request: the daemon never
//! sends it a `Response` at all. Instead the connection stays open and
//! the daemon pushes an unsolicited `ChangeNotice` (`CHANGED
//! <key>=<encoded-value>`) every time any connection's write succeeds,
//! for as long as the peer stays connected. See `ChangeNotice` and
//! `ipc::server::handle_subscriber`.

use crate::grants::{Decision, Grant, Risk, Scope};
use crate::settings::value::Value;
use std::io::{self, BufRead, Write};

#[derive(Debug, Clone)]
pub enum Request {
    Get {
        key: String,
    },
    Set {
        key: String,
        value: Value,
    },
    List {
        category: Option<String>,
    },
    /// `None` means "reset every setting".
    Reset {
        key: Option<String>,
    },
    Ping,
    /// Diagnostic: ask the daemon who it thinks is asking, per
    /// `SO_PEERCRED`. Mostly useful for confirming peer-credential
    /// resolution actually works end to end — see `ipc::permissions`.
    WhoAmI,

    /// Change a user's password. Requires root privileges.
    ChangePassword {
        username: String,
        new_password: String,
    },

    /// List every grant currently held by mitos-service, the single
    /// rulebook owner — see the `grants` module doc comment. Not
    /// paginated or filtered; the rulebook isn't expected to be huge.
    ListGrants,
    /// Ask mitos-service to record a decision for `sha256`/
    /// `capability`. No `uid` field here: the daemon supplies the
    /// *connecting peer's own* uid (via `SO_PEERCRED`, the same
    /// identity every other privileged write in this protocol is
    /// authorized against) — a person can only ever grant something
    /// against their own mitos-session session, never someone else's,
    /// through this client. If mitos-service classifies
    /// `capability` as `Dangerous`/`Critical`, the daemon's reply is
    /// deferred until that person answers a real elevation prompt (or
    /// declines it, or it times out) — see `grants::service_client::grant`.
    SetGrant {
        sha256: String,
        capability: String,
        decision: Decision,
        scope: Scope,
    },
    /// Ask mitos-service to remove whatever decision exists for
    /// `sha256`/`capability`, regardless of its scope. Revoking is
    /// never dangerous the way granting is (see mitos-service's own
    /// `ipc.rs`), so this never triggers elevation.
    RevokeGrant {
        sha256: String,
        capability: String,
    },

    /// Apply every `(key, value)` pair atomically: the daemon validates
    /// the whole batch before applying any of it, the same
    /// validate-then-apply shape `SettingsManager::import_values` already
    /// uses locally -- see `SettingsManager::import_values_for_peer`.
    /// Exists because a batch sent as separate `Set` requests can apply
    /// some and fail partway if one fails after others already
    /// succeeded; see README's "Known gaps".
    SetMany { pairs: Vec<(String, Value)> },

    /// Puts this connection into subscribe mode. After this, the daemon
    /// expects no further requests on it and instead pushes an
    /// unsolicited `ChangeNotice` (`CHANGED <key>=<encoded-value>`)
    /// every time any connection's write succeeds -- this connection's
    /// own writes included. Lets a long-running client (the GUI, mainly)
    /// reflect a setting changed by another process without polling.
    /// `ipc::client::subscribe` is the client-side counterpart; nothing
    /// currently mixes this with ordinary requests on the same
    /// connection, and the daemon doesn't support that.
    Subscribe,
}

#[derive(Debug, Clone)]
pub enum Response {
    Ok(String),
    Err(String),
    Data(Vec<(String, String)>),
    /// Reply to `ListGrants` — kept as its own variant rather than
    /// shoehorned into `Data`'s flat `key=value` rows, since a `Grant`
    /// has more fields than that shape carries.
    Grants(Vec<Grant>),
}

fn encode_risk(risk: Risk) -> &'static str {
    risk.as_str()
}

fn decode_risk(s: &str) -> Option<Risk> {
    Risk::parse(s)
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
            // ADD THIS:
            Request::ChangePassword {
                username,
                new_password,
            } => {
                writeln!(w, "CHANGEPASSWORD")?;
                writeln!(w, "{}", username)?;
                writeln!(w, "{}", new_password)
            }

            Request::ListGrants => writeln!(w, "LISTGRANTS"),
            Request::SetGrant {
                sha256,
                capability,
                decision,
                scope,
            } => {
                writeln!(w, "SETGRANT")?;
                writeln!(w, "{sha256}")?;
                writeln!(w, "{capability}")?;
                writeln!(w, "{}", decision.as_str())?;
                writeln!(w, "{}", scope.as_str())
            }
            Request::RevokeGrant { sha256, capability } => {
                writeln!(w, "REVOKEGRANT")?;
                writeln!(w, "{sha256}")?;
                writeln!(w, "{capability}")
            }

            // Same "bare verb, then one row per pair, `END`-terminated"
            // shape `Response::Data`/`Response::Grants` already use --
            // each row reuses `SET`'s own `key value` framing so it
            // parses with the exact same line-splitting `read_from`
            // already does for a standalone `SET`.
            Request::SetMany { pairs } => {
                writeln!(w, "SETMANY")?;
                for (key, value) in pairs {
                    writeln!(w, "SET {key} {}", value.encode())?;
                }
                writeln!(w, "END")
            }

            Request::Subscribe => writeln!(w, "SUBSCRIBE"),
        }
    }

    pub fn read_from<R: BufRead>(mut r: R) -> io::Result<Request> {
        let mut line = String::new();
        r.read_line(&mut line)?;
        let line = line.trim();
        let mut parts = line.splitn(3, ' ');
        let verb = parts.next().unwrap_or("");
        match verb {
            "GET" => Ok(Request::Get {
                key: parts.next().unwrap_or("").to_string(),
            }),
            "SET" => {
                let key = parts.next().unwrap_or("").to_string();
                let value_raw = parts.next().unwrap_or("");
                let value = Value::decode(value_raw)
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
                Ok(Request::Set { key, value })
            }
            "LIST" => Ok(Request::List {
                category: parts.next().map(str::to_string).filter(|s| !s.is_empty()),
            }),
            "RESET" => {
                let arg = parts.next().unwrap_or("");
                if arg.is_empty() || arg == "--all" {
                    Ok(Request::Reset { key: None })
                } else {
                    Ok(Request::Reset {
                        key: Some(arg.to_string()),
                    })
                }
            }
            "PING" => Ok(Request::Ping),
            "WHOAMI" => Ok(Request::WhoAmI),

            "CHANGEPASSWORD" => {
                let mut username = String::new();
                r.read_line(&mut username)?;
                let username = username.trim_end().to_string();

                let mut new_password = String::new();
                r.read_line(&mut new_password)?;
                let new_password = new_password.trim_end().to_string();

                Ok(Request::ChangePassword {
                    username,
                    new_password,
                })
            }

            "LISTGRANTS" => Ok(Request::ListGrants),
            "SETGRANT" => {
                let mut sha256 = String::new();
                r.read_line(&mut sha256)?;
                let sha256 = sha256.trim_end().to_string();

                let mut capability = String::new();
                r.read_line(&mut capability)?;
                let capability = capability.trim_end().to_string();

                let mut decision_line = String::new();
                r.read_line(&mut decision_line)?;
                let decision = Decision::parse(decision_line.trim_end()).ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("unknown decision '{}'", decision_line.trim_end()),
                    )
                })?;

                let mut scope_line = String::new();
                r.read_line(&mut scope_line)?;
                let scope = Scope::parse(scope_line.trim_end()).ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("unknown scope '{}'", scope_line.trim_end()),
                    )
                })?;

                Ok(Request::SetGrant {
                    sha256,
                    capability,
                    decision,
                    scope,
                })
            }
            "REVOKEGRANT" => {
                let mut sha256 = String::new();
                r.read_line(&mut sha256)?;
                let sha256 = sha256.trim_end().to_string();

                let mut capability = String::new();
                r.read_line(&mut capability)?;
                let capability = capability.trim_end().to_string();

                Ok(Request::RevokeGrant { sha256, capability })
            }

            "SETMANY" => {
                let mut pairs = Vec::new();
                loop {
                    let mut row = String::new();
                    if r.read_line(&mut row)? == 0 {
                        // EOF mid-batch -- malformed, but return what
                        // parsed rather than hanging; the daemon will
                        // reject an empty/short batch on its own terms.
                        break;
                    }
                    let row = row.trim_end();
                    if row == "END" {
                        break;
                    }
                    let mut row_parts = row.splitn(3, ' ');
                    let row_verb = row_parts.next().unwrap_or("");
                    if row_verb != "SET" {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!("expected a SET row inside SETMANY, got '{row}'"),
                        ));
                    }
                    let key = row_parts.next().unwrap_or("").to_string();
                    let value_raw = row_parts.next().unwrap_or("");
                    let value = Value::decode(value_raw)
                        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
                    pairs.push((key, value));
                }
                Ok(Request::SetMany { pairs })
            }

            "SUBSCRIBE" => Ok(Request::Subscribe),

            other => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("unknown verb '{other}'"),
            )),
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
            Response::Grants(rows) => {
                writeln!(w, "GRANTS")?;
                for row in rows {
                    writeln!(
                        w,
                        "GRANT {}\u{1f}{}\u{1f}{}\u{1f}{}\u{1f}{}\u{1f}{}",
                        row.sha256,
                        row.capability,
                        row.decision.as_str(),
                        row.scope.as_str(),
                        row.granted_at,
                        encode_risk(row.risk),
                    )?;
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
        if first == "GRANTS" {
            let mut rows = Vec::new();
            loop {
                let mut line = String::new();
                if r.read_line(&mut line)? == 0 || line.trim_end() == "END" {
                    break;
                }
                let Some(rest) = line.trim_end().strip_prefix("GRANT ") else {
                    continue; // skip anything malformed rather than aborting the whole list
                };
                let fields: Vec<&str> = rest.split('\u{1f}').collect();
                if fields.len() != 6 {
                    continue; // skip anything malformed rather than aborting the whole list
                }
                let (Some(decision), Some(scope), Ok(granted_at), Some(risk)) = (
                    Decision::parse(fields[2]),
                    Scope::parse(fields[3]),
                    fields[4].parse::<u64>(),
                    decode_risk(fields[5]),
                ) else {
                    continue;
                };
                rows.push(Grant {
                    sha256: fields[0].to_string(),
                    capability: fields[1].to_string(),
                    decision,
                    scope,
                    granted_at,
                    risk,
                });
            }
            return Ok(Response::Grants(rows));
        }
        Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("malformed response '{first}'"),
        ))
    }
}

/// An unsolicited notification the daemon pushes to every connection in
/// subscribe mode (see `Request::Subscribe`) whenever a setting changes:
/// `CHANGED <key>=<encoded-value>`, one per line, with no `END`
/// terminator between them -- unlike `Response::Data`/`Grants`, this is
/// an open-ended stream rather than a single bounded reply. Not a
/// `Response` variant: nothing ever replies to a subscribed connection,
/// so it doesn't share `Response`'s `OK`/`ERR`/`DATA` framing.
#[derive(Debug, Clone, PartialEq)]
pub struct ChangeNotice {
    pub key: String,
    pub value: Value,
}

impl ChangeNotice {
    pub fn write_to<W: Write>(&self, mut w: W) -> io::Result<()> {
        writeln!(w, "CHANGED {}={}", self.key, self.value.encode())
    }

    /// `Ok(None)` means the connection closed (EOF) -- the ordinary way
    /// a subscription ends when the peer disconnects, not an error.
    pub fn read_from<R: BufRead>(mut r: R) -> io::Result<Option<ChangeNotice>> {
        let mut line = String::new();
        if r.read_line(&mut line)? == 0 {
            return Ok(None);
        }
        let line = line.trim_end();
        let rest = line.strip_prefix("CHANGED ").ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("expected a CHANGED line, got '{line}'"),
            )
        })?;
        // `split_once` (not `split`) so a `str`-kind value containing its
        // own `=` still decodes correctly -- same reasoning as
        // `Response::read_from`'s `DATA` row parsing.
        let (key, encoded) = rest.split_once('=').ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("malformed CHANGED line '{line}'"),
            )
        })?;
        let value =
            Value::decode(encoded).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        Ok(Some(ChangeNotice {
            key: key.to_string(),
            value,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn request_round_trips() {
        let requests = vec![
            Request::Get {
                key: "display.brightness".into(),
            },
            Request::Set {
                key: "sound.volume".into(),
                value: Value::Int(50),
            },
            Request::List {
                category: Some("network".into()),
            },
            Request::List { category: None },
            Request::Reset {
                key: Some("sound.volume".into()),
            },
            Request::Reset { key: None },
            Request::Ping,
            Request::WhoAmI,
            Request::ListGrants,
            Request::SetGrant {
                sha256: "a".repeat(64),
                capability: "raw_disk".into(),
                decision: Decision::Allow,
                scope: Scope::Always,
            },
            Request::RevokeGrant {
                sha256: "b".repeat(64),
                capability: "camera".into(),
            },
            Request::SetMany {
                pairs: vec![
                    ("sound.volume".into(), Value::Int(55)),
                    ("display.brightness".into(), Value::Int(80)),
                ],
            },
            Request::SetMany { pairs: vec![] },
            Request::Subscribe,
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
        let rows = vec![
            ("a".to_string(), "int:1".to_string()),
            ("b".to_string(), "bool:true".to_string()),
        ];
        let mut buf = Vec::new();
        Response::Data(rows.clone()).write_to(&mut buf).unwrap();
        let parsed = Response::read_from(Cursor::new(buf)).unwrap();
        match parsed {
            Response::Data(got) => assert_eq!(got, rows),
            other => panic!("expected Data, got {other:?}"),
        }
    }

    #[test]
    fn grants_response_round_trips() {
        let rows = vec![
            Grant {
                sha256: "a".repeat(64),
                capability: "raw_disk".into(),
                decision: Decision::Allow,
                scope: Scope::Always,
                granted_at: 1_700_000_000,
                risk: Risk::Critical,
            },
            Grant {
                sha256: "b".repeat(64),
                capability: "location".into(),
                decision: Decision::Deny,
                scope: Scope::Session,
                granted_at: 1_700_000_001,
                risk: Risk::Moderate,
            },
        ];
        let mut buf = Vec::new();
        Response::Grants(rows).write_to(&mut buf).unwrap();
        let parsed = Response::read_from(Cursor::new(buf)).unwrap();
        match parsed {
            Response::Grants(got) => {
                assert_eq!(got.len(), 2);
                assert_eq!(got[0].capability, "raw_disk");
                assert_eq!(got[0].decision, Decision::Allow);
                assert_eq!(got[1].scope, Scope::Session);
                assert_eq!(got[1].risk, Risk::Moderate);
            }
            other => panic!("expected Grants, got {other:?}"),
        }
    }

    #[test]
    fn an_empty_grants_list_round_trips_too() {
        let mut buf = Vec::new();
        Response::Grants(vec![]).write_to(&mut buf).unwrap();
        let parsed = Response::read_from(Cursor::new(buf)).unwrap();
        assert!(matches!(parsed, Response::Grants(rows) if rows.is_empty()));
    }

    #[test]
    fn change_notice_round_trips() {
        let notice = ChangeNotice {
            key: "sound.volume".into(),
            value: Value::Int(55),
        };
        let mut buf = Vec::new();
        notice.write_to(&mut buf).unwrap();
        let parsed = ChangeNotice::read_from(Cursor::new(buf)).unwrap();
        assert_eq!(parsed, Some(notice));
    }

    #[test]
    fn change_notice_round_trips_a_str_value_containing_its_own_equals_sign() {
        let notice = ChangeNotice {
            key: "network.proxy_mode".into(),
            value: Value::Str("http://user:pass@host:8080/?token=abc=def".into()),
        };
        let mut buf = Vec::new();
        notice.write_to(&mut buf).unwrap();
        let parsed = ChangeNotice::read_from(Cursor::new(buf)).unwrap();
        assert_eq!(parsed, Some(notice));
    }

    #[test]
    fn change_notice_read_from_returns_none_at_eof() {
        let parsed = ChangeNotice::read_from(Cursor::new(Vec::new())).unwrap();
        assert!(parsed.is_none());
    }
}
