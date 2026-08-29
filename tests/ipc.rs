//! Spins up a real `IpcServer` on a throwaway Unix socket, backed by a real
//! (temp-file) `SettingsManager`, and talks to it with a real `IpcClient`
//! — end to end, no mocks.

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

    let set_response =
        IpcClient::send(&socket, &Request::Set { key: "sound.volume".to_string(), value: Value::Int(66) }).unwrap();
    assert!(matches!(set_response, Response::Ok(_)), "unexpected response: {set_response:?}");

    let get_response = IpcClient::send(&socket, &Request::Get { key: "sound.volume".to_string() }).unwrap();
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

    let response = IpcClient::send(&socket, &Request::List { category: Some("sound".to_string()) }).unwrap();
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

    let response = IpcClient::send(&socket, &Request::Get { key: "nonexistent.key".to_string() }).unwrap();
    assert!(matches!(response, Response::Err(_)));

    std::fs::remove_dir_all(socket.parent().unwrap()).ok();
}

#[test]
fn reset_via_socket_restores_default() {
    let socket = temp_socket_path("reset");
    spawn_test_daemon(socket.clone(), Mode::DaemonAuthority);

    IpcClient::send(&socket, &Request::Set { key: "sound.volume".to_string(), value: Value::Int(3) }).unwrap();
    let reset_response = IpcClient::send(&socket, &Request::Reset { key: Some("sound.volume".to_string()) }).unwrap();
    assert!(matches!(reset_response, Response::Ok(_)));

    let get_response = IpcClient::send(&socket, &Request::Get { key: "sound.volume".to_string() }).unwrap();
    match get_response {
        Response::Ok(encoded) => assert_eq!(Value::decode(&encoded).unwrap(), Value::Int(50)),
        other => panic!("expected Ok, got {other:?}"),
    }

    std::fs::remove_dir_all(socket.parent().unwrap()).ok();
}
