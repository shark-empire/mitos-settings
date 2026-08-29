use crate::categories::Category;
use crate::permissions::PrivilegeLevel;
use crate::settings::schema::{Schema, SettingSpec};
use crate::settings::value::{Value, ValueKind};

pub struct BluetoothCategory;

impl Category for BluetoothCategory {
    fn id(&self) -> &'static str {
        "bluetooth"
    }
    fn name(&self) -> &'static str {
        "Bluetooth"
    }
    fn icon(&self) -> &'static str {
        "bluetooth"
    }
    fn subitems(&self) -> &'static [&'static str] {
        &["Devices", "Pairing", "Discovery"]
    }

    fn register(&self, schema: &mut Schema) {
        schema.register(SettingSpec::new(
            "bluetooth.enabled",
            "bluetooth",
            "Bluetooth",
            "Power the Bluetooth adapter on or off",
            ValueKind::Bool,
            Value::Bool(true),
            PrivilegeLevel::User,
        ));

        schema.register(SettingSpec::new(
            "bluetooth.discoverable",
            "bluetooth",
            "Discoverable",
            "Let nearby devices find this machine for pairing",
            ValueKind::Bool,
            Value::Bool(false),
            PrivilegeLevel::User,
        ));

        schema.register(SettingSpec::new(
            "bluetooth.auto_scan",
            "bluetooth",
            "Discovery",
            "Continuously scan for nearby devices while Settings is open",
            ValueKind::Bool,
            Value::Bool(true),
            PrivilegeLevel::User,
        ));
    }

    fn live_info(&self) -> Vec<(&'static str, String)> {
        if !crate::hardware::bluetooth::is_present() {
            return vec![("adapter", "no Bluetooth adapter detected".to_string())];
        }
        crate::services::bluetooth::list_devices()
            .into_iter()
            .map(|d| ("device", format!("{} ({})", d.name, d.mac)))
            .collect()
    }
}
