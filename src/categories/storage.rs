use crate::categories::Category;
use crate::permissions::PrivilegeLevel;
use crate::settings::schema::{Schema, SettingSpec};
use crate::settings::value::{Value, ValueKind};

pub struct StorageCategory;

impl Category for StorageCategory {
    fn id(&self) -> &'static str {
        "storage"
    }
    fn name(&self) -> &'static str {
        "Storage"
    }
    fn icon(&self) -> &'static str {
        "drive-harddisk"
    }
    fn subitems(&self) -> &'static [&'static str] {
        &["Disks", "Partitions", "Storage usage", "Mounts"]
    }

    fn register(&self, schema: &mut Schema) {
        schema.register(SettingSpec::new(
            "storage.automount_removable_media",
            "storage",
            "Auto-mount removable media",
            "Automatically mount USB drives and other removable media when connected",
            ValueKind::Bool,
            Value::Bool(true),
            PrivilegeLevel::User,
        ));
    }

    fn live_info(&self) -> Vec<(&'static str, String)> {
        let mut rows = Vec::new();
        match crate::services::storage::disk_usage() {
            Ok(usages) => {
                for u in usages {
                    rows.push(("usage", format!("{} on {}: {}% used", u.filesystem, u.mount, u.used_percent)));
                }
            }
            Err(e) => rows.push(("usage", format!("could not read disk usage: {e}"))),
        }
        for m in crate::services::storage::list_mounts() {
            rows.push(("mount", format!("{} -> {} ({})", m.device, m.target, m.fstype)));
        }
        rows
    }
}
