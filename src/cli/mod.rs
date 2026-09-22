//! Parses `argv` into one of these subcommands and dispatches to the
//! matching module, each of which does its own argument parsing and
//! (except `grants`, which talks to `IpcClient`/mitos-service directly)
//! works against a `SettingsManager` -- the same one the interactive app
//! and the daemon use. `--daemon` is intercepted by `main.rs` before it
//! ever reaches here, since running the daemon is a fundamentally
//! different, blocking mode rather than a single request/response
//! command.

pub mod get;
pub mod grants;
pub mod list;
pub mod pick_wallpaper;
pub mod reset;
pub mod schema;
pub mod set;

use crate::settings::manager::{Mode, SettingsManager};

const USAGE: &str = "\
Usage: mitos-settings <command> [args]

Commands:
  get <key>              Print the current value of a setting
  set <key> <value>      Change a setting
  list [category] [--json]  List categories/settings; --json for machine-readable output
  reset <key> | --all    Restore a setting (or everything) to its default
  schema                  Dump the full schema (types, defaults, constraints) as JSON
  grants <subcommand>     List/set/revoke mitos-service permission grants -- run
                          `mitos-settings grants` with no subcommand for details
  pick-wallpaper          Open the MITOS file picker and set the wallpaper
  --daemon                Run as the privileged settings daemon
  --help                  Show this message

Run with no command to open the interactive navigator.
See INTEGRATION.md for how other MITOS projects should talk to this one.";

/// Returns a process exit code, the way `main` expects.
pub fn run(args: &[String]) -> i32 {
    let Some(command) = args.first() else {
        return run_interactive();
    };

    if command == "--help" || command == "-h" {
        println!("{USAGE}");
        return 0;
    }

    // `grants` talks to `IpcClient`/mitos-service directly and never
    // touches a `SettingsManager` -- see `grants`'s own doc comment.
    // Handled here, before `SettingsManager::load` below, so a grants
    // command still works even if this machine's settings files
    // themselves are unreadable or corrupt; nothing about listing or
    // changing a permission grant should depend on that.
    if command == "grants" {
        return print_result(grants::execute(&args[1..]));
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
        "schema" => schema::execute(&manager),
        "pick-wallpaper" => pick_wallpaper::execute(&mut manager),
        other => Err(format!("unknown command '{other}'\n\n{USAGE}")),
    };

    print_result(result)
}

fn print_result(result: Result<String, String>) -> i32 {
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
