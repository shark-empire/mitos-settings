use std::io;
use std::process::Command;

pub fn set_volume(percent: u8) -> io::Result<()> {
    let pct = percent.min(100);
    run("amixer", &["-q", "set", "Master", &format!("{pct}%")])
        .or_else(|_| run("pactl", &["set-sink-volume", "@DEFAULT_SINK@", &format!("{pct}%")]))
}

pub fn set_muted(muted: bool) -> io::Result<()> {
    let state = if muted { "mute" } else { "unmute" };
    run("amixer", &["-q", "set", "Master", state])
        .or_else(|_| run("pactl", &["set-sink-mute", "@DEFAULT_SINK@", if muted { "1" } else { "0" }]))
}

fn run(cmd: &str, args: &[&str]) -> io::Result<()> {
    let status = Command::new(cmd).args(args).status()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::new(io::ErrorKind::Other, format!("{cmd} exited with {status}")))
    }
}
