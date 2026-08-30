//! Parses `argv` into one of the four subcommands and dispatches to the
//! matching module, each of which works directly against a
//! `SettingsManager` — the same one the interactive app and the daemon
//! use. `--daemon` is intercepted by `main.rs` before it ever reaches
//! here, since running the daemon is a fundamentally different, blocking
//! mode rather than a single request/response command.

pub mod get;
pub mod list;
pub mod pick_wallpaper;
pub mod reset;
pub mod set;

use crate::settings::manager::{Mode, SettingsManager};

const USAGE: &str = "\
Usage: mitos-settings <command> [args]

Commands:
  get <key>              Print the current value of a setting
  set <key> <value>      Change a setting
  list [category]        List every category, or every setting in one
  reset <key> | --all    Restore a setting (or everything) to its default
  pick-wallpaper          Open the MITOS file picker and set the wallpaper
  --daemon                Run as the privileged settings daemon
  --help                  Show this message

Run with no command to open the interactive navigator.";

/// Returns a process exit code, the way `main` expects.
pub fn run(args: &[String]) -> i32 {
    let Some(command) = args.first() else {
        return run_interactive();
    };

    if command == "--help" || command == "-h" {
        println!("{USAGE}");
        return 0;
    }

    let mut manager = match SettingsManager::load(Mode::Standalone) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("mitos-settings: could not load settings: {e}");
            return 1;
        }
    };

    let rest = &args[1..];
    let result = match command.as_str() {
        "get" => get::execute(&manager, rest),
        "set" => set::execute(&mut manager, rest),
        "list" => list::execute(&manager, rest),
        "reset" => reset::execute(&mut manager, rest),
        "pick-wallpaper" => pick_wallpaper::execute(&mut manager),
        other => Err(format!("unknown command '{other}'\n\n{USAGE}")),
    };

    match result {
        Ok(output) => {
            if !output.is_empty() {
                print!("{output}");
                if !output.ends_with('\n') {
                    println!();
                }
            }
            0
        }
        Err(e) => {
            eprintln!("mitos-settings: {e}");
            1
        }
    }
}

fn run_interactive() -> i32 {
    match crate::app::Application::new() {
        Ok(mut app) => {
            app.run();
            0
        }
        Err(e) => {
            eprintln!("mitos-settings: could not start: {e}");
            1
        }
    }
}
