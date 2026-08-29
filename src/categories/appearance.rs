use crate::categories::Category;
use crate::permissions::PrivilegeLevel;
use crate::settings::schema::{Schema, SettingSpec};
use crate::settings::value::{Value, ValueKind};

pub struct AppearanceCategory;

impl Category for AppearanceCategory {
    fn id(&self) -> &'static str {
        "appearance"
    }
    fn name(&self) -> &'static str {
        "Appearance"
    }
    fn icon(&self) -> &'static str {
        "preferences-desktop-appearance"
    }
    fn subitems(&self) -> &'static [&'static str] {
        &["Colors", "Fonts", "Animations", "Icon theme"]
    }

    fn register(&self, schema: &mut Schema) {
        schema.register(
            SettingSpec::new(
                "appearance.accent_color",
                "appearance",
                "Accent color",
                "Highlight color used for selections, links, and toggles",
                ValueKind::Str,
                Value::Str("blue".into()),
                PrivilegeLevel::User,
            )
            .choices(&["blue", "purple", "green", "orange", "red", "graphite"]),
        );

        schema.register(SettingSpec::new(
            "appearance.font_family",
            "appearance",
            "Font",
            "System-wide default UI font",
            ValueKind::Str,
            Value::Str("Inter".into()),
            PrivilegeLevel::User,
        ));

        schema.register(
            SettingSpec::new(
                "appearance.font_size",
                "appearance",
                "Font size",
                "Base UI font size in points",
                ValueKind::Float,
                Value::Float(10.0),
                PrivilegeLevel::User,
            )
            .range(7.0, 18.0),
        );

        schema.register(SettingSpec::new(
            "appearance.animations_enabled",
            "appearance",
            "Animations",
            "Enable window and transition animations",
            ValueKind::Bool,
            Value::Bool(true),
            PrivilegeLevel::User,
        ));

        schema.register(SettingSpec::new(
            "appearance.icon_theme",
            "appearance",
            "Icon theme",
            "Icon pack used across the desktop",
            ValueKind::Str,
            Value::Str("mitos-default".into()),
            PrivilegeLevel::User,
        ));
    }
}
