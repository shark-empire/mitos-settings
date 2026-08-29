use crate::categories::Category;
use crate::permissions::PrivilegeLevel;
use crate::settings::schema::{Schema, SettingSpec};
use crate::settings::value::{Value, ValueKind};

pub struct PrivacyCategory;

impl Category for PrivacyCategory {
    fn id(&self) -> &'static str {
        "privacy"
    }
    fn name(&self) -> &'static str {
        "Privacy"
    }
    fn icon(&self) -> &'static str {
        "preferences-system-privacy"
    }
    fn subitems(&self) -> &'static [&'static str] {
        &["Location", "Camera", "Microphone", "Notifications", "Recent files", "Application permissions"]
    }

    fn register(&self, schema: &mut Schema) {
        schema.register(SettingSpec::new(
            "privacy.location_services_enabled",
            "privacy",
            "Location services",
            "Allow apps to request the device's location",
            ValueKind::Bool,
            Value::Bool(false),
            PrivilegeLevel::User,
        ));

        schema.register(SettingSpec::new(
            "privacy.camera_access_enabled",
            "privacy",
            "Camera access",
            "Allow apps to use the camera",
            ValueKind::Bool,
            Value::Bool(true),
            PrivilegeLevel::User,
        ));

        schema.register(SettingSpec::new(
            "privacy.microphone_access_enabled",
            "privacy",
            "Microphone access",
            "Allow apps to use the microphone",
            ValueKind::Bool,
            Value::Bool(true),
            PrivilegeLevel::User,
        ));

        schema.register(SettingSpec::new(
            "privacy.lock_screen_notifications",
            "privacy",
            "Notifications on lock screen",
            "Show notification content while the screen is locked",
            ValueKind::Bool,
            Value::Bool(false),
            PrivilegeLevel::User,
        ));

        schema.register(SettingSpec::new(
            "privacy.recent_files_tracking",
            "privacy",
            "Recent files",
            "Remember recently opened files and folders",
            ValueKind::Bool,
            Value::Bool(true),
            PrivilegeLevel::User,
        ));

        schema.register(SettingSpec::new(
            "privacy.prompt_for_app_permissions",
            "privacy",
            "Application permissions",
            "Ask for confirmation the first time an app requests a sensitive permission",
            ValueKind::Bool,
            Value::Bool(true),
            PrivilegeLevel::User,
        ));
    }
}
