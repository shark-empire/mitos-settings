use std::fs;
use std::io;
use std::path::PathBuf;

fn backlight_dir() -> Option<PathBuf> {
    fs::read_dir("/sys/class/backlight").ok()?.flatten().next().map(|e| e.path())
}

pub fn get_brightness() -> Option<u8> {
    let dir = backlight_dir()?;
    let max: u32 = fs::read_to_string(dir.join("max_brightness")).ok()?.trim().parse().ok()?;
    let cur: u32 = fs::read_to_string(dir.join("brightness")).ok()?.trim().parse().ok()?;
    if max == 0 {
        return None;
    }
    Some(((cur as f64 / max as f64) * 100.0).round() as u8)
}

/// Writes a percentage (0-100) to the first backlight device found under
/// `/sys/class/backlight`. Writing that file requires either running as
/// root or being covered by a udev rule granting the `video` group write
/// access — this surfaces the underlying `io::Error` untouched so the
/// caller (`SettingsManager`) can explain what went wrong.
pub fn set_brightness(percent: u8) -> io::Result<()> {
    let dir = backlight_dir().ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no backlight device found"))?;
    let max: u32 = fs::read_to_string(dir.join("max_brightness"))?.trim().parse().unwrap_or(100);
    let raw = ((percent.min(100) as u32 * max) as f64 / 100.0).round() as u32;
    fs::write(dir.join("brightness"), raw.to_string())
}

/// Night light is a compositor feature, and this project doesn't ship a
/// compositor. MITOS's session compositor is expected to watch the
/// persisted `display.night_light` setting itself, so this is an
/// acknowledgement hook (returns `Ok` once the value is safely persisted)
/// rather than a direct system call.
pub fn set_night_light(_enabled: bool) -> io::Result<()> {
    Ok(())
}
