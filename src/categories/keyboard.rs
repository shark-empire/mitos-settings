use crate::categories::Category;
use crate::permissions::PrivilegeLevel;
use crate::settings::schema::{Schema, SettingSpec};
use crate::settings::value::{Value, ValueKind};

pub struct KeyboardCategory;

impl Category for KeyboardCategory {
    fn id(&self) -> &'static str {
        "keyboard"
    }
    fn name(&self) -> &'static str {
        "Keyboard"
    }
    fn icon(&self) -> &'static str {
        "input-keyboard"
    }
    fn subitems(&self) -> &'static [&'static str] {
        &["Layout", "Repeat rate", "Shortcuts", "Compose key"]
    }

    fn register(&self, schema: &mut Schema) {
        schema.register(SettingSpec::new(
            "keyboard.layout",
            "keyboard",
            "Layout",
            "Active keyboard layout (XKB layout code)",
            ValueKind::Str,
            Value::Str("us".into()),
            PrivilegeLevel::User,
        ));

        schema.register(SettingSpec::new(
            "keyboard.repeat_enabled",
            "keyboard",
            "Key repeat",
            "Repeat a character while its key is held down",
            ValueKind::Bool,
            Value::Bool(true),
            PrivilegeLevel::User,
        ));

        schema.register(
            SettingSpec::new(
                "keyboard.repeat_rate",
                "keyboard",
                "Repeat rate",
                "Repeats per second once a key is held",
                ValueKind::Int,
                Value::Int(25),
                PrivilegeLevel::User,
            )
            .range(1.0, 60.0),
        );

        schema.register(
            SettingSpec::new(
                "keyboard.repeat_delay_ms",
                "keyboard",
                "Repeat delay",
                "Milliseconds before a held key starts repeating",
                ValueKind::Int,
                Value::Int(500),
                PrivilegeLevel::User,
            )
            .range(100.0, 2000.0),
        );

        schema.register(SettingSpec::new(
            "keyboard.shortcuts_enabled",
            "keyboard",
            "Shortcuts",
            "Enable global keyboard shortcuts",
            ValueKind::Bool,
            Value::Bool(true),
            PrivilegeLevel::User,
        ));

        schema.register(
            SettingSpec::new(
                "keyboard.compose_key",
                "keyboard",
                "Compose key",
                "Key used to type accented/special characters",
                ValueKind::Str,
                Value::Str("none".into()),
                PrivilegeLevel::User,
            )
            .choices(&["none", "right_alt", "caps_lock", "right_ctrl", "menu"]),
        );
    }

    fn live_info(&self) -> Vec<(&'static str, String)> {
        crate::hardware::keyboard::list().into_iter().map(|d| ("device", d.name)).collect()
    }
}
