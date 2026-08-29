use crate::categories::Category;
use crate::permissions::PrivilegeLevel;
use crate::settings::schema::{Schema, SettingSpec};
use crate::settings::value::{Value, ValueKind};

pub struct NetworkCategory;

impl Category for NetworkCategory {
    fn id(&self) -> &'static str {
        "network"
    }
    fn name(&self) -> &'static str {
        "Network"
    }
    fn icon(&self) -> &'static str {
        "network-wired"
    }
    fn subitems(&self) -> &'static [&'static str] {
        &["Ethernet", "Wi-Fi", "VPN", "Proxy", "DNS", "Firewall"]
    }

    fn register(&self, schema: &mut Schema) {
        schema.register(SettingSpec::new(
            "network.ethernet_enabled",
            "network",
            "Ethernet",
            "Enable the wired network connection",
            ValueKind::Bool,
            Value::Bool(true),
            PrivilegeLevel::User,
        ));

        schema.register(SettingSpec::new(
            "network.wifi_enabled",
            "network",
            "Wi-Fi",
            "Enable the wireless radio",
            ValueKind::Bool,
            Value::Bool(true),
            PrivilegeLevel::User,
        ));

        schema.register(SettingSpec::new(
            "network.vpn_active_profile",
            "network",
            "VPN",
            "Name of the active VPN profile, empty for none",
            ValueKind::Str,
            Value::Str(String::new()),
            PrivilegeLevel::User,
        ));

        schema.register(
            SettingSpec::new(
                "network.proxy_mode",
                "network",
                "Proxy",
                "How outgoing connections are proxied",
                ValueKind::Str,
                Value::Str("none".into()),
                PrivilegeLevel::Admin,
            )
            .choices(&["none", "manual", "automatic"]),
        );

        schema.register(SettingSpec::new(
            "network.dns_servers",
            "network",
            "DNS",
            "Comma-separated list of DNS server addresses (empty = use DHCP)",
            ValueKind::StrList,
            Value::StrList(Vec::new()),
            PrivilegeLevel::Admin,
        ));

        schema.register(SettingSpec::new(
            "network.firewall_enabled",
            "network",
            "Firewall",
            "Block unsolicited incoming connections",
            ValueKind::Bool,
            Value::Bool(true),
            PrivilegeLevel::Admin,
        ));
    }

    fn live_info(&self) -> Vec<(&'static str, String)> {
        crate::services::network::list_interfaces()
            .into_iter()
            .map(|i| ("interface", format!("{}: {}", i.name, i.operstate)))
            .collect()
    }
}
