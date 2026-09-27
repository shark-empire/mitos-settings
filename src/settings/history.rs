//! A bounded, append-only log of setting changes: what changed, to what,
//! and when. `SettingsManager::set_with_context` appends one line here
//! right after a write actually lands, for every successful `set` --
//! direct or daemon-forwarded, from any caller. Deliberately separate
//! from `notifications::EventBus`: the event bus is in-process and
//! best-effort, with nothing subscribed to it unless something's actively
//! listening (the GUI doesn't, as of this writing); this writes straight
//! to disk, so `mitos-settings history` has something to show even if
//! nothing was watching when the change happened.
//!
//! One line per entry: `<unix seconds> <key> <encoded value>`, reusing
//! `Value::encode`/`decode` -- the same one-line, newline-free encoding
//! `persistence::Store` already uses for the same data, rather than
//! inventing a second encoding for it. The key never contains a space, so
//! `<key>` and `<encoded value>` split unambiguously even though an
//! encoded `Str`/`StrList` value legally can (see `parse_line`).

use crate::settings::value::Value;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

/// Kept small on purpose -- this is "what changed recently," not a full
/// audit trail. Oldest entries are dropped once the log passes this many
/// lines; see `append`.
const MAX_ENTRIES: usize = 500;

#[derive(Debug, Clone, PartialEq)]
pub struct HistoryEntry {
    pub timestamp_unix: u64,
    pub key: String,
    pub value: Value,
}

/// Appends one entry to the log at `path`, then trims it back down to
/// `MAX_ENTRIES` if that pushed it over. Best-effort: a write failure here
/// (a read-only filesystem, a full disk) is reported to stderr and
/// otherwise ignored rather than turned into a `SettingsError` -- losing
/// a history line is a much smaller problem than failing the settings
/// change that's actually being recorded alongside it.
pub fn append(path: &Path, key: &str, value: &Value) {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let line = format!("{now} {key} {}\n", value.encode());

    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let result = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .and_then(|mut f| f.write_all(line.as_bytes()));

    match result {
        Ok(()) => trim(path, MAX_ENTRIES),
        Err(e) => eprintln!(
            "mitos-settings: couldn't record history for '{key}' in {}: {e}",
            path.display()
        ),
    }
}

/// Every entry currently in the log, oldest first. Skips a malformed line
/// rather than failing the whole read -- the same way `config::loader`
/// skips a malformed config line -- since a history file is diagnostic,
/// not load-bearing, and one bad line shouldn't hide the rest. Returns an
/// empty log (not an error) if the file doesn't exist yet, which is the
/// normal state before the first setting change of any given scope.
pub fn read_all(path: &Path) -> Vec<HistoryEntry> {
    let Ok(file) = std::fs::File::open(path) else {
        return Vec::new();
    };
    BufReader::new(file)
        .lines()
        .map_while(Result::ok)
        .filter_map(|line| parse_line(&line))
        .collect()
}

/// The most recent `count` entries, newest first -- what `mitos-settings
/// history` actually shows.
pub fn recent(path: &Path, count: usize) -> Vec<HistoryEntry> {
    let mut entries = read_all(path);
    entries.reverse();
    entries.truncate(count);
    entries
}

fn parse_line(line: &str) -> Option<HistoryEntry> {
    let mut parts = line.splitn(3, ' ');
    let timestamp_unix = parts.next()?.parse::<u64>().ok()?;
    let key = parts.next()?.to_string();
    let value = Value::decode(parts.next()?).ok()?;
    Some(HistoryEntry {
        timestamp_unix,
        key,
        value,
    })
}

fn trim(path: &Path, max_entries: usize) {
    let entries = read_all(path);
    if entries.len() <= max_entries {
        return;
    }
    let mut out = String::new();
    for e in &entries[entries.len() - max_entries..] {
        out.push_str(&format!("{} {} {}\n", e.timestamp_unix, e.key, e.value.encode()));
    }
    let _ = std::fs::write(path, out);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_path(name: &str) -> PathBuf {
        let n = std::time::SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "mitos-settings-history-test-{}-{name}-{n}",
            std::process::id()
        ))
    }

    #[test]
    fn append_then_read_all_round_trips_in_order() {
        let path = temp_path("roundtrip");
        append(&path, "sound.volume", &Value::Int(42));
        append(&path, "network.wifi_enabled", &Value::Bool(true));

        let entries = read_all(&path);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].key, "sound.volume");
        assert_eq!(entries[0].value, Value::Int(42));
        assert_eq!(entries[1].key, "network.wifi_enabled");
        assert_eq!(entries[1].value, Value::Bool(true));

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn recent_returns_newest_first_and_respects_count() {
        let path = temp_path("recent");
        append(&path, "a", &Value::Int(1));
        append(&path, "b", &Value::Int(2));
        append(&path, "c", &Value::Int(3));

        let last_two = recent(&path, 2);
        assert_eq!(last_two.len(), 2);
        assert_eq!(last_two[0].key, "c");
        assert_eq!(last_two[1].key, "b");

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn a_value_containing_spaces_round_trips() {
        let path = temp_path("spaces");
        append(&path, "test.key", &Value::Str("hello world".to_string()));

        let entries = read_all(&path);
        assert_eq!(entries[0].value, Value::Str("hello world".to_string()));

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn a_strlist_value_round_trips() {
        let path = temp_path("strlist");
        append(
            &path,
            "language.keyboard_layouts",
            &Value::StrList(vec!["us".into(), "de".into()]),
        );

        let entries = read_all(&path);
        assert_eq!(
            entries[0].value,
            Value::StrList(vec!["us".into(), "de".into()])
        );

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn reading_a_missing_file_returns_an_empty_log_not_an_error() {
        let path = temp_path("missing");
        assert!(read_all(&path).is_empty());
    }

    #[test]
    fn trim_keeps_only_the_most_recent_entries() {
        let path = temp_path("trim");
        for i in 0..5 {
            append(&path, "test.counter", &Value::Int(i));
        }
        trim(&path, 3);

        let entries = read_all(&path);
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].value, Value::Int(2));
        assert_eq!(entries[2].value, Value::Int(4));

        std::fs::remove_file(&path).ok();
    }
}
