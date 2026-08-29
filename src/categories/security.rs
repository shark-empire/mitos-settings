use crate::categories::Category;
use crate::permissions::PrivilegeLevel;
use crate::settings::schema::{Schema, SettingSpec};
use crate::settings::value::{Value, ValueKind};
use std::fs;
use std::process::Command;

pub struct SecurityCategory;

impl Category for SecurityCategory {
    fn id(&self) -> &'static str {
        "security"
    }
    fn name(&self) -> &'static str {
        "Security"
    }
    fn icon(&self) -> &'static str {
        "security-high"
    }
    fn subitems(&self) -> &'static [&'static str] {
        &["Firewall", "Disk encryption status", "Secure boot status", "Login security", "Security updates"]
    }

    fn register(&self, schema: &mut Schema) {
        schema.register(
            SettingSpec::new(
                "security.firewall_status",
                "security",
                "Firewall status",
                "Read-only summary; toggle the firewall itself under Network",
                ValueKind::Str,
                Value::Str("unknown".into()),
                PrivilegeLevel::Admin,
            )
            .read_only(),
        );

        schema.register(
            SettingSpec::new(
                "security.disk_encryption_status",
                "security",
                "Disk encryption",
                "Whether the root filesystem is encrypted at rest",
                ValueKind::Str,
                Value::Str("unknown".into()),
                PrivilegeLevel::Admin,
            )
            .read_only(),
        );

        schema.register(
            SettingSpec::new(
                "security.secure_boot_status",
                "security",
                "Secure boot",
                "Whether UEFI Secure Boot is currently enforced",
                ValueKind::Str,
                Value::Str("unknown".into()),
                PrivilegeLevel::Admin,
            )
            .read_only(),
        );

        schema.register(SettingSpec::new(
            "security.require_password_immediately",
            "security",
            "Require password immediately",
            "No grace period before the lock screen demands a password",
            ValueKind::Bool,
            Value::Bool(true),
            PrivilegeLevel::Admin,
        ));

        schema.register(SettingSpec::new(
            "security.automatic_security_updates",
            "security",
            "Automatic security updates",
            "Install security patches without waiting for manual approval",
            ValueKind::Bool,
            Value::Bool(true),
            PrivilegeLevel::Admin,
        ));
    }

    fn live_info(&self) -> Vec<(&'static str, String)> {
        vec![
            ("firewall", firewall_status()),
            ("secure_boot", secure_boot_status()),
            ("disk_encryption", disk_encryption_status()),
        ]
    }
}

/// Best-effort check for a known firewall service being active under
/// systemd. Doesn't require root: `systemctl is-active` is readable by any
/// user.
fn firewall_status() -> String {
    for svc in ["ufw", "firewalld", "nftables"] {
        if let Ok(out) = Command::new("systemctl").args(["is-active", svc]).output() {
            if String::from_utf8_lossy(&out.stdout).trim() == "active" {
                return format!("{svc} active");
            }
        }
    }
    "no known firewall service active (or systemd not in use)".to_string()
}

/// Reads the EFI `SecureBoot` variable exposed under sysfs. No root
/// required — this file is world-readable.
fn secure_boot_status() -> String {
    let entries = match fs::read_dir("/sys/firmware/efi/efivars") {
        Ok(e) => e,
        Err(_) => return "not applicable (legacy BIOS boot)".to_string(),
    };
    let Some(var) = entries.flatten().find(|e| e.file_name().to_string_lossy().starts_with("SecureBoot-")) else {
        return "unknown".to_string();
    };
    match fs::read(var.path()) {
        // The first 4 bytes are EFI variable attributes; byte 5 is the value.
        Ok(bytes) if bytes.len() > 4 => {
            if bytes[4] == 1 { "enabled".to_string() } else { "disabled".to_string() }
        }
        _ => "unknown".to_string(),
    }
}

/// Best-effort check for LUKS-encrypted block devices via `/proc/mounts` +
/// `lsblk`, without requiring root.
fn disk_encryption_status() -> String {
    let has_crypt_mount = fs::read_to_string("/proc/mounts")
        .map(|m| m.lines().any(|l| l.contains("/dev/mapper/") && l.starts_with("/dev/mapper")))
        .unwrap_or(false);
    if has_crypt_mount {
        "likely encrypted (dm-crypt mapping active)".to_string()
    } else {
        "no dm-crypt mapping detected".to_string()
    }
}
