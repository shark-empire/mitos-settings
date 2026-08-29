use crate::categories::Category;
use crate::permissions::PrivilegeLevel;
use crate::settings::schema::{Schema, SettingSpec};
use crate::settings::value::{Value, ValueKind};

pub struct DisplayCategory;

impl Category for DisplayCategory {
    fn id(&self) -> &'static str {
        "display"
    }
    fn name(&self) -> &'static str {
        "Display"
    }
    fn icon(&self) -> &'static str {
        "video-display"
    }
    fn subitems(&self) -> &'static [&'static str] {
        &["Resolution", "Refresh rate", "Scaling", "Brightness", "Night light", "Multiple displays", "Orientation"]
    }

    fn register(&self, schema: &mut Schema) {
        schema.register(SettingSpec::new(
            "display.resolution",
            "display",
            "Resolution",
            "Active display resolution, as WIDTHxHEIGHT",
            ValueKind::Str,
            Value::Str("1920x1080".into()),
            PrivilegeLevel::User,
        ));

        schema.register(
            SettingSpec::new(
                "display.refresh_rate",
                "display",
                "Refresh rate",
                "Display refresh rate in Hz",
                ValueKind::Int,
                Value::Int(60),
                PrivilegeLevel::User,
            )
            .range(30.0, 360.0),
        );

        schema.register(
            SettingSpec::new(
                "display.scaling",
                "display",
                "Scaling",
                "UI scale factor",
                ValueKind::Float,
                Value::Float(1.0),
                PrivilegeLevel::User,
            )
            .range(0.5, 3.0),
        );

        schema.register(
            SettingSpec::new(
                "display.brightness",
                "display",
                "Brightness",
                "Screen brightness percentage",
                ValueKind::Int,
                Value::Int(80),
                PrivilegeLevel::User,
            )
            .range(0.0, 100.0),
        );

        schema.register(SettingSpec::new(
            "display.night_light",
            "display",
            "Night light",
            "Shift colors warmer in the evening to reduce blue light",
            ValueKind::Bool,
            Value::Bool(false),
            PrivilegeLevel::User,
        ));

        schema.register(
            SettingSpec::new(
                "display.multiple_displays_mode",
                "display",
                "Multiple displays",
                "How additional displays are arranged relative to the primary one",
                ValueKind::Str,
                Value::Str("extend".into()),
                PrivilegeLevel::User,
            )
            .choices(&["extend", "mirror", "single"]),
        );

        schema.register(
            SettingSpec::new(
                "display.orientation",
                "display",
                "Orientation",
                "Screen rotation",
                ValueKind::Str,
                Value::Str("landscape".into()),
                PrivilegeLevel::User,
            )
            .choices(&["landscape", "portrait", "landscape-flipped", "portrait-flipped"]),
        );
    }

    fn live_info(&self) -> Vec<(&'static str, String)> {
        crate::hardware::displays::list_connectors()
            .into_iter()
            .map(|c| ("connector", format!("{}: {}", c.name, if c.connected { "connected" } else { "disconnected" })))
            .collect()
    }
}
