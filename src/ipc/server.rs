use super::permissions::ensure_daemon_may_apply;
use super::protocol::{Request, Response};
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
    /// run, and restricts its permissions to owner+group (see
    /// docs/security.md for what that boundary actually guarantees).
    pub fn bind(path: &Path) -> std::io::Result<Self> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let _ = fs::remove_file(path);
        let listener = UnixListener::bind(path)?;
        restrict(path)?;
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
fn restrict(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = fs::metadata(path)?.permissions();
    perms.set_mode(0o660);
    fs::set_permissions(path, perms)
}

#[cfg(not(unix))]
fn restrict(_path: &Path) -> std::io::Result<()> {
    Ok(())
}

fn handle(stream: UnixStream, manager: &Arc<Mutex<SettingsManager>>) {
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

    let response = dispatch(request, manager);
    let _ = response.write_to(&stream);
}

fn dispatch(request: Request, manager: &Arc<Mutex<SettingsManager>>) -> Response {
    let mut manager = match manager.lock() {
        Ok(g) => g,
        Err(_) => return Response::Err("daemon state poisoned; restart the daemon".into()),
    };

    match request {
        Request::Ping => Response::Ok("pong".into()),

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
            match manager.set(&key, value) {
                Ok(()) => Response::Ok("applied".into()),
                Err(e) => Response::Err(e.to_string()),
            }
        }

        Request::Reset { key: Some(key) } => match manager.reset(&key) {
            Ok(()) => Response::Ok("reset".into()),
            Err(e) => Response::Err(e.to_string()),
        },

        Request::Reset { key: None } => match manager.reset_all() {
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
