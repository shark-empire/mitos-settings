use crate::categories::Category;
use crate::permissions::PrivilegeLevel;
use crate::settings::schema::{Schema, SettingSpec};
use crate::settings::value::{Value, ValueKind};

pub struct UpdatesCategory;

impl Category for UpdatesCategory {
    fn id(&self) -> &'static str {
        "updates"
    }
    fn name(&self) -> &'static str {
        "Updates"
    }
    fn icon(&self) -> &'static str {
        "system-software-update"
    }
    fn subitems(&self) -> &'static [&'static str] {
        &["System updates", "Security updates", "Update preferences"]
    }

    fn register(&self, schema: &mut Schema) {
        schema.register(SettingSpec::new(
            "updates.auto_check",
            "updates",
            "Check for updates automatically",
            "Periodically check for available package updates",
            ValueKind::Bool,
            Value::Bool(true),
            PrivilegeLevel::User,
        ));

        schema.register(SettingSpec::new(
            "updates.auto_install_security",
            "updates",
            "Auto-install security updates",
            "Install security patches as soon as they're available",
            ValueKind::Bool,
            Value::Bool(true),
            PrivilegeLevel::Admin,
        ));

        schema.register(
            SettingSpec::new(
                "updates.channel",
                "updates",
                "Update channel",
                "Which release channel to track",
                ValueKind::Str,
                Value::Str("stable".into()),
                PrivilegeLevel::Admin,
            )
            .choices(&["stable", "beta"]),
        );
    }

    fn live_info(&self) -> Vec<(&'static str, String)> {
        let mut rows = Vec::new();
        match crate::services::updates::detect() {
            Some(pm) => rows.push(("package_manager", format!("{pm:?}"))),
            None => rows.push(("package_manager", "not detected".to_string())),
        }
        match crate::services::updates::check_pending() {
            Some(count) => rows.push(("pending_updates", count.to_string())),
            None => rows.push(("pending_updates", "unavailable".to_string())),
        }
        rows
    }
}
