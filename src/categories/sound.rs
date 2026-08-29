use crate::categories::Category;
use crate::permissions::PrivilegeLevel;
use crate::settings::schema::{Schema, SettingSpec};
use crate::settings::value::{Value, ValueKind};

pub struct SoundCategory;

impl Category for SoundCategory {
    fn id(&self) -> &'static str {
        "sound"
    }
    fn name(&self) -> &'static str {
        "Sound"
    }
    fn icon(&self) -> &'static str {
        "audio-speakers"
    }
    fn subitems(&self) -> &'static [&'static str] {
        &["Output device", "Input device", "Volume", "Applications", "Microphone", "Alerts"]
    }

    fn register(&self, schema: &mut Schema) {
        schema.register(SettingSpec::new(
            "sound.output_device",
            "sound",
            "Output device",
            "Sound card used for audio playback",
            ValueKind::Str,
            Value::Str("default".into()),
            PrivilegeLevel::User,
        ));

        schema.register(SettingSpec::new(
            "sound.input_device",
            "sound",
            "Input device",
            "Sound card used for audio capture",
            ValueKind::Str,
            Value::Str("default".into()),
            PrivilegeLevel::User,
        ));

        schema.register(
            SettingSpec::new(
                "sound.volume",
                "sound",
                "Volume",
                "Output volume percentage",
                ValueKind::Int,
                Value::Int(50),
                PrivilegeLevel::User,
            )
            .range(0.0, 100.0),
        );

        schema.register(SettingSpec::new(
            "sound.output_muted",
            "sound",
            "Mute",
            "Mute all audio output",
            ValueKind::Bool,
            Value::Bool(false),
            PrivilegeLevel::User,
        ));

        schema.register(
            SettingSpec::new(
                "sound.microphone_level",
                "sound",
                "Microphone level",
                "Input gain percentage",
                ValueKind::Int,
                Value::Int(70),
                PrivilegeLevel::User,
            )
            .range(0.0, 100.0),
        );

        schema.register(SettingSpec::new(
            "sound.alert_sound",
            "sound",
            "Alert sound",
            "Sound played for system alerts",
            ValueKind::Str,
            Value::Str("chime".into()),
            PrivilegeLevel::User,
        ).choices(&["chime", "glass", "ping", "none"]));
    }

    fn live_info(&self) -> Vec<(&'static str, String)> {
        crate::hardware::audio::list_cards()
            .into_iter()
            .map(|c| ("card", format!("{}: {}", c.index, c.description)))
            .collect()
    }
}
