use crate::categories::Category;
use crate::permissions::PrivilegeLevel;
use crate::settings::schema::{Schema, SettingSpec};
use crate::settings::value::{Value, ValueKind};

pub struct MouseCategory;

impl Category for MouseCategory {
    fn id(&self) -> &'static str {
        "mouse"
    }
    fn name(&self) -> &'static str {
        "Mouse"
    }
    fn icon(&self) -> &'static str {
        "input-mouse"
    }
    fn subitems(&self) -> &'static [&'static str] {
        &["Pointer speed", "Acceleration", "Button mapping", "Scroll speed"]
    }

    fn register(&self, schema: &mut Schema) {
        schema.register(
            SettingSpec::new(
                "mouse.pointer_speed",
                "mouse",
                "Pointer speed",
                "Cursor movement speed",
                ValueKind::Float,
                Value::Float(0.0),
                PrivilegeLevel::User,
            )
            .range(-1.0, 1.0),
        );

        schema.register(SettingSpec::new(
            "mouse.acceleration_enabled",
            "mouse",
            "Acceleration",
            "Scale pointer speed with movement velocity",
            ValueKind::Bool,
            Value::Bool(true),
            PrivilegeLevel::User,
        ));

        schema.register(
            SettingSpec::new(
                "mouse.button_mapping",
                "mouse",
                "Button mapping",
                "Left- or right-handed button layout",
                ValueKind::Str,
                Value::Str("right_handed".into()),
                PrivilegeLevel::User,
            )
            .choices(&["right_handed", "left_handed"]),
        );

        schema.register(
            SettingSpec::new(
                "mouse.scroll_speed",
                "mouse",
                "Scroll speed",
                "Lines scrolled per wheel notch",
                ValueKind::Int,
                Value::Int(3),
                PrivilegeLevel::User,
            )
            .range(1.0, 20.0),
        );

        schema.register(SettingSpec::new(
            "mouse.natural_scrolling",
            "mouse",
            "Natural scrolling",
            "Content follows finger/wheel direction instead of the traditional convention",
            ValueKind::Bool,
            Value::Bool(false),
            PrivilegeLevel::User,
        ));
    }

    fn live_info(&self) -> Vec<(&'static str, String)> {
        crate::hardware::mouse::list().into_iter().map(|d| ("device", d.name)).collect()
    }
}
