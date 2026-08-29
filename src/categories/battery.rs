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
        let batteries = crate::hardware::battery::list();
        if batteries.is_empty() {
            return vec![("battery", "no battery detected (desktop or VM)".to_string())];
        }
        batteries
            .into_iter()
            .map(|b| {
                let percent = b.capacity_percent.map(|p| format!("{p}%")).unwrap_or_else(|| "unknown".into());
                let status = b.status.unwrap_or_else(|| "unknown".into());
                ("battery", format!("{}: {percent}, {status}", b.name))
            })
            .collect()
    }
}
