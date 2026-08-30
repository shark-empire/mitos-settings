use crate::services::dbus;
use crate::settings::manager::SettingsManager;
use crate::settings::value::Value;

/// Asks the MITOS file picker (over D-Bus) for a wallpaper image, and sets
/// `wallpaper.desktop_path` to whatever the user chose. No custom file
/// browser here by design — see `services::dbus` and docs/home-conf.md.
pub fn execute(manager: &mut SettingsManager) -> Result<String, String> {
    match dbus::pick_file() {
        Ok(Some(path)) => {
            manager.set("wallpaper.desktop_path", Value::Str(path.clone())).map_err(|e| e.to_string())?;
            Ok(format!("Wallpaper set to {path}."))
        }
        Ok(None) => Ok("No file selected.".to_string()),
        Err(e) => Err(format!("could not reach the MITOS file picker: {e}")),
    }
}
