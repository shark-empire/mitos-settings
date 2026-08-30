//! Centralizes every filesystem path the app touches, following the XDG
//! base directory conventions for user-scope files.

use std::env;
use std::path::PathBuf;

/// `$XDG_CONFIG_HOME/mitos-settings`, or `~/.config/mitos-settings`.
pub fn user_config_dir() -> PathBuf {
    if let Ok(xdg) = env::var("XDG_CONFIG_HOME") {
        if !xdg.is_empty() {
            return PathBuf::from(xdg).join("mitos-settings");
        }
    }
    let home = env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(home).join(".config").join("mitos-settings")
}

pub fn user_config_path() -> PathBuf {
    user_config_dir().join("settings.conf")
}

/// System-wide store, only writable by root / the daemon.
pub fn system_config_dir() -> PathBuf {
    PathBuf::from("/etc/mitos-settings")
}

pub fn system_config_path() -> PathBuf {
    system_config_dir().join("settings.conf")
}

pub fn runtime_dir() -> PathBuf {
    if let Ok(dir) = env::var("XDG_RUNTIME_DIR") {
        if !dir.is_empty() {
            return PathBuf::from(dir);
        }
    }
    PathBuf::from("/run")
}

/// Well-known location for the privileged daemon's Unix socket. Fixed
/// (rather than under a per-user runtime dir) because the daemon is a
/// single, system-wide, root-owned process — see docs/security.md.
pub fn daemon_socket_path() -> PathBuf {
    PathBuf::from("/run/mitos-settings/daemon.sock")
}

/// Shared desktop-shell config directory, `$XDG_CONFIG_HOME/mitos` (falls
/// back to `~/.config/mitos`) — distinct from mitos-settings' own config
/// dir (`user_config_dir`, above). This is where `home.conf` lives: a
/// small, other-programs-readable projection of a handful of settings that
/// mitos-gui and mitos-file-manager watch directly via inotify. See
/// docs/home-conf.md.
pub fn mitos_shared_config_dir() -> PathBuf {
    if let Ok(xdg) = env::var("XDG_CONFIG_HOME") {
        if !xdg.is_empty() {
            return PathBuf::from(xdg).join("mitos");
        }
    }
    let home = env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(home).join(".config").join("mitos")
}

pub fn home_conf_path() -> PathBuf {
    mitos_shared_config_dir().join("home.conf")
}
