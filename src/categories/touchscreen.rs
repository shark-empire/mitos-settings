use crate::categories::Category;
use crate::permissions::PrivilegeLevel;
use crate::settings::schema::{Schema, SettingSpec};
use crate::settings::value::{Value, ValueKind};

pub struct TouchscreenCategory;

impl Category for TouchscreenCategory {
    fn id(&self) -> &'static str {
        "touchscreen"
    }
    fn name(&self) -> &'static str {
        "Touchscreen"
    }
    fn icon(&self) -> &'static str {
        "input-tablet"
    }
    fn subitems(&self) -> &'static [&'static str] {
        &[
            "Touchscreen",
            "Auto-rotate",
            "Palm rejection",
            "Edge gestures",
            "Touch sensitivity",
        ]
    }

    fn register(&self, schema: &mut Schema) {
        schema.register(SettingSpec::new(
            "touchscreen.enabled",
            "touchscreen",
            "Touchscreen",
            "Respond to touch input on this display",
            ValueKind::Bool,
            Value::Bool(true),
            PrivilegeLevel::User,
        ));

        schema.register(SettingSpec::new(
            "touchscreen.auto_rotate",
            "touchscreen",
            "Auto-rotate",
            "Rotate the display to match how the device is held",
            ValueKind::Bool,
            Value::Bool(true),
            PrivilegeLevel::User,
        ));

        schema.register(SettingSpec::new(
            "touchscreen.palm_rejection",
            "touchscreen",
            "Palm rejection",
            "Ignore touches from the edge of your palm while writing or drawing",
            ValueKind::Bool,
            Value::Bool(true),
            PrivilegeLevel::User,
        ));

        schema.register(SettingSpec::new(
            "touchscreen.edge_gestures",
            "touchscreen",
            "Edge gestures",
            "Swipe in from a screen edge to switch apps or go back",
            ValueKind::Bool,
            Value::Bool(true),
            PrivilegeLevel::User,
        ));

        schema.register(
            SettingSpec::new(
                "touchscreen.touch_sensitivity",
                "touchscreen",
                "Touch sensitivity",
                "How much pressure a touch needs to register",
                ValueKind::Float,
                Value::Float(0.0),
                PrivilegeLevel::User,
            )
            .range(-1.0, 1.0),
        );
    }
}
