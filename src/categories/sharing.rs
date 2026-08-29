use crate::categories::Category;
use crate::permissions::PrivilegeLevel;
use crate::settings::schema::{Schema, SettingSpec};
use crate::settings::value::{Value, ValueKind};

pub struct SharingCategory;

impl Category for SharingCategory {
    fn id(&self) -> &'static str {
        "sharing"
    }
    fn name(&self) -> &'static str {
        "Sharing"
    }
    fn icon(&self) -> &'static str {
        "preferences-system-sharing"
    }
    fn subitems(&self) -> &'static [&'static str] {
        &["File sharing", "Screen sharing", "Remote access"]
    }

    fn register(&self, schema: &mut Schema) {
        // Each of these opens a network-facing service on the whole
        // machine, so all three require Admin even though they read like
        // simple toggles.
        schema.register(SettingSpec::new(
            "sharing.file_sharing_enabled",
            "sharing",
            "File sharing",
            "Share folders with other devices on the network",
            ValueKind::Bool,
            Value::Bool(false),
            PrivilegeLevel::Admin,
        ));

        schema.register(SettingSpec::new(
            "sharing.screen_sharing_enabled",
            "sharing",
            "Screen sharing",
            "Allow other devices to view or control this screen",
            ValueKind::Bool,
            Value::Bool(false),
            PrivilegeLevel::Admin,
        ));

        schema.register(SettingSpec::new(
            "sharing.remote_access_ssh_enabled",
            "sharing",
            "Remote access (SSH)",
            "Allow incoming SSH connections",
            ValueKind::Bool,
            Value::Bool(false),
            PrivilegeLevel::Admin,
        ));
    }
}
