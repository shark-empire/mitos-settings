//! Generic reader for the plain-text `key=value` format used by both the
//! user and system settings stores. Deliberately format-agnostic about what
//! `value` means — `settings::persistence` is what knows these strings are
//! `Value::encode()` output.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::Path;

/// Bump this whenever the on-disk key layout changes, and add a step to
/// `config::migration` to carry old files forward.
pub const CURRENT_VERSION: u32 = 2;

#[derive(Debug, Default, Clone)]
pub struct RawDocument {
    pub version: u32,
    pub entries: BTreeMap<String, String>,
}

/// Reads a config file. A missing file is not an error — it just means
/// "nothing has been customized yet" — and yields an empty, current-version
/// document.
pub fn load(path: &Path) -> io::Result<RawDocument> {
    let contents = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            return Ok(RawDocument { version: CURRENT_VERSION, entries: BTreeMap::new() })
        }
        Err(e) => return Err(e),
    };

    let mut doc = RawDocument { version: 1, entries: BTreeMap::new() };
    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else { continue };
        let key = key.trim();
        if key == "__version__" {
            doc.version = value.trim().parse().unwrap_or(1);
            continue;
        }
        doc.entries.insert(key.to_string(), value.to_string());
    }
    Ok(doc)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn missing_file_yields_empty_current_version_doc() {
        let doc = load(Path::new("/nonexistent/path/for/mitos/tests.conf")).unwrap();
        assert_eq!(doc.version, CURRENT_VERSION);
        assert!(doc.entries.is_empty());
    }

    #[test]
    fn parses_versioned_entries_and_skips_comments() {
        let dir = std::env::temp_dir().join(format!("mitos-loader-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.conf");
        let mut f = fs::File::create(&path).unwrap();
        writeln!(f, "__version__=2").unwrap();
        writeln!(f, "# a comment").unwrap();
        writeln!(f, "display.brightness=int:80").unwrap();
        drop(f);

        let doc = load(&path).unwrap();
        assert_eq!(doc.version, 2);
        assert_eq!(doc.entries.get("display.brightness").unwrap(), "int:80");
        fs::remove_dir_all(&dir).ok();
    }
}
