//! The shape of a grant, mirroring mitos-service's own `rulebook::Grant`
//! and `risk::Risk` exactly -- deliberately not this crate's own
//! invention anymore. See the `grants` module doc comment for why: this
//! used to be a local guess (an `app_id`/`app_name` pair this crate made
//! up, a three-tier risk table this crate maintained by hand), and
//! that guess is gone. What's here now is just enough structure to
//! parse mitos-service's wire responses into and render them from.

use std::fmt;

/// mitos-service identifies an app by the SHA-256 of its binary, not a
/// name -- see that project's `rulebook.rs`. There is no display name
/// anywhere in this data; `service_client::compute_sha256` is how the
/// GUI gets one to grant in the first place, by hashing a file the
/// person picks, and the hash itself (usually shortened for display)
/// is all there ever is to show for an existing grant.
pub type Sha256Hex = String;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    Allow,
    Deny,
}

impl Decision {
    pub fn as_str(self) -> &'static str {
        match self {
            Decision::Allow => "allow",
            Decision::Deny => "deny",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "allow" => Some(Decision::Allow),
            "deny" => Some(Decision::Deny),
            _ => None,
        }
    }
}

impl fmt::Display for Decision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// Consumed by the very next `CHECK` for the same pair, whatever
    /// the result -- an unusual choice to set up in advance from
    /// Settings (it exists mainly for a live, in-the-moment prompt
    /// flow this project doesn't build), but mitos-service accepts it
    /// here too, so the GUI exposes it rather than silently hiding an
    /// option mitos-service itself supports.
    Once,
    /// Lasts until mitos-service restarts.
    Session,
    /// Persisted; survives a restart until explicitly revoked or the
    /// binary's hash changes.
    Always,
}

impl Scope {
    pub fn as_str(self) -> &'static str {
        match self {
            Scope::Once => "once",
            Scope::Session => "session",
            Scope::Always => "always",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "once" => Some(Scope::Once),
            "session" => Some(Scope::Session),
            "always" => Some(Scope::Always),
            _ => None,
        }
    }
}

impl fmt::Display for Scope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Mirrors mitos-service's `risk::Risk` -- classified *by mitos-service*
/// (`risk::classify`), reported alongside each grant in `LIST-RAW`, not
/// computed here. This crate keeps no table of its own to classify
/// capabilities from; that was exactly the duplicate-source-of-truth
/// problem this rewrite exists to remove. `Low`/`Moderate` are ever only
/// display info here (mitos-service applies those without elevating);
/// `Dangerous`/`Critical` are what mitos-service itself blocks a
/// `GRANT` on until mitos-session verifies a password.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Risk {
    Low,
    Moderate,
    Dangerous,
    Critical,
}

impl Risk {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "low" => Some(Risk::Low),
            "moderate" => Some(Risk::Moderate),
            "dangerous" => Some(Risk::Dangerous),
            "critical" => Some(Risk::Critical),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Risk::Low => "low",
            Risk::Moderate => "moderate",
            Risk::Dangerous => "dangerous",
            Risk::Critical => "critical",
        }
    }
}

impl fmt::Display for Risk {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One row of mitos-service's rulebook, as reported by `LIST-RAW`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Grant {
    pub sha256: Sha256Hex,
    pub capability: String,
    pub decision: Decision,
    pub scope: Scope,
    /// Unix seconds, as mitos-service's own `rulebook::Grant` stores it.
    pub granted_at: u64,
    pub risk: Risk,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decision_scope_and_risk_all_round_trip_through_their_string_form() {
        for d in [Decision::Allow, Decision::Deny] {
            assert_eq!(Decision::parse(d.as_str()), Some(d));
        }
        for s in [Scope::Once, Scope::Session, Scope::Always] {
            assert_eq!(Scope::parse(s.as_str()), Some(s));
        }
        for r in [Risk::Low, Risk::Moderate, Risk::Dangerous, Risk::Critical] {
            assert_eq!(Risk::parse(r.as_str()), Some(r));
        }
    }

    #[test]
    fn unrecognized_strings_parse_to_none_rather_than_guessing() {
        assert_eq!(Decision::parse("maybe"), None);
        assert_eq!(Scope::parse("forever"), None);
        assert_eq!(Risk::parse("extreme"), None);
    }
}
