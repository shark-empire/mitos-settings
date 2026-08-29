use crate::categories::Category;
use crate::permissions::PrivilegeLevel;
use crate::settings::schema::{Schema, SettingSpec};
use crate::settings::value::{Value, ValueKind};

pub struct RegionCategory;

impl Category for RegionCategory {
    fn id(&self) -> &'static str {
        "region"
    }
    fn name(&self) -> &'static str {
        "Region"
    }
    fn icon(&self) -> &'static str {
        "preferences-desktop-locale"
    }
    fn subitems(&self) -> &'static [&'static str] {
        &["Region", "Currency", "Formats"]
    }

    fn register(&self, schema: &mut Schema) {
        schema.register(SettingSpec::new(
            "region.country",
            "region",
            "Region",
            "Country/region code used for formatting defaults (ISO 3166-1 alpha-2)",
            ValueKind::Str,
            Value::Str("US".into()),
            PrivilegeLevel::User,
        ));

        schema.register(SettingSpec::new(
            "region.currency",
            "region",
            "Currency",
            "Currency code used to format prices (ISO 4217)",
            ValueKind::Str,
            Value::Str("USD".into()),
            PrivilegeLevel::User,
        ));

        schema.register(
            SettingSpec::new(
                "region.measurement_system",
                "region",
                "Measurement units",
                "Metric or imperial units",
                ValueKind::Str,
                Value::Str("metric".into()),
                PrivilegeLevel::User,
            )
            .choices(&["metric", "imperial"]),
        );

        schema.register(
            SettingSpec::new(
                "region.first_day_of_week",
                "region",
                "First day of week",
                "Which day calendars start on",
                ValueKind::Str,
                Value::Str("monday".into()),
                PrivilegeLevel::User,
            )
            .choices(&["sunday", "monday"]),
        );
    }
}
