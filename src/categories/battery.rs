use crate::categories::Category;
use crate::permissions::PrivilegeLevel;
use crate::settings::schema::{Schema, SettingSpec};
use crate::settings::value::{Value, ValueKind};

pub struct BatteryCategory;

impl Category for BatteryCategory {
    fn id(&self) -> &'static str {
        "battery"
    }
    fn name(&self) -> &'static str {
        "Battery"
    }
    fn icon(&self) -> &'static str {
        "battery"
    }
    fn subitems(&self) -> &'static [&'static str] {
        &["Charge level", "Status", "Low battery threshold"]
    }

    fn register(&self, schema: &mut Schema) {
        schema.register(
            SettingSpec::new(
                "battery.low_battery_threshold",
                "battery",
                "Low battery threshold",
                "Percentage at which a low-battery warning is shown",
                ValueKind::Int,
                Value::Int(15),
                PrivilegeLevel::User,
            )
            .range(1.0, 50.0),
        );
    }

    fn live_info(&self) -> Vec<(&'static str, String)> {
        crate::services::battery::live_status()
            .into_iter()
            .map(|(_, value)| ("battery", value))
            .collect()
    }
}
