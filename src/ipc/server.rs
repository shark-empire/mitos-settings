use super::permissions::{ensure_daemon_may_apply, peer_credentials};
use super::protocol::{Request, Response};
use crate::permissions::{self, AuthContext};
use crate::settings::manager::SettingsManager;
use std::io::BufReader;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::{fs, thread};

pub struct IpcServer {
    listener: UnixListener,
}

impl IpcServer {
    /// Binds `path`, clearing out a stale socket left behind by a previous
    /// run, and restricts both the socket and its parent directory's
    /// permissions (see docs/security.md for what that boundary actually
    /// guarantees, and `ipc::permissions` for the per-connection check on
    /// top of it).
    pub fn bind(path: &Path) -> std::io::Result<Self> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
            restrict_dir(parent)?;
        }
        let _ = fs::remove_file(path);
        let listener = UnixListener::bind(path)?;
        restrict_socket(path)?;
        Ok(IpcServer { listener })
    }

    /// Accepts connections forever, handling each on its own thread against
    /// a shared, mutex-guarded `SettingsManager`.
    pub fn run(self, manager: Arc<Mutex<SettingsManager>>) {
        for conn in self.listener.incoming() {
            match conn {
                Ok(stream) => {
                    let manager = Arc::clone(&manager);
                    thread::spawn(move || handle(stream, &manager));
                }
                Err(e) => eprintln!("mitos-settings daemon: accept failed: {e}"),
            }
        }
    }
}

#[cfg(unix)]
fn restrict_socket(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = fs::metadata(path)?.permissions();
    perms.set_mode(0o660); // owner + group only
    fs::set_permissions(path, perms)
}
#[cfg(not(unix))]
fn restrict_socket(_path: &Path) -> std::io::Result<()> {
    Ok(())
}

/// Locks down the socket's parent directory too (not just the socket
/// file): without this, `/run/mitos-settings/` would get whatever the
/// creating process's default umask leaves it at -- often
/// world-traversable -- which would let anyone race to recreate the
/// socket file between the `remove_file` and `bind` above, or otherwise
/// tamper with the directory. Owner gets full access; the admin group can
/// enter and open a file it already knows the name of (`--x`, no `r`), so
/// it still can't *list* the directory's contents; everyone else gets
/// nothing.
#[cfg(unix)]
fn restrict_dir(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = fs::metadata(path)?.permissions();
    perms.set_mode(0o710);
    fs::set_permissions(path, perms)
}
#[cfg(not(unix))]
fn restrict_dir(_path: &Path) -> std::io::Result<()> {
    Ok(())
}

fn handle(stream: UnixStream, manager: &Arc<Mutex<SettingsManager>>) {
    let peer = match peer_credentials(&stream) {
        Ok(cred) => permissions::context_for_uid(cred.uid),
        Err(e) => {
            let _ = Response::Err(format!("could not verify peer identity: {e}")).write_to(&stream);
            return;
        }
    };

    let reader = BufReader::new(match stream.try_clone() {
        Ok(s) => s,
        Err(_) => return,
    });
    let request = match Request::read_from(reader) {
        Ok(r) => r,
        Err(e) => {
            let _ = Response::Err(e.to_string()).write_to(&stream);
            return;
        }
    };

    let response = dispatch(request, manager, &peer);
    let _ = response.write_to(&stream);
}

fn dispatch(request: Request, manager: &Arc<Mutex<SettingsManager>>, peer: &AuthContext) -> Response {
    let mut manager = match manager.lock() {
        Ok(g) => g,
        Err(_) => return Response::Err("daemon state poisoned; restart the daemon".into()),
    };

    match request {
        Request::Ping => Response::Ok("pong".into()),

        Request::WhoAmI => {
            Response::Ok(format!("{} (uid {}, {})", peer.username, peer.uid, peer.level()))
        }

        Request::Get { key } => match manager.get(&key) {
            Ok(v) => Response::Ok(v.encode()),
            Err(e) => Response::Err(e.to_string()),
        },

        Request::Set { key, value } => {
            if let Some(spec) = manager.schema().get(&key) {
                if let Err(e) = ensure_daemon_may_apply(spec) {
                    return Response::Err(e);
                }
            }
            match manager.set_for_peer(&key, value, peer) {
                Ok(()) => Response::Ok("applied".into()),
                Err(e) => Response::Err(e.to_string()),
            }
        }

        Request::Reset { key: Some(key) } => match manager.reset_for_peer(&key, peer) {
            Ok(()) => Response::Ok("reset".into()),
            Err(e) => Response::Err(e.to_string()),
        },

        Request::Reset { key: None } => match manager.reset_all_for_peer(peer) {
            Ok(()) => Response::Ok("reset".into()),
            Err(e) => Response::Err(e.to_string()),
        },

        Request::List { category } => {
            let rows: Vec<(String, String)> = manager
                .schema()
                .all()
                .filter(|s| category.as_deref().map(|c| c == s.category).unwrap_or(true))
                .map(|s| {
                    let encoded = manager.get(s.key).map(|v| v.encode()).unwrap_or_default();
                    (s.key.to_string(), encoded)
                })
                .collect();
            Response::Data(rows)
        }
    }
}
