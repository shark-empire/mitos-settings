use std::io;
use std::process::Command;

pub fn current_language() -> Option<String> {
    std::env::var("LANG").ok()
}

pub fn set_language(locale: &str) -> io::Result<()> {
    let arg = format!("LANG={locale}");
    let status = Command::new("localectl").args(["set-locale", &arg]).status()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::new(io::ErrorKind::Other, "localectl set-locale failed (needs root)"))
    }
}
