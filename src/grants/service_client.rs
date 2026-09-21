//! A plain-text client for mitos-service's control socket
//! (`/run/mitos-service/control.sock`) -- see that project's `ipc.rs`
//! for the protocol itself (newline-delimited commands, one reply per
//! connection, then the server closes it). No serialization dependency
//! needed for this one, unlike mitos-session's bincode wire protocol:
//! it's plain text on both ends, so this crate stays fully
//! dependency-free even with mitos-service wired in -- see
//! `Cargo.toml`.

use crate::grants::model::{Decision, Grant, Risk, Scope};
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

fn socket_path() -> PathBuf {
    PathBuf::from("/run/mitos-service/control.sock")
}

/// `CHECK`'s three possible answers -- `Ask` carries the risk tier
/// mitos-service classified the capability at, purely for display; it
/// isn't this crate's call to make (see the `grants` module doc
/// comment).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckResult {
    Allow,
    Deny,
    Ask(Risk),
}

/// `GRANT`'s three possible outcomes -- see mitos-service's
/// `INTEGRATION.md` for why there are three distinct shapes here
/// rather than a plain `Result<(), String>`: `Denied` means elevation
/// genuinely ran and came back negative (wrong password, declined),
/// which a caller may well want to present differently from `Error`
/// (couldn't even get that far -- mitos-service unreachable, no
/// mitos-session session for this uid, bad arguments).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GrantResult {
    Granted,
    Denied(String),
}

fn connect(timeout: Duration) -> Result<UnixStream, String> {
    let stream = UnixStream::connect(socket_path())
        .map_err(|e| format!("could not reach mitos-service: {e}"))?;
    let _ = stream.set_read_timeout(Some(timeout));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(10)));
    Ok(stream)
}

/// Sends `command` (without its trailing newline) and reads the whole
/// reply -- mitos-service closes the connection after responding, the
/// same one-shot shape `mitosvc-ctl` itself relies on, so reading to
/// EOF is always correct here, never a hang waiting for more that
/// isn't coming.
fn send(command: &str, timeout: Duration) -> Result<String, String> {
    let mut stream = connect(timeout)?;
    stream
        .write_all(format!("{command}\n").as_bytes())
        .map_err(|e| format!("could not send to mitos-service: {e}"))?;
    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .map_err(|e| format!("could not read from mitos-service: {e}"))?;
    Ok(response)
}

const SHORT_TIMEOUT: Duration = Duration::from_secs(5);
// Comfortably past mitos-session's own prompt-timeout default (120s),
// so a slow-but-healthy elevation wait during GRANT is never mistaken
// for a hung connection -- see mitos-service's session_client.rs for
// the matching choice on the other end of this same wait.
const GRANT_TIMEOUT: Duration = Duration::from_secs(180);

pub fn ping() -> bool {
    matches!(send("PING", SHORT_TIMEOUT), Ok(r) if r.trim() == "PONG")
}

pub fn check(sha256: &str, capability: &str) -> Result<CheckResult, String> {
    let response = send(&format!("CHECK {sha256} {capability}"), SHORT_TIMEOUT)?;
    let line = response.trim();
    if line == "ALLOW" {
        return Ok(CheckResult::Allow);
    }
    if line == "DENY" {
        return Ok(CheckResult::Deny);
    }
    if let Some(risk_str) = line.strip_prefix("ASK ") {
        let risk = Risk::parse(risk_str)
            .ok_or_else(|| format!("mitos-service reported an unrecognized risk '{risk_str}'"))?;
        return Ok(CheckResult::Ask(risk));
    }
    Err(format!("unexpected reply to CHECK: {line}"))
}

/// Records `decision` for `sha256`/`capability` with the given
/// `scope`, on `uid`'s behalf. For a `Dangerous`/`Critical`
/// capability, **this blocks until `uid`'s own mitos-session session
/// answers a real elevation prompt** (or declines it, or it times
/// out) -- see mitos-service's `session_client.rs`. Callers on a GUI's
/// main/event thread must run this on a background thread instead,
/// the same requirement (and for the same reason) this crate's own
/// `gui/src/permissions_page.rs` already follows.
pub fn grant(
    sha256: &str,
    capability: &str,
    decision: Decision,
    scope: Scope,
    uid: u32,
) -> Result<GrantResult, String> {
    let response = send(
        &format!("GRANT {sha256} {capability} {} {} {uid}", decision.as_str(), scope.as_str()),
        GRANT_TIMEOUT,
    )?;
    let line = response.trim();
    if line == "granted" {
        return Ok(GrantResult::Granted);
    }
    if let Some(reason) = line.strip_prefix("denied: ") {
        return Ok(GrantResult::Denied(reason.to_string()));
    }
    if let Some(reason) = line.strip_prefix("error: ") {
        return Err(reason.to_string());
    }
    Err(format!("unexpected reply to GRANT: {line}"))
}

pub fn revoke(sha256: &str, capability: &str) -> Result<(), String> {
    let response = send(&format!("REVOKE {sha256} {capability}"), SHORT_TIMEOUT)?;
    if response.trim() == "revoked" {
        Ok(())
    } else {
        Err(format!("unexpected reply to REVOKE: {}", response.trim()))
    }
}

/// Every currently-held grant, via `LIST-RAW` (see mitos-service's
/// `ipc.rs` for why that's a separate command from the human-formatted
/// `LIST`). A line that doesn't parse as exactly six `|`-separated
/// fields, or whose decision/scope/risk isn't one this crate
/// recognizes, is skipped rather than aborting the whole list -- the
/// same "don't let one bad row take down the page" choice this
/// crate's earlier, now-removed local store made for its own file.
pub fn list() -> Result<Vec<Grant>, String> {
    let response = send("LIST-RAW", SHORT_TIMEOUT)?;
    let mut grants = Vec::new();
    for line in response.lines() {
        let fields: Vec<&str> = line.split('|').collect();
        if fields.len() != 6 {
            continue;
        }
        let (Some(decision), Some(scope), Ok(granted_at), Some(risk)) = (
            Decision::parse(fields[2]),
            Scope::parse(fields[3]),
            fields[4].parse::<u64>(),
            Risk::parse(fields[5]),
        ) else {
            continue;
        };
        grants.push(Grant {
            sha256: fields[0].to_string(),
            capability: fields[1].to_string(),
            decision,
            scope,
            granted_at,
            risk,
        });
    }
    Ok(grants)
}

/// Hashes the file at `path` by shelling out to the system `sha256sum`
/// rather than implementing or depending on a SHA-256 crate -- for a
/// one-off "hash a file someone picked" utility, reusing a
/// battle-tested coreutil already on virtually every Linux system
/// beats adding a cryptographic dependency (or hand-rolling one) to a
/// daemon that otherwise has none. Returns the lowercase hex digest
/// alone, `sha256sum`'s own leading column, with the filename column
/// it also prints discarded.
pub fn compute_sha256(path: &Path) -> Result<String, String> {
    let output = Command::new("sha256sum")
        .arg(path)
        .output()
        .map_err(|e| format!("could not run sha256sum: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "sha256sum failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout
        .split_whitespace()
        .next()
        .filter(|h| h.len() == 64 && h.bytes().all(|b| b.is_ascii_hexdigit()))
        .map(|h| h.to_lowercase())
        .ok_or_else(|| format!("could not parse sha256sum output: {stdout}"))
}
