use crate::categories::Category;
use crate::permissions::PrivilegeLevel;
use crate::settings::schema::{Schema, SettingSpec};
use crate::settings::value::{Value, ValueKind};

pub struct NotificationsCategory;

impl Category for NotificationsCategory {
    fn id(&self) -> &'static str {
        "notifications"
    }
    fn name(&self) -> &'static str {
        "Notifications"
    }
    fn icon(&self) -> &'static str {
        "preferences-system-notifications"
    }
    fn subitems(&self) -> &'static [&'static str] {
        &["Do Not Disturb", "Application notifications", "Notification sounds"]
    }

    fn register(&self, schema: &mut Schema) {
        schema.register(SettingSpec::new(
            "notifications.do_not_disturb",
            "notifications",
            "Do Not Disturb",
            "Suppress notification banners and sounds",
            ValueKind::Bool,
            Value::Bool(false),
            PrivilegeLevel::User,
        ));

        schema.register(SettingSpec::new(
            "notifications.show_on_lock_screen",
            "notifications",
            "Show on lock screen",
            "Display notification banners while the screen is locked",
            ValueKind::Bool,
            Value::Bool(false),
            PrivilegeLevel::User,
        ));

        schema.register(SettingSpec::new(
            "notifications.sounds_enabled",
            "notifications",
            "Notification sounds",
            "Play a sound when a notification arrives",
            ValueKind::Bool,
            Value::Bool(true),
            PrivilegeLevel::User,
        ));

        schema.register(
            SettingSpec::new(
                "notifications.banner_style",
                "notifications",
                "Banner style",
                "How application notifications are presented",
                ValueKind::Str,
                Value::Str("banner".into()),
                PrivilegeLevel::User,
            )
            .choices(&["banner", "alert", "none"]),
        );
    }
}
