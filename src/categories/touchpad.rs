use crate::categories::Category;
use crate::permissions::PrivilegeLevel;
use crate::settings::schema::{Schema, SettingSpec};
use crate::settings::value::{Value, ValueKind};

pub struct TouchpadCategory;

impl Category for TouchpadCategory {
    fn id(&self) -> &'static str {
        "touchpad"
    }
    fn name(&self) -> &'static str {
        "Touchpad"
    }
    fn icon(&self) -> &'static str {
        "input-touchpad"
    }
    fn subitems(&self) -> &'static [&'static str] {
        &["Natural scrolling", "Tap-to-click", "Two-finger scroll", "Gestures", "Disable while typing"]
    }

    fn register(&self, schema: &mut Schema) {
        schema.register(SettingSpec::new(
            "touchpad.natural_scrolling",
            "touchpad",
            "Natural scrolling",
            "Content follows finger direction instead of the traditional convention",
            ValueKind::Bool,
            Value::Bool(true),
            PrivilegeLevel::User,
        ));

        schema.register(SettingSpec::new(
            "touchpad.tap_to_click",
            "touchpad",
            "Tap to click",
            "Tap the touchpad to register a click",
            ValueKind::Bool,
            Value::Bool(true),
            PrivilegeLevel::User,
        ));

        schema.register(SettingSpec::new(
            "touchpad.two_finger_scroll",
            "touchpad",
            "Two-finger scroll",
            "Scroll by dragging two fingers",
            ValueKind::Bool,
            Value::Bool(true),
            PrivilegeLevel::User,
        ));

        schema.register(SettingSpec::new(
            "touchpad.gestures_enabled",
            "touchpad",
            "Gestures",
            "Enable multi-finger swipe and pinch gestures",
            ValueKind::Bool,
            Value::Bool(true),
            PrivilegeLevel::User,
        ));

        schema.register(SettingSpec::new(
            "touchpad.disable_while_typing",
            "touchpad",
            "Disable while typing",
            "Ignore touchpad input briefly after a keystroke",
            ValueKind::Bool,
            Value::Bool(true),
            PrivilegeLevel::User,
        ));

        schema.register(
            SettingSpec::new(
                "touchpad.pointer_speed",
                "touchpad",
                "Pointer speed",
                "Cursor movement speed",
                ValueKind::Float,
                Value::Float(0.0),
                PrivilegeLevel::User,
            )
            .range(-1.0, 1.0),
        );
    }

    fn live_info(&self) -> Vec<(&'static str, String)> {
        crate::hardware::touchpad::list().into_iter().map(|d| ("device", d.name)).collect()
    }
}
