//! `mitos-settings grants ...` -- a command-line surface for the same
//! three operations `gui/src/permissions_page.rs` already exposes
//! graphically: listing mitos-service's rulebook, adding a grant to it,
//! and revoking one. This talks to `IpcClient` directly, exactly like
//! the GUI does -- there's no `SettingsManager` involved anywhere here,
//! since grants aren't schema-backed settings (see the `grants` module
//! doc comment for why mitos-service, not this crate, owns that data).
//!
//! Blocking here is fine in a way it isn't for the GUI: `set`, for a
//! `Dangerous`/`Critical` capability, doesn't return until mitos-session's
//! elevation prompt is answered (see `IpcClient::set_grant`'s doc
//! comment), but a CLI process sitting there waiting on that is normal,
//! expected behavior -- there's no event loop here to freeze.

use crate::grants::{Decision, Scope};
use crate::ipc::IpcClient;

const USAGE: &str = "\
usage: mitos-settings grants <subcommand>

Subcommands:
  list
      List every grant mitos-service currently holds.
  set <sha256> <capability> <allow|deny> <once|session|always>
      Record a decision. Blocks on a real elevation prompt first if
      mitos-service classifies <capability> as Dangerous or Critical.
  revoke <sha256> <capability>
      Remove whatever grant exists for that pair, regardless of scope.";

pub fn execute(args: &[String]) -> Result<String, String> {
    match args.first().map(String::as_str) {
        Some("list") => list(),
        Some("set") => set(&args[1..]),
        Some("revoke") => revoke(&args[1..]),
        _ => Err(USAGE.to_string()),
    }
}

fn list() -> Result<String, String> {
    let grants = IpcClient::list_grants()?;
    if grants.is_empty() {
        return Ok("No grants recorded yet.\n".to_string());
    }

    let mut out = String::new();
    out.push_str(&format!(
        "{:<64} {:<24} {:<7} {:<8} {}\n",
        "SHA-256", "CAPABILITY", "DECISION", "SCOPE", "RISK"
    ));
    for grant in grants {
        out.push_str(&format!(
            "{:<64} {:<24} {:<7} {:<8} {}\n",
            grant.sha256,
            grant.capability,
            grant.decision.as_str(),
            grant.scope.as_str(),
            grant.risk.as_str(),
        ));
    }
    Ok(out)
}

fn set(args: &[String]) -> Result<String, String> {
    let [sha256, capability, decision, scope] = args else {
        return Err(
            "usage: mitos-settings grants set <sha256> <capability> <allow|deny> <once|session|always>"
                .to_string(),
        );
    };
    let decision = Decision::parse(decision)
        .ok_or_else(|| format!("'{decision}' is not 'allow' or 'deny'"))?;
    let scope = Scope::parse(scope)
        .ok_or_else(|| format!("'{scope}' is not 'once', 'session', or 'always'"))?;

    IpcClient::set_grant(sha256, capability, decision, scope).map(|()| {
        format!(
            "Recorded: {sha256} / {capability} -> {} ({}).\n",
            decision.as_str(),
            scope.as_str()
        )
    })
}

fn revoke(args: &[String]) -> Result<String, String> {
    let [sha256, capability] = args else {
        return Err("usage: mitos-settings grants revoke <sha256> <capability>".to_string());
    };
    IpcClient::revoke_grant(sha256, capability)
        .map(|()| format!("Revoked: {sha256} / {capability}.\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    // None of these touch the network: every case here fails argument
    // parsing/validation before ever reaching an `IpcClient` call, so
    // they're deterministic with no daemon required, matching how
    // `grants::service_client`'s own socket-touching functions are left
    // untested at the unit level (see that module) in favor of
    // `tests/ipc.rs` for anything that needs a live listener.

    #[test]
    fn no_subcommand_is_a_usage_error() {
        assert!(execute(&[]).is_err());
    }

    #[test]
    fn unknown_subcommand_is_a_usage_error() {
        assert!(execute(&["frobnicate".to_string()]).is_err());
    }

    #[test]
    fn set_rejects_wrong_argument_count_before_touching_the_network() {
        let err = execute(&["set".to_string(), "onlyonearg".to_string()]).unwrap_err();
        assert!(err.contains("usage"));
    }

    #[test]
    fn set_rejects_an_unrecognized_decision_before_touching_the_network() {
        let err = execute(&[
            "set".to_string(),
            "a".repeat(64),
            "camera".to_string(),
            "maybe".to_string(),
            "always".to_string(),
        ])
        .unwrap_err();
        assert!(err.contains("allow"));
    }

    #[test]
    fn set_rejects_an_unrecognized_scope_before_touching_the_network() {
        let err = execute(&[
            "set".to_string(),
            "a".repeat(64),
            "camera".to_string(),
            "allow".to_string(),
            "forever".to_string(),
        ])
        .unwrap_err();
        assert!(err.contains("once"));
    }

    #[test]
    fn revoke_rejects_wrong_argument_count_before_touching_the_network() {
        assert!(execute(&["revoke".to_string(), "onlyonearg".to_string()]).is_err());
    }
}
