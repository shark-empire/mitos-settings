//! Glass & translucency settings. Registered like every other category so
//! the schema-driven GUI renders its sliders automatically.

use crate::permissions::PrivilegeLevel;
use crate::settings::schema::{Schema, SettingSpec, ValueKind};
use crate::settings::value::Value;

pub struct GlassCategory;

impl super::Category for GlassCategory {
    fn id(&self) -> &'static str {
        "glass"
    }
    fn name(&self) -> &'static str {
        "Glass & Effects"
    }
    fn icon(&self) -> &'static str {
        "preferences-desktop-appearance"
    }
    fn subitems(&self) -> &'static [&'static str] {
        &["Frost", "Transparency", "Tint"]
    }

    fn register(&self, schema: &mut Schema) {
        schema.register(SettingSpec::new(
            "glass.enabled",
            "glass",
            "Liquid glass",
            "Frosted translucency across the shell and all GTK4 apps",
            ValueKind::Bool,
            Value::Bool(true),
            PrivilegeLevel::User,
        ));
        schema.register(SettingSpec::new(
            "glass.blur",
            "glass",
            "Blur strength",
            "How strongly backgrounds behind translucent surfaces are frosted (0-100)",
            ValueKind::Int,
            Value::Int(60),
            PrivilegeLevel::User,
        ));
        schema.register(SettingSpec::new(
            "glass.opacity",
            "glass",
            "Surface opacity",
            "How solid translucent surfaces feel (0-100, higher = more opaque)",
            ValueKind::Int,
            Value::Int(55),
            PrivilegeLevel::User,
        ));
        schema.register(SettingSpec::new(
            "glass.tint",
            "glass",
            "Frost tint",
            "Accent colour mixed into the frost (#RRGGBB)",
            ValueKind::Str,
            Value::Str("#4C8DFF".into()),
            PrivilegeLevel::User,
        ));
    }
}
