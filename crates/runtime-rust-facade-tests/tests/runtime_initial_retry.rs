//! Task070: public lifecycle and generated endpoint controls. Serialized so
//! Linux worker counts belong exclusively to the test holding the lock.
mod common;

use ams_gra_oms_runtime_rust::{
    RuntimeConfig, RuntimeError, RuntimeEvent, SleetRuntime, WORKER_THREAD_NAME,
};
use ams_gra_oms_runtime_rust_facade_tests::codec::{TestCodec, shared};
use ams_gra_oms_runtime_rust_facade_tests::runtime_test::model::SharedPayload;
use ams_gra_oms_runtime_rust_facade_tests::runtime_test::service_api::function_track as track;
use common::{InitReply, MockPeer, QUIET, WAIT};
use sleet_client::ClientError;
use std::error::Error;
use std::sync::Mutex;
use std::time::Duration;
use tokio_tungstenite::tungstenite::Error as WsError;

static SERIAL: Mutex<()> = Mutex::new(());

fn config(url: &str) -> RuntimeConfig {
    RuntimeConfig::new(url, "svc-1", "000.1.0")
        .with_connect_timeout(Duration::from_millis(500))
        .with_initial_connect_retry(3, Duration::from_millis(2), Duration::from_millis(4))
}

fn no_workers() {
    #[cfg(target_os = "linux")]
    assert_eq!(
        std::fs::read_dir("/proc/self/task")
            .unwrap()
            .filter_map(Result::ok)
            .filter(|task| {
                std::fs::read_to_string(task.path().join("comm")).is_ok_and(|name| {
                    let name = name.trim_end();
                    !name.is_empty() && WORKER_THREAD_NAME.starts_with(name)
                })
            })
            .count(),
        0,
        "runtime worker outlived joined connect/close"
    );
}

#[test]
fn task070_terminal_refusal_preserves_error_and_joins_worker() {
    let _guard = SERIAL.lock().unwrap();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    drop(listener);
    let error = match SleetRuntime::connect(config(&format!("ws://{addr}")), TestCodec) {
        Ok(_) => panic!("refusal must not return a runtime"),
        Err(error) => error,
    };
    assert!(
        error
            .source()
            .unwrap()
            .downcast_ref::<ClientError>()
            .is_some()
    );
    let RuntimeError::Client(client) = error else {
        unreachable!()
    };
    assert!(
        matches!(*client, ClientError::Transport(WsError::Io(ref io)) if io.kind() == std::io::ErrorKind::ConnectionRefused)
    );
    no_workers();
}

#[test]
fn task070_init_rejection_is_terminal() {
    let _guard = SERIAL.lock().unwrap();
    let peer = MockPeer::start_retry_control(InitReply::Frame("-ERR Illegal-State rejected"));
    let error = match SleetRuntime::connect(config(&peer.url), TestCodec) {
        Ok(_) => panic!("INIT rejection must fail"),
        Err(error) => error,
    };
    assert!(
        matches!(error, RuntimeError::Client(ref client) if matches!(client.as_ref(), ClientError::Server { details: Some(details), .. } if details == "rejected"))
    );
    assert!(peer.next_text().starts_with("INIT "));
    peer.expect_closed();
    peer.expect_no_connection();
    no_workers();
}

#[test]
fn task070_established_session_never_reconnects_or_replays() {
    let _guard = SERIAL.lock().unwrap();
    let peer = MockPeer::start_retry_control(InitReply::Info);
    let mut runtime = SleetRuntime::connect(config(&peer.url), TestCodec).unwrap();
    assert!(peer.next_text().starts_with("INIT "));
    let handle = track::exchange_input_a::subscribe(&mut runtime, |_: &SharedPayload| {}).unwrap();
    peer.expect("SUB sub-1 {urn:test}MessageA input-topic");
    track::exchange_output_a::publish(&mut runtime, &shared(1)).unwrap();
    peer.expect(r#"PUB output-topic {"{urn:test}MessageA":{"Count":1}}"#);
    peer.expect_silence(QUIET);
    peer.close();
    assert!(matches!(
        runtime.recv_event_timeout(WAIT),
        Some(RuntimeEvent::ConnectionClosed { .. })
    ));
    // Receivers were dropped when the worker stopped; these calls cannot
    // start a new worker or bind the old SID to a fresh connection.
    assert!(matches!(
        track::exchange_output_a::publish(&mut runtime, &shared(2)),
        Err(RuntimeError::WorkerStopped)
    ));
    assert!(matches!(
        handle.unsubscribe(),
        Err(RuntimeError::WorkerStopped)
    ));
    runtime.close().unwrap();
    peer.expect_no_connection();
    no_workers();
}

#[test]
fn task070_generated_publish_once_and_close() {
    let _guard = SERIAL.lock().unwrap();
    let peer = MockPeer::start_retry_control(InitReply::Info);
    let mut runtime = SleetRuntime::connect(config(&peer.url), TestCodec).unwrap();
    assert!(peer.next_text().starts_with("INIT "));
    track::exchange_output_a::publish(&mut runtime, &shared(7)).unwrap();
    peer.expect(r#"PUB output-topic {"{urn:test}MessageA":{"Count":7}}"#);
    runtime.close().unwrap();
    peer.expect_closed();
    peer.expect_no_connection();
    no_workers();
}
