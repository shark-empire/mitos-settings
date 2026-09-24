//! A dependency-free advisory file lock, meant to serialize
//! `SettingsManager`'s read-modify-write cycle (see `settings::manager`'s
//! `persist`) *across processes* -- the in-process case (multiple IPC
//! requests hitting one running daemon) is already handled by
//! `Arc<Mutex<SettingsManager>>` in `ipc::server`; this is for the
//! separate case of, say, the GUI and a CLI `set` both touching
//! `settings.conf` at nearly the same moment. Not yet wired into
//! `persist` -- see this crate's README for why, and for the exact
//! integration point once that's wanted.
//!
//! Exclusive access is "I successfully created this `.lock` file":
//! `OpenOptions::create_new` is atomic at the filesystem level, so
//! exactly one caller -- across every process racing for the same path
//! -- will ever see it succeed. This is *not* a real OS-level lock
//! (`flock(2)`/`fcntl(2)` -- see below for why this crate doesn't reach
//! for those): most importantly, it isn't released automatically if the
//! holding process crashes mid-write. To cover that, the lock file's
//! contents are just the holding process's PID, and a caller that fails
//! to acquire the lock checks whether that PID is still alive (via
//! `/proc/<pid>`, so this is Linux-only, consistent with the rest of
//! this project) before deciding to wait -- a lock left behind by a dead
//! process is stolen immediately instead of waited on.
//!
//! **Why not a real OS lock:** `flock`/`fcntl` aren't exposed by `std`,
//! so using them means either the `fs2`/`fs4` crate or raw `libc`
//! bindings. This project is deliberately dependency-free (see
//! INTEGRATION.md's "On dependencies" section), so pulling one in is a
//! call for whoever owns this repo to make, not something to add
//! unilaterally. A real lock would also be released automatically on a
//! crash, which this can only approximate via the PID check above.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

/// Held for as long as this value is alive; dropping it removes the lock
/// file. Acquired by `acquire`.
pub struct FileLock {
    path: PathBuf,
}

impl Drop for FileLock {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

/// Waits (briefly, retrying every 20ms) until the lock at `path` can be
/// acquired, stealing it immediately if the process that currently holds
/// it is no longer running. Gives up and returns an error once `timeout`
/// has elapsed with the lock still held by a live process.
pub fn acquire(path: &Path, timeout: Duration) -> Result<FileLock, String> {
    let deadline = Instant::now() + timeout;
    loop {
        if try_create(path).is_ok() {
            return Ok(FileLock {
                path: path.to_path_buf(),
            });
        }

        if !holder_is_alive(path) {
            // Stale lock from a crashed process (or one that released it
            // in the instant between our failed create and this check) --
            // clear it and loop back to try creating it again right away,
            // instead of sleeping first for no reason.
            let _ = std::fs::remove_file(path);
            continue;
        }
        if Instant::now() >= deadline {
            return Err(format!(
                "couldn't acquire lock at {} within {timeout:?} -- another mitos-settings process is writing",
                path.display()
            ));
        }
        thread::sleep(Duration::from_millis(20));
    }
}

fn try_create(path: &Path) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    write!(file, "{}", std::process::id())
}

fn holder_is_alive(lock_path: &Path) -> bool {
    let Ok(contents) = std::fs::read_to_string(lock_path) else {
        return false;
    };
    let Ok(pid) = contents.trim().parse::<u32>() else {
        return false;
    };
    Path::new(&format!("/proc/{pid}")).exists()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(name: &str) -> PathBuf {
        let n = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "mitos-settings-lock-test-{}-{name}-{n}",
            std::process::id()
        ))
    }

    #[test]
    fn acquire_then_drop_releases_the_lock() {
        let path = temp_path("release");
        {
            let _lock = acquire(&path, Duration::from_millis(200)).unwrap();
            assert!(path.exists());
        }
        assert!(!path.exists());
    }

    #[test]
    fn a_second_acquire_waits_and_times_out_while_the_first_is_held() {
        let path = temp_path("contention");
        let _first = acquire(&path, Duration::from_millis(200)).unwrap();

        let start = Instant::now();
        let result = acquire(&path, Duration::from_millis(300));
        assert!(result.is_err());
        assert!(start.elapsed() >= Duration::from_millis(300));

        drop(_first);
    }

    #[test]
    fn a_lock_left_by_a_dead_pid_is_stolen_immediately_not_waited_out() {
        let path = temp_path("stale");
        // No real process will ever have this PID -- Linux's own pid_max
        // tops out far below it -- so this reliably simulates a lock file
        // left behind by a process that has since died, on any machine.
        std::fs::write(&path, "4000000000").unwrap();

        let start = Instant::now();
        let _lock = acquire(&path, Duration::from_secs(5)).unwrap();
        assert!(start.elapsed() < Duration::from_millis(500));
    }

    #[test]
    fn a_lock_file_with_unparseable_contents_is_also_treated_as_stale() {
        let path = temp_path("garbage");
        std::fs::write(&path, "not a pid").unwrap();

        let start = Instant::now();
        let _lock = acquire(&path, Duration::from_secs(5)).unwrap();
        assert!(start.elapsed() < Duration::from_millis(500));
    }
}
