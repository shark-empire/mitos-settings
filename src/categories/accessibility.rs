use crate::categories::Category;
use crate::permissions::PrivilegeLevel;
use crate::settings::schema::{Schema, SettingSpec};
use crate::settings::value::{Value, ValueKind};

pub struct AccessibilityCategory;

impl Category for AccessibilityCategory {
    fn id(&self) -> &'static str {
        "accessibility"
    }
    fn name(&self) -> &'static str {
        "Accessibility"
    }
    fn icon(&self) -> &'static str {
        "preferences-desktop-accessibility"
    }
    fn subitems(&self) -> &'static [&'static str] {
        &["Screen reader", "Magnification", "High contrast", "Large text", "Sticky keys", "Mouse accessibility"]
    }

    fn register(&self, schema: &mut Schema) {
        schema.register(SettingSpec::new(
            "accessibility.screen_reader_enabled",
            "accessibility",
            "Screen reader",
            "Read screen content aloud",
            ValueKind::Bool,
            Value::Bool(false),
            PrivilegeLevel::User,
        ));

        schema.register(SettingSpec::new(
            "accessibility.magnification_enabled",
            "accessibility",
            "Magnification",
            "Zoom into the screen around the pointer",
            ValueKind::Bool,
            Value::Bool(false),
            PrivilegeLevel::User,
        ));

        schema.register(
            SettingSpec::new(
                "accessibility.magnification_level",
                "accessibility",
                "Magnification level",
                "Zoom factor when magnification is on",
                ValueKind::Float,
                Value::Float(2.0),
                PrivilegeLevel::User,
            )
            .range(1.0, 8.0),
        );

        schema.register(SettingSpec::new(
            "accessibility.high_contrast",
            "accessibility",
            "High contrast",
            "Increase contrast between foreground and background colors",
            ValueKind::Bool,
            Value::Bool(false),
            PrivilegeLevel::User,
        ));

        schema.register(SettingSpec::new(
            "accessibility.large_text",
            "accessibility",
            "Large text",
            "Scale up system text size",
            ValueKind::Bool,
            Value::Bool(false),
            PrivilegeLevel::User,
        ));

        schema.register(
            SettingSpec::new(
                "accessibility.text_scale",
                "accessibility",
                "Text scale",
                "Multiplier applied to text size when large text is on",
                ValueKind::Float,
                Value::Float(1.25),
                PrivilegeLevel::User,
            )
            .range(1.0, 2.5),
        );

        schema.register(SettingSpec::new(
            "accessibility.sticky_keys",
            "accessibility",
            "Sticky keys",
            "Press modifier keys one at a time instead of holding them",
            ValueKind::Bool,
            Value::Bool(false),
            PrivilegeLevel::User,
        ));

        schema.register(SettingSpec::new(
            "accessibility.mouse_keys",
            "accessibility",
            "Mouse accessibility",
            "Control the pointer using the numeric keypad",
            ValueKind::Bool,
            Value::Bool(false),
            PrivilegeLevel::User,
        ));
    }
}
