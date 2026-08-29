use crate::categories::Category;
use crate::permissions::PrivilegeLevel;
use crate::settings::schema::{Schema, SettingSpec};
use crate::settings::value::{Value, ValueKind};

pub struct DeveloperCategory;

impl Category for DeveloperCategory {
    fn id(&self) -> &'static str {
        "developer"
    }
    fn name(&self) -> &'static str {
        "Developer"
    }
    fn icon(&self) -> &'static str {
        "applications-development"
    }
    fn subitems(&self) -> &'static [&'static str] {
        &["Developer mode", "Logs", "Diagnostics", "Debugging"]
    }

    fn register(&self, schema: &mut Schema) {
        schema.register(SettingSpec::new(
            "developer.mode_enabled",
            "developer",
            "Developer mode",
            "Unlock developer-only features (local root shell, unsigned packages, ...)",
            ValueKind::Bool,
            Value::Bool(false),
            PrivilegeLevel::Admin,
        ));

        schema.register(SettingSpec::new(
            "developer.diagnostics_enabled",
            "developer",
            "Diagnostics",
            "Collect crash reports and diagnostic data locally",
            ValueKind::Bool,
            Value::Bool(false),
            PrivilegeLevel::User,
        ));

        schema.register(SettingSpec::new(
            "developer.debug_logging",
            "developer",
            "Debug logging",
            "Increase log verbosity across system services",
            ValueKind::Bool,
            Value::Bool(false),
            PrivilegeLevel::User,
        ));
    }

    fn live_info(&self) -> Vec<(&'static str, String)> {
        vec![("log_directory", "/var/log/mitos".to_string())]
    }
}
