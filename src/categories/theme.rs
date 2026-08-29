use crate::categories::Category;
use crate::permissions::PrivilegeLevel;
use crate::settings::schema::{Schema, SettingSpec};
use crate::settings::value::{Value, ValueKind};

pub struct ThemeCategory;

impl Category for ThemeCategory {
    fn id(&self) -> &'static str {
        "theme"
    }
    fn name(&self) -> &'static str {
        "Theme"
    }
    fn icon(&self) -> &'static str {
        "preferences-desktop-theme"
    }
    fn subitems(&self) -> &'static [&'static str] {
        &["Light/dark mode", "Window theme", "Cursor theme"]
    }

    fn register(&self, schema: &mut Schema) {
        schema.register(
            SettingSpec::new(
                "theme.mode",
                "theme",
                "Appearance mode",
                "Whether the desktop uses a light or dark color scheme",
                ValueKind::Str,
                Value::Str("system".into()),
                PrivilegeLevel::User,
            )
            .choices(&["light", "dark", "system"]),
        );

        schema.register(SettingSpec::new(
            "theme.window_theme",
            "theme",
            "Window theme",
            "GTK/Qt-style window decoration theme",
            ValueKind::Str,
            Value::Str("mitos-default".into()),
            PrivilegeLevel::User,
        ));

        schema.register(SettingSpec::new(
            "theme.cursor_theme",
            "theme",
            "Cursor theme",
            "Mouse pointer icon set",
            ValueKind::Str,
            Value::Str("mitos-default".into()),
            PrivilegeLevel::User,
        ));

        schema.register(
            SettingSpec::new(
                "theme.cursor_size",
                "theme",
                "Cursor size",
                "Pointer size in pixels",
                ValueKind::Int,
                Value::Int(24),
                PrivilegeLevel::User,
            )
            .range(16.0, 64.0),
        );
    }
}
