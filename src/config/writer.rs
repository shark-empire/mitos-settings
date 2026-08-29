//! Counterpart to `config::loader`: writes a `RawDocument` back out.
//! Writes go to a temp file and are then renamed into place, so a crash or
//! power loss mid-write can never leave a half-written, corrupt config file.

use super::loader::RawDocument;
use std::fs;
use std::io::{self, Write};
use std::path::Path;

pub fn write(path: &Path, doc: &RawDocument) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp_path = path.with_extension("tmp");
    {
        let mut f = fs::File::create(&tmp_path)?;
        writeln!(f, "__version__={}", doc.version)?;
        for (k, v) in &doc.entries {
            writeln!(f, "{k}={v}")?;
        }
        f.sync_all()?;
    }
    restrict_permissions(&tmp_path)?;
    fs::rename(&tmp_path, path)
}

#[cfg(unix)]
fn restrict_permissions(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = fs::metadata(path)?.permissions();
    perms.set_mode(0o600);
    fs::set_permissions(path, perms)
}

#[cfg(not(unix))]
fn restrict_permissions(_path: &Path) -> io::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::loader;

    #[test]
    fn round_trips_through_load() {
        let dir = std::env::temp_dir().join(format!("mitos-writer-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.conf");

        let mut doc = RawDocument { version: 2, entries: Default::default() };
        doc.entries.insert("sound.volume".to_string(), "int:65".to_string());
        write(&path, &doc).unwrap();

        let reloaded = loader::load(&path).unwrap();
        assert_eq!(reloaded.version, 2);
        assert_eq!(reloaded.entries.get("sound.volume").unwrap(), "int:65");
        fs::remove_dir_all(&dir).ok();
    }
}
