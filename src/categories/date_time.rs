use crate::categories::Category;
use crate::permissions::PrivilegeLevel;
use crate::settings::schema::{Schema, SettingSpec};
use crate::settings::value::{Value, ValueKind};

pub struct DateTimeCategory;

impl Category for DateTimeCategory {
    fn id(&self) -> &'static str {
        "date_time"
    }
    fn name(&self) -> &'static str {
        "Date & Time"
    }
    fn icon(&self) -> &'static str {
        "preferences-system-time"
    }
    fn subitems(&self) -> &'static [&'static str] {
        &["Time zone", "Automatic time", "Date format", "12/24 hour"]
    }

    fn register(&self, schema: &mut Schema) {
        schema.register(SettingSpec::new(
            "date_time.timezone",
            "date_time",
            "Time zone",
            "IANA time zone name, e.g. America/New_York",
            ValueKind::Str,
            Value::Str("UTC".into()),
            PrivilegeLevel::Admin,
        ));

        schema.register(SettingSpec::new(
            "date_time.automatic_time",
            "date_time",
            "Set automatically",
            "Sync the clock over the network (NTP)",
            ValueKind::Bool,
            Value::Bool(true),
            PrivilegeLevel::Admin,
        ));

        schema.register(
            SettingSpec::new(
                "date_time.date_format",
                "date_time",
                "Date format",
                "How dates are displayed",
                ValueKind::Str,
                Value::Str("YYYY-MM-DD".into()),
                PrivilegeLevel::User,
            )
            .choices(&["YYYY-MM-DD", "MM/DD/YYYY", "DD/MM/YYYY", "DD Month YYYY"]),
        );

        schema.register(SettingSpec::new(
            "date_time.hour_24",
            "date_time",
            "24-hour time",
            "Use a 24-hour clock instead of AM/PM",
            ValueKind::Bool,
            Value::Bool(true),
            PrivilegeLevel::User,
        ));
    }

    fn live_info(&self) -> Vec<(&'static str, String)> {
        match crate::services::time::current_timezone() {
            Some(tz) => vec![("system_timezone", tz)],
            None => vec![("system_timezone", "unavailable (timedatectl not found)".to_string())],
        }
    }
}
