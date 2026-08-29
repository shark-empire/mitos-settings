//! Each file in this module is one entry in the Settings navigator. A
//! `Category` does two things: it registers its `SettingSpec`s into the
//! `Schema` (so they're persistable and validated generically), and,
//! optionally, exposes `live_info` for read-only data that's computed on
//! the fly from `hardware`/`services` rather than stored (disk usage,
//! paired Bluetooth devices, kernel version, ...).

use crate::settings::schema::{CategoryMeta, Schema};

pub mod about;
pub mod accessibility;
pub mod appearance;
pub mod applications;
pub mod battery;
pub mod bluetooth;
pub mod date_time;
pub mod developer;
pub mod display;
pub mod keyboard;
pub mod language;
pub mod mouse;
pub mod network;
pub mod notifications;
pub mod power;
pub mod printers;
pub mod privacy;
pub mod region;
pub mod security;
pub mod sharing;
pub mod sound;
pub mod storage;
pub mod theme;
pub mod touchpad;
pub mod updates;
pub mod users;
pub mod wallpaper;

/// Something that shows up as a top-level entry in the Settings navigator.
pub trait Category {
    fn id(&self) -> &'static str;
    fn name(&self) -> &'static str;
    fn icon(&self) -> &'static str;
    fn subitems(&self) -> &'static [&'static str];
    fn register(&self, schema: &mut Schema);

    /// Live, read-only key/value pairs sourced straight from hardware or a
    /// service. Most categories have none; About, Storage, Battery,
    /// Printers, and Users override this.
    fn live_info(&self) -> Vec<(&'static str, String)> {
        Vec::new()
    }
}

/// Every category, in navigator order.
pub fn all() -> Vec<Box<dyn Category>> {
    vec![
        Box::new(appearance::AppearanceCategory),
        Box::new(theme::ThemeCategory),
        Box::new(wallpaper::WallpaperCategory),
        Box::new(display::DisplayCategory),
        Box::new(sound::SoundCategory),
        Box::new(network::NetworkCategory),
        Box::new(bluetooth::BluetoothCategory),
        Box::new(keyboard::KeyboardCategory),
        Box::new(mouse::MouseCategory),
        Box::new(touchpad::TouchpadCategory),
        Box::new(power::PowerCategory),
        Box::new(battery::BatteryCategory),
        Box::new(users::UsersCategory),
        Box::new(privacy::PrivacyCategory),
        Box::new(security::SecurityCategory),
        Box::new(applications::ApplicationsCategory),
        Box::new(notifications::NotificationsCategory),
        Box::new(accessibility::AccessibilityCategory),
        Box::new(date_time::DateTimeCategory),
        Box::new(language::LanguageCategory),
        Box::new(region::RegionCategory),
        Box::new(storage::StorageCategory),
        Box::new(printers::PrintersCategory),
        Box::new(sharing::SharingCategory),
        Box::new(updates::UpdatesCategory),
        Box::new(developer::DeveloperCategory),
        Box::new(about::AboutCategory),
    ]
}

pub fn register_all(schema: &mut Schema) {
    for category in all() {
        schema.register_category(CategoryMeta {
            id: category.id(),
            name: category.name(),
            icon: category.icon(),
            subitems: category.subitems(),
        });
        category.register(schema);
    }
}

pub fn find(id: &str) -> Option<Box<dyn Category>> {
    all().into_iter().find(|c| c.id() == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_category_has_a_unique_id() {
        let categories = all();
        let mut ids: Vec<&str> = categories.iter().map(|c| c.id()).collect();
        ids.sort_unstable();
        let mut deduped = ids.clone();
        deduped.dedup();
        assert_eq!(ids.len(), deduped.len(), "duplicate category id found");
    }

    #[test]
    fn register_all_populates_schema() {
        let mut schema = Schema::new();
        register_all(&mut schema);
        assert!(!schema.is_empty());
        assert_eq!(schema.categories().len(), all().len());
    }
}
