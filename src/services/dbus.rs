//! Minimal D-Bus client support, implemented by shelling out to `gdbus`
//! (ships with glib2, which mitos-file-manager already depends on as a
//! GTK4 app), with a `dbus-send` fallback for systems without glib. There's
//! no D-Bus wire-protocol implementation here — that's a large, fiddly
//! binary protocol, and shelling out keeps this project dependency-free,
//! consistent with everything else in `services`.
//!
//! **Assumptions to confirm once mitos-gui/mitos-file-manager actually
//! define their D-Bus service:** the file picker is reachable at object
//! path `/org/mitos/FilePicker` implementing an interface named
//! `org.mitos.FilePicker` (matching its bus name) — a common D-Bus
//! convention, but not a guarantee. Update the constants below if MITOS
//! ends up doing it differently.

use std::io;
use std::process::Command;

const FILE_PICKER_BUS_NAME: &str = "org.mitos.FilePicker";
const FILE_PICKER_OBJECT_PATH: &str = "/org/mitos/FilePicker";
const FILE_PICKER_INTERFACE: &str = "org.mitos.FilePicker";

/// Asks the MITOS file picker service to let the user choose a file, and
/// returns the path they picked — or `Ok(None)` if they cancelled.
pub fn pick_file() -> io::Result<Option<String>> {
    let method = format!("{FILE_PICKER_INTERFACE}.OpenFile");
    let raw = match call_gdbus(&method) {
        Ok(r) => r,
        Err(_) => call_dbus_send(&method)?,
    };
    Ok(parse_file_picker_response(&raw))
}

fn call_gdbus(method: &str) -> io::Result<String> {
    let output = Command::new("gdbus")
        .args([
            "call",
            "--session",
            "--dest",
            FILE_PICKER_BUS_NAME,
            "--object-path",
            FILE_PICKER_OBJECT_PATH,
            "--method",
            method,
        ])
        .output()?;
    if !output.status.success() {
        return Err(io::Error::new(io::ErrorKind::Other, String::from_utf8_lossy(&output.stderr).trim().to_string()));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn call_dbus_send(method: &str) -> io::Result<String> {
    let output = Command::new("dbus-send")
        .args([
            "--session",
            "--print-reply",
            &format!("--dest={FILE_PICKER_BUS_NAME}"),
            FILE_PICKER_OBJECT_PATH,
            method,
        ])
        .output()?;
    if !output.status.success() {
        return Err(io::Error::new(io::ErrorKind::Other, String::from_utf8_lossy(&output.stderr).trim().to_string()));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Both `gdbus` and `dbus-send` print a human-readable rendering of the
/// return tuple rather than a clean value (e.g. `('/home/amy/bg.png',)`),
/// so this pulls the first quoted string out of whatever came back.
/// Returns `None` if the reply doesn't contain one — an empty tuple `()`
/// means the user cancelled the picker.
fn parse_file_picker_response(raw: &str) -> Option<String> {
    let start = raw.find('"')?;
    let rest = &raw[start + 1..];
    let end = rest.find('"')?;
    let path = &rest[..end];
    (!path.is_empty()).then(|| path.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_gdbus_style_single_string_reply() {
        assert_eq!(parse_file_picker_response("('/home/amy/Pictures/bg.png',)"), Some("/home/amy/Pictures/bg.png".to_string()));
    }

    #[test]
    fn parses_dbus_send_style_reply() {
        let raw = "method return time=123 sender=:1.5 -> dest=:1.7\n   string \"/home/amy/wall.jpg\"";
        assert_eq!(parse_file_picker_response(raw), Some("/home/amy/wall.jpg".to_string()));
    }

    #[test]
    fn empty_tuple_means_cancelled() {
        assert_eq!(parse_file_picker_response("()"), None);
    }

    #[test]
    fn garbage_input_returns_none_not_a_panic() {
        assert_eq!(parse_file_picker_response(""), None);
        assert_eq!(parse_file_picker_response("no quotes here"), None);
    }
}
