use crate::categories::Category;
use crate::permissions::PrivilegeLevel;
use crate::settings::schema::{Schema, SettingSpec};
use crate::settings::value::{Value, ValueKind};

pub struct ApplicationsCategory;

impl Category for ApplicationsCategory {
    fn id(&self) -> &'static str {
        "applications"
    }
    fn name(&self) -> &'static str {
        "Applications"
    }
    fn icon(&self) -> &'static str {
        "preferences-desktop-default-applications"
    }
    fn subitems(&self) -> &'static [&'static str] {
        &[
            "Default applications",
            "Startup applications",
            "Installed applications",
            "Application permissions",
            "File manager preferences",
        ]
    }

    fn register(&self, schema: &mut Schema) {
        schema.register(SettingSpec::new(
            "applications.default_browser",
            "applications",
            "Default web browser",
            "Application used to open web links",
            ValueKind::Str,
            Value::Str("mitos-browser.desktop".into()),
            PrivilegeLevel::User,
        ));

        schema.register(SettingSpec::new(
            "applications.default_terminal",
            "applications",
            "Default terminal",
            "Application used to open a terminal",
            ValueKind::Str,
            Value::Str("mitos-terminal.desktop".into()),
            PrivilegeLevel::User,
        ));

        schema.register(SettingSpec::new(
            "applications.default_file_manager",
            "applications",
            "Default file manager",
            "Application used to browse files",
            ValueKind::Str,
            Value::Str("mitos-files.desktop".into()),
            PrivilegeLevel::User,
        ));

        schema.register(SettingSpec::new(
            "applications.default_email_client",
            "applications",
            "Default email client",
            "Application used to open mailto: links",
            ValueKind::Str,
            Value::Str("mitos-mail.desktop".into()),
            PrivilegeLevel::User,
        ));

        schema.register(SettingSpec::new(
            "applications.startup_applications",
            "applications",
            "Startup applications",
            "Applications launched automatically at login",
            ValueKind::StrList,
            Value::StrList(Vec::new()),
            PrivilegeLevel::User,
        ));

        // --- File manager preferences: feed home.conf for mitos-file-manager ---
        schema.register(SettingSpec::new(
            "applications.file_manager_show_hidden",
            "applications",
            "Show hidden files",
            "Show dotfiles and other hidden entries in the file manager",
            ValueKind::Bool,
            Value::Bool(false),
            PrivilegeLevel::User,
        ));

        schema.register(SettingSpec::new(
            "applications.file_manager_thumbnails_enabled",
            "applications",
            "Enable thumbnails",
            "Generate image/video thumbnails in the file manager",
            ValueKind::Bool,
            Value::Bool(true),
            PrivilegeLevel::User,
        ));

        schema.register(
            SettingSpec::new(
                "applications.file_manager_thumbnail_max_mb",
                "applications",
                "Thumbnail size limit",
                "Skip generating thumbnails for files larger than this, in megabytes",
                ValueKind::Int,
                Value::Int(50),
                PrivilegeLevel::User,
            )
            .range(1.0, 500.0),
        );
    }
}
