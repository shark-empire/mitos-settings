//! Linux-specific low-level helpers. Everything in `hardware`/`services`/
//! `permissions` that needs to run a command or check one exists funnels
//! through here, so there's exactly one place that knows *how* a command
//! is actually invoked.

use std::collections::HashMap;
use std::fs;
use std::io;
use std::process::Command;

/// Runs `cmd` with `args` and returns its trimmed stdout if it exits
/// successfully.
pub fn run_command(cmd: &str, args: &[&str]) -> io::Result<String> {
    let output = Command::new(cmd).args(args).output()?;
    if !output.status.success() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("{cmd} exited with {}", output.status),
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// True if `bin` resolves on `$PATH`.
pub fn command_exists(bin: &str) -> bool {
    Command::new("which")
        .arg(bin)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Parses `/etc/os-release` (`KEY=value`, values optionally quoted) into a
/// map. Used by `categories::about` to show a friendly distro name.
pub fn os_release() -> HashMap<String, String> {
    let mut map = HashMap::new();
    let Ok(content) = fs::read_to_string("/etc/os-release") else {
        return map;
    };
    for line in content.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        map.insert(
            key.trim().to_string(),
            value.trim().trim_matches('"').to_string(),
        );
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_command_captures_stdout() {
        assert_eq!(run_command("echo", &["hello"]).unwrap(), "hello");
    }

    #[test]
    fn command_exists_finds_a_coreutil() {
        assert!(command_exists("ls"));
    }

    #[test]
    fn command_exists_rejects_bogus_binary() {
        assert!(!command_exists("definitely-not-a-real-binary-xyz"));
    }
}
