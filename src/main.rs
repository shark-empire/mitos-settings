//! Thin binary shell. All real logic lives in the library (`lib.rs` and
//! below); this file just decides whether we're starting the privileged
//! daemon or doing everything else (a CLI subcommand, or the interactive
//! navigator with no arguments), and turns the result into a process exit
//! code.

use mitos_settings::config::paths;
use mitos_settings::ipc::IpcServer;
use mitos_settings::settings::manager::{Mode, SettingsManager};
use std::sync::{Arc, Mutex};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    let code = if args.first().map(String::as_str) == Some("--daemon") {
        run_daemon()
    } else {
        mitos_settings::cli::run(&args)
    };

    std::process::exit(code);
}

/// Runs as the privileged, long-lived settings daemon: binds the well-known
/// Unix socket and serves requests forever. Intended to be started by
/// systemd as root; see docs/security.md for why root matters here and
/// what happens if it isn't.
fn run_daemon() -> i32 {
    let manager = match SettingsManager::load(Mode::DaemonAuthority) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("mitos-settings daemon: could not load settings: {e}");
            return 1;
        }
    };

    let ctx = manager.context();
    if ctx.uid != 0 {
        eprintln!(
            "mitos-settings daemon: warning: running as '{}' (uid {}), not root -- \
             Root-level settings will be rejected until this runs as root",
            ctx.username, ctx.uid
        );
    }

    let socket_path = paths::daemon_socket_path();
    let server = match IpcServer::bind(&socket_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("mitos-settings daemon: could not bind {}: {e}", socket_path.display());
            return 1;
        }
    };

    println!("mitos-settings daemon: listening on {}", socket_path.display());
    server.run(Arc::new(Mutex::new(manager)));
    0 // unreachable in practice -- `run` serves connections forever
}
