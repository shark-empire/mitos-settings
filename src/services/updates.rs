use crate::platform;
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageManager {
    Apt,
    Dnf,
    Pacman,
    Zypper,
}

impl PackageManager {
    fn binary(self) -> &'static str {
        match self {
            PackageManager::Apt => "apt",
            PackageManager::Dnf => "dnf",
            PackageManager::Pacman => "pacman",
            PackageManager::Zypper => "zypper",
        }
    }
}

pub fn detect() -> Option<PackageManager> {
    [PackageManager::Apt, PackageManager::Dnf, PackageManager::Pacman, PackageManager::Zypper]
        .into_iter()
        .find(|pm| platform::command_exists(pm.binary()))
}

/// Read-only "how many updates are pending" check.
///
/// Deliberately *not* implemented: an `apply_updates()` that actually
/// upgrades packages. That's a privileged, potentially disruptive action
/// (it can restart services, need a reboot, or fail halfway through) and
/// deserves its own confirmation flow in a dedicated updater — not a bare
/// `mitos-settings set updates.foo true` call.
pub fn check_pending() -> Option<usize> {
    match detect()? {
        PackageManager::Apt => {
            let out = Command::new("apt").args(["list", "--upgradable"]).output().ok()?;
            Some(String::from_utf8_lossy(&out.stdout).lines().filter(|l| l.contains('/')).count())
        }
        PackageManager::Dnf => {
            let out = Command::new("dnf").arg("check-update").output().ok()?;
            Some(String::from_utf8_lossy(&out.stdout).lines().filter(|l| l.contains('.')).count())
        }
        PackageManager::Pacman => {
            let out = Command::new("pacman").arg("-Qu").output().ok()?;
            Some(String::from_utf8_lossy(&out.stdout).lines().count())
        }
        PackageManager::Zypper => {
            let out = Command::new("zypper").arg("list-updates").output().ok()?;
            Some(String::from_utf8_lossy(&out.stdout).lines().filter(|l| l.starts_with('v')).count())
        }
    }
}
