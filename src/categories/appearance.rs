use crate::categories::Category;
use crate::permissions::PrivilegeLevel;
use crate::settings::schema::{Schema, SettingSpec, ValueFormat};
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
        &["Colors", "Fonts", "Animations", "Icon theme", "Glass & panels", "Shell layout"]
    }

    fn register(&self, schema: &mut Schema) {
        // Feeds `accent_color` in ~/.config/mitos/home.conf -- see
        // docs/home-conf.md. Stored as a hex string (not a named palette
        // choice) because that's the format mitos-gui and
        // mitos-file-manager both consume directly.
        schema.register(
            SettingSpec::new(
                "appearance.accent_color",
                "appearance",
                "Accent color",
                "Highlight color used for selections, links, toggles, and the MITOS shell. Hex: #RGB, #RRGGBB, or #RRGGBBAA",
                ValueKind::Str,
                Value::Str("#4d9eff".into()),
                PrivilegeLevel::User,
            )
            .format(ValueFormat::HexColor),
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

        // --- Glass & panels: feeds glass_opacity / panel_radius in home.conf ---
        schema.register(
            SettingSpec::new(
                "appearance.glass_opacity",
                "appearance",
                "Glass opacity",
                "Translucency of the shell's glass panels (0 = fully transparent, 1 = opaque)",
                ValueKind::Float,
                Value::Float(0.72),
                PrivilegeLevel::User,
            )
            .range(0.0, 1.0),
        );

        schema.register(
            SettingSpec::new(
                "appearance.panel_radius",
                "appearance",
                "Panel corner radius",
                "Corner rounding, in pixels, for shell panels and windows",
                ValueKind::Float,
                Value::Float(18.0),
                PrivilegeLevel::User,
            )
            .range(0.0, 48.0),
        );

        // --- Shell layout: feeds top_bar / dock / launcher in home.conf ---
        schema.register(SettingSpec::new(
            "appearance.top_bar_enabled",
            "appearance",
            "Top bar",
            "Show the top status bar",
            ValueKind::Bool,
            Value::Bool(true),
            PrivilegeLevel::User,
        ));

        schema.register(
            SettingSpec::new(
                "appearance.top_bar_height",
                "appearance",
                "Top bar height",
                "Height of the top bar, in pixels",
                ValueKind::Float,
                Value::Float(38.0),
                PrivilegeLevel::User,
            )
            .range(24.0, 64.0),
        );

        schema.register(SettingSpec::new(
            "appearance.dock_enabled",
            "appearance",
            "Dock",
            "Show the application dock",
            ValueKind::Bool,
            Value::Bool(true),
            PrivilegeLevel::User,
        ));

        schema.register(
            SettingSpec::new(
                "appearance.dock_height",
                "appearance",
                "Dock height",
                "Height of the dock, in pixels",
                ValueKind::Float,
                Value::Float(72.0),
                PrivilegeLevel::User,
            )
            .range(32.0, 160.0),
        );

        schema.register(SettingSpec::new(
            "appearance.launcher_enabled",
            "appearance",
            "Launcher",
            "Show the application launcher button",
            ValueKind::Bool,
            Value::Bool(true),
            PrivilegeLevel::User,
        ));
    }
}
