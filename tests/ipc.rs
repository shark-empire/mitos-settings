//! Spins up a real `IpcServer` on a throwaway Unix socket, backed by a real
//! (temp-file) `SettingsManager`, and talks to it with a real `IpcClient`
//! — end to end, no mocks.

use mitos_settings::grants::{Decision, Scope};
use mitos_settings::ipc::{IpcClient, IpcServer, Request, Response};
use mitos_settings::settings::manager::{Mode, SettingsManager};
use mitos_settings::settings::persistence::Store;
use mitos_settings::settings::value::Value;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

fn temp_socket_path(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("mitos-ipc-itest-{label}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir.join("daemon.sock")
}

/// Binds and starts serving on a background thread. `bind()` itself is
/// synchronous, so by the time this returns the socket file exists; the
/// short sleep afterward is just a safety margin for the accept loop to
/// actually be listening under scheduler jitter.
fn spawn_test_daemon(socket: PathBuf, mode: Mode) {
    let store_dir = socket.parent().unwrap().to_path_buf();
    let manager = SettingsManager::with_stores(
        mode,
        Store::at(store_dir.join("user.conf")),
        Store::at(store_dir.join("system.conf")),
    )
    .unwrap();
    let server = IpcServer::bind(&socket).unwrap();
    thread::spawn(move || {
        server.run(Arc::new(Mutex::new(manager)));
    });
    thread::sleep(Duration::from_millis(50));
}

#[test]
fn ping_gets_a_pong() {
    let socket = temp_socket_path("ping");
    spawn_test_daemon(socket.clone(), Mode::DaemonAuthority);

    let response = IpcClient::send(&socket, &Request::Ping).unwrap();
    assert!(matches!(response, Response::Ok(s) if s == "pong"));

    std::fs::remove_dir_all(socket.parent().unwrap()).ok();
}

#[test]
fn get_and_set_round_trip_over_the_socket() {
    let socket = temp_socket_path("get-set");
    spawn_test_daemon(socket.clone(), Mode::DaemonAuthority);

    let set_response = IpcClient::send(
        &socket,
        &Request::Set {
            key: "sound.volume".to_string(),
            value: Value::Int(66),
        },
    )
    .unwrap();
    assert!(
        matches!(set_response, Response::Ok(_)),
        "unexpected response: {set_response:?}"
    );

    let get_response = IpcClient::send(
        &socket,
        &Request::Get {
            key: "sound.volume".to_string(),
        },
    )
    .unwrap();
    match get_response {
        Response::Ok(encoded) => assert_eq!(Value::decode(&encoded).unwrap(), Value::Int(66)),
        other => panic!("expected Ok, got {other:?}"),
    }

    std::fs::remove_dir_all(socket.parent().unwrap()).ok();
}

#[test]
fn list_returns_rows_for_a_category() {
    let socket = temp_socket_path("list");
    spawn_test_daemon(socket.clone(), Mode::DaemonAuthority);

    let response = IpcClient::send(
        &socket,
        &Request::List {
            category: Some("sound".to_string()),
        },
    )
    .unwrap();
    match response {
        Response::Data(rows) => assert!(rows.iter().any(|(k, _)| k == "sound.volume")),
        other => panic!("expected Data, got {other:?}"),
    }

    std::fs::remove_dir_all(socket.parent().unwrap()).ok();
}

#[test]
fn unknown_key_produces_an_error_response_not_a_dropped_connection() {
    let socket = temp_socket_path("unknown-key");
    spawn_test_daemon(socket.clone(), Mode::DaemonAuthority);

    let response = IpcClient::send(
        &socket,
        &Request::Get {
            key: "nonexistent.key".to_string(),
        },
    )
    .unwrap();
    assert!(matches!(response, Response::Err(_)));

    std::fs::remove_dir_all(socket.parent().unwrap()).ok();
}

#[test]
fn reset_via_socket_restores_default() {
    let socket = temp_socket_path("reset");
    spawn_test_daemon(socket.clone(), Mode::DaemonAuthority);

    IpcClient::send(
        &socket,
        &Request::Set {
            key: "sound.volume".to_string(),
            value: Value::Int(3),
        },
    )
    .unwrap();
    let reset_response = IpcClient::send(
        &socket,
        &Request::Reset {
            key: Some("sound.volume".to_string()),
        },
    )
    .unwrap();
    assert!(matches!(reset_response, Response::Ok(_)));

    let get_response = IpcClient::send(
        &socket,
        &Request::Get {
            key: "sound.volume".to_string(),
        },
    )
    .unwrap();
    match get_response {
        Response::Ok(encoded) => assert_eq!(Value::decode(&encoded).unwrap(), Value::Int(50)),
        other => panic!("expected Ok, got {other:?}"),
    }

    std::fs::remove_dir_all(socket.parent().unwrap()).ok();
}

#[test]
fn whoami_reports_the_real_connecting_uid() {
    let socket = temp_socket_path("whoami");
    spawn_test_daemon(socket.clone(), Mode::DaemonAuthority);

    let response = IpcClient::send(&socket, &Request::WhoAmI).unwrap();
    let expected_uid = mitos_settings::permissions::current_context().uid;
    match response {
        // The test connects to its own daemon, so SO_PEERCRED should
        // report this same process's uid back -- proving peer resolution
        // actually round-trips through a real socket, not just a
        // same-process UnixStream::pair() (see ipc::permissions's own
        // unit test for that narrower check).
        Response::Ok(msg) => assert!(
            msg.contains(&format!("uid {expected_uid}")),
            "unexpected whoami: {msg}"
        ),
        other => panic!("expected Ok, got {other:?}"),
    }

    std::fs::remove_dir_all(socket.parent().unwrap()).ok();
}

#[test]
fn user_level_setting_succeeds_regardless_of_peer_privilege() {
    // sound.volume is User-level, so this should succeed whether or not
    // the connecting peer (in practice, this test process) happens to be
    // an admin -- unlike an Admin/Root-level key, which would depend on
    // the sandbox's actual privilege level and so isn't a reliable thing
    // for a portable test to assert on.
    let socket = temp_socket_path("user-level-peer");
    spawn_test_daemon(socket.clone(), Mode::DaemonAuthority);

    let response = IpcClient::send(
        &socket,
        &Request::Set {
            key: "sound.volume".to_string(),
            value: Value::Int(12),
        },
    )
    .unwrap();
    assert!(
        matches!(response, Response::Ok(_)),
        "unexpected response: {response:?}"
    );

    std::fs::remove_dir_all(socket.parent().unwrap()).ok();
}

// --- Permission grants ---
//
// mitos-settings holds no grant state of its own anymore -- every one
// of these requests is a pass-through to mitos-service's control
// socket (`grants::service_client`), which this sandbox doesn't run.
// So unlike the settings tests above, none of these can exercise a
// *successful* grant change end to end; what they can and do verify
// is that mitos-settings' own socket still behaves correctly when its
// dependency is missing -- a clean `Response::Err`, on every one of
// `ListGrants`/`SetGrant`/`RevokeGrant`, never a hang or a dropped
// connection -- which matters regardless of risk tier, since the
// low/moderate-vs-dangerous distinction is now decided entirely on
// mitos-service's side, not this daemon's.

#[test]
fn listing_grants_fails_cleanly_when_mitos_service_is_unreachable() {
    let socket = temp_socket_path("grants-list-no-service");
    spawn_test_daemon(socket.clone(), Mode::DaemonAuthority);

    let response = IpcClient::send(&socket, &Request::ListGrants).unwrap();
    assert!(matches!(response, Response::Err(_)), "expected an error, got {response:?}");

    std::fs::remove_dir_all(socket.parent().unwrap()).ok();
}

#[test]
fn setting_a_grant_fails_cleanly_when_mitos_service_is_unreachable() {
    let socket = temp_socket_path("grants-set-no-service");
    spawn_test_daemon(socket.clone(), Mode::DaemonAuthority);

    let response = IpcClient::send(
        &socket,
        &Request::SetGrant {
            sha256: "a".repeat(64),
            capability: "camera".to_string(),
            decision: Decision::Allow,
            scope: Scope::Always,
        },
    )
    .unwrap();
    assert!(matches!(response, Response::Err(_)), "expected an error, got {response:?}");

    std::fs::remove_dir_all(socket.parent().unwrap()).ok();
}

#[test]
fn revoking_a_grant_fails_cleanly_when_mitos_service_is_unreachable() {
    let socket = temp_socket_path("grants-revoke-no-service");
    spawn_test_daemon(socket.clone(), Mode::DaemonAuthority);

    let response = IpcClient::send(
        &socket,
        &Request::RevokeGrant {
            sha256: "a".repeat(64),
            capability: "camera".to_string(),
        },
    )
    .unwrap();
    assert!(matches!(response, Response::Err(_)), "expected an error, got {response:?}");

    std::fs::remove_dir_all(socket.parent().unwrap()).ok();
}
