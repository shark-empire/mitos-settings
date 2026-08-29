use crate::categories::Category;
use crate::permissions::PrivilegeLevel;
use crate::settings::schema::{Schema, SettingSpec};
use crate::settings::value::{Value, ValueKind};

pub struct UsersCategory;

impl Category for UsersCategory {
    fn id(&self) -> &'static str {
        "users"
    }
    fn name(&self) -> &'static str {
        "Users"
    }
    fn icon(&self) -> &'static str {
        "system-users"
    }
    fn subitems(&self) -> &'static [&'static str] {
        &["Accounts", "Password", "Administrator", "Login options"]
    }

    fn register(&self, schema: &mut Schema) {
        schema.register(SettingSpec::new(
            "users.require_password_on_wake",
            "users",
            "Require password on wake",
            "Ask for a password when returning from sleep or the screensaver",
            ValueKind::Bool,
            Value::Bool(true),
            PrivilegeLevel::User,
        ));

        schema.register(SettingSpec::new(
            "users.auto_login",
            "users",
            "Automatic login",
            "Sign in automatically without a password at boot",
            ValueKind::Bool,
            Value::Bool(false),
            PrivilegeLevel::Admin,
        ));

        schema.register(SettingSpec::new(
            "users.guest_account_enabled",
            "users",
            "Guest account",
            "Allow signing in as a temporary, unprivileged guest",
            ValueKind::Bool,
            Value::Bool(false),
            PrivilegeLevel::Admin,
        ));
    }

    fn live_info(&self) -> Vec<(&'static str, String)> {
        crate::services::accounts::list()
            .into_iter()
            .map(|a| ("account", format!("{} (uid {})", a.username, a.uid)))
            .collect()
    }
}
