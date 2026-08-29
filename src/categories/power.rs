use crate::categories::Category;
use crate::permissions::PrivilegeLevel;
use crate::settings::schema::{Schema, SettingSpec};
use crate::settings::value::{Value, ValueKind};

pub struct PowerCategory;

impl Category for PowerCategory {
    fn id(&self) -> &'static str {
        "power"
    }
    fn name(&self) -> &'static str {
        "Power"
    }
    fn icon(&self) -> &'static str {
        "preferences-system-power"
    }
    fn subitems(&self) -> &'static [&'static str] {
        &["Battery", "Power profiles", "Screen timeout", "Suspend", "Lid behavior"]
    }

    fn register(&self, schema: &mut Schema) {
        schema.register(
            SettingSpec::new(
                "power.profile",
                "power",
                "Power profile",
                "Balances performance against battery life",
                ValueKind::Str,
                Value::Str("balanced".into()),
                PrivilegeLevel::User,
            )
            .choices(&["power-saver", "balanced", "performance"]),
        );

        schema.register(
            SettingSpec::new(
                "power.screen_timeout_minutes",
                "power",
                "Screen timeout",
                "Minutes of inactivity before the screen turns off (0 = never)",
                ValueKind::Int,
                Value::Int(5),
                PrivilegeLevel::User,
            )
            .range(0.0, 180.0),
        );

        schema.register(
            SettingSpec::new(
                "power.suspend_timeout_minutes",
                "power",
                "Suspend",
                "Minutes of inactivity before the system suspends (0 = never)",
                ValueKind::Int,
                Value::Int(15),
                PrivilegeLevel::User,
            )
            .range(0.0, 360.0),
        );

        schema.register(SettingSpec::new(
            "power.suspend_on_battery_low",
            "power",
            "Auto-suspend on low battery",
            "Force-suspend when the battery drops critically low",
            ValueKind::Bool,
            Value::Bool(true),
            PrivilegeLevel::User,
        ));

        schema.register(
            SettingSpec::new(
                "power.lid_close_action",
                "power",
                "Lid behavior",
                "What happens when a laptop lid is closed",
                ValueKind::Str,
                Value::Str("suspend".into()),
                PrivilegeLevel::Admin,
            )
            .choices(&["suspend", "hibernate", "shutdown", "nothing"]),
        );
    }
}
