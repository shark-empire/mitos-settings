use crate::categories::Category;
use crate::permissions::PrivilegeLevel;
use crate::settings::schema::{Schema, SettingSpec};
use crate::settings::value::{Value, ValueKind};

pub struct LanguageCategory;

impl Category for LanguageCategory {
    fn id(&self) -> &'static str {
        "language"
    }
    fn name(&self) -> &'static str {
        "Language"
    }
    fn icon(&self) -> &'static str {
        "preferences-desktop-locale"
    }
    fn subitems(&self) -> &'static [&'static str] {
        &["Language", "Keyboard layouts"]
    }

    fn register(&self, schema: &mut Schema) {
        schema.register(SettingSpec::new(
            "language.system_language",
            "language",
            "Language",
            "System-wide display language, as a POSIX locale (e.g. en_US.UTF-8)",
            ValueKind::Str,
            Value::Str("en_US.UTF-8".into()),
            PrivilegeLevel::Admin,
        ));

        schema.register(SettingSpec::new(
            "language.keyboard_layouts",
            "language",
            "Keyboard layouts",
            "Input sources available via the layout switcher (first is active)",
            ValueKind::StrList,
            Value::StrList(vec!["us".into()]),
            PrivilegeLevel::User,
        ));
    }

    fn live_info(&self) -> Vec<(&'static str, String)> {
        match crate::services::locale::current_language() {
            Some(lang) => vec![("current_lang_env", lang)],
            None => vec![("current_lang_env", "LANG is not set".to_string())],
        }
    }
}
