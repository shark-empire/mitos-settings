use crate::categories::Category;
use crate::permissions::PrivilegeLevel;
use crate::settings::schema::{Schema, SettingSpec};
use crate::settings::value::{Value, ValueKind};

pub struct WallpaperCategory;

impl Category for WallpaperCategory {
    fn id(&self) -> &'static str {
        "wallpaper"
    }
    fn name(&self) -> &'static str {
        "Wallpaper"
    }
    fn icon(&self) -> &'static str {
        "preferences-desktop-wallpaper"
    }
    fn subitems(&self) -> &'static [&'static str] {
        &["Desktop background", "Lock screen background", "Slideshow", "Picture position"]
    }

    fn register(&self, schema: &mut Schema) {
        schema.register(SettingSpec::new(
            "wallpaper.desktop_path",
            "wallpaper",
            "Desktop background",
            "Path to the current desktop wallpaper image",
            ValueKind::Str,
            Value::Str("/usr/share/mitos/wallpapers/default.png".into()),
            PrivilegeLevel::User,
        ));

        schema.register(SettingSpec::new(
            "wallpaper.lock_screen_path",
            "wallpaper",
            "Lock screen background",
            "Path to the image shown on the lock screen",
            ValueKind::Str,
            Value::Str("/usr/share/mitos/wallpapers/default.png".into()),
            PrivilegeLevel::User,
        ));

        schema.register(SettingSpec::new(
            "wallpaper.slideshow_enabled",
            "wallpaper",
            "Slideshow",
            "Cycle through a folder of images instead of a single wallpaper",
            ValueKind::Bool,
            Value::Bool(false),
            PrivilegeLevel::User,
        ));

        schema.register(
            SettingSpec::new(
                "wallpaper.slideshow_interval_minutes",
                "wallpaper",
                "Slideshow interval",
                "Minutes between slideshow image changes",
                ValueKind::Int,
                Value::Int(30),
                PrivilegeLevel::User,
            )
            .range(1.0, 1440.0),
        );

        schema.register(
            SettingSpec::new(
                "wallpaper.position",
                "wallpaper",
                "Picture position",
                "How the wallpaper image is fit to the screen",
                ValueKind::Str,
                Value::Str("fill".into()),
                PrivilegeLevel::User,
            )
            .choices(&["fill", "fit", "stretch", "center", "tile"]),
        );
    }
}
