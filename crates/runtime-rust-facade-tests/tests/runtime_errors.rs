//! Task 049: error and event paths of the Rust LA-CAL runtime, driven
//! through GENERATED endpoint functions against the mock OWP peer.

mod common;

use ams_gra_oms_runtime_rust::{RuntimeConfig, RuntimeError, RuntimeEvent, SleetRuntime};
use ams_gra_oms_runtime_rust_facade_tests::codec::{TestCodec, shared};
use ams_gra_oms_runtime_rust_facade_tests::runtime_test::model::SharedPayload;
use ams_gra_oms_runtime_rust_facade_tests::runtime_test::service_api::function_misconfigured as bad;
use ams_gra_oms_runtime_rust_facade_tests::runtime_test::service_api::function_track as track;
use common::{InitReply, MockPeer, QUIET, WAIT, connect, is_invalid_input};

/// Authored routing text that the contract accepts but OWP does not is
/// rejected by sleet-client, returned to the caller, and never written.
#[test]
fn task049_invalid_routing_metadata_is_a_runtime_error_not_a_frame() {
    let peer = MockPeer::start();
    let mut runtime = connect(&peer);

    let error = bad::exchange_output_bad_topic::publish(&mut runtime, &shared(1))
        .expect_err("invalid topic");
    assert!(is_invalid_input(&error, "bad topic"), "{error}");

    let error = bad::exchange_input_bad_group::subscribe(&mut runtime, |_: &SharedPayload| {})
        .expect_err("invalid group");
    assert!(is_invalid_input(&error, "invalid group"), "{error}");
    peer.expect_silence(QUIET);

    // The connection is unaffected. The rejected SUB consumed sub-1: IDs
    // are never reused, so the next subscription is sub-2.
    let handle = track::exchange_input_a::subscribe(&mut runtime, |_: &SharedPayload| {})
        .expect("valid subscribe");
    assert_eq!(handle.id(), "sub-2");
    peer.expect("SUB sub-2 {urn:test}MessageA input-topic");
    runtime.close().expect("close");
}

/// Non-verbose: publish returns once written; the later -ERR is a runtime
/// event, not lost.
#[test]
fn task049_server_err_after_publish_is_a_runtime_event() {
    let peer = MockPeer::start();
    let mut runtime = connect(&peer);
    track::exchange_output_a::publish(&mut runtime, &shared(1)).expect("written");
    peer.expect(r#"PUB output-topic {"{urn:test}MessageA":{"Count":1}}"#);
    peer.send("-ERR Illegal-State topic not allowed");
    match runtime.recv_event_timeout(WAIT) {
        Some(RuntimeEvent::ServerError { error, details }) => {
            assert_eq!(error, "Illegal-State");
            assert_eq!(details.as_deref(), Some("topic not allowed"));
        }
        other => panic!("expected ServerError, got {other:?}"),
    }
    // The connection keeps serving after a server error.
    track::exchange_output_a::publish(&mut runtime, &shared(2)).expect("still usable");
    peer.expect(r#"PUB output-topic {"{urn:test}MessageA":{"Count":2}}"#);
    runtime.close().expect("close");
}

fn connect_error(config: RuntimeConfig) -> RuntimeError {
    match SleetRuntime::connect(config, TestCodec) {
        Ok(_) => panic!("connect must fail"),
        Err(error) => error,
    }
}

/// A server -ERR to INIT fails `connect`; no runtime is returned.
#[test]
fn task049_init_rejection_fails_connect() {
    let peer = MockPeer::start_with(InitReply::Frame(
        "-ERR Unsupported-Schema requested schema 000.1.0 does not match 002.5.0",
    ));
    let error = connect_error(RuntimeConfig::new(&peer.url, "svc-1", "000.1.0"));
    assert!(matches!(error, RuntimeError::Client(_)), "{error}");
    assert!(error.to_string().contains("Unsupported-Schema"), "{error}");
}

/// sleet-client rejects an illegal service ID before any connection, and
/// configuration errors are distinct from client errors.
#[test]
fn task049_connect_input_errors() {
    let error = connect_error(RuntimeConfig::new(
        "ws://127.0.0.1:9",
        "bad service",
        "000.1.0",
    ));
    assert!(is_invalid_input(&error, "bad service"), "{error}");
    let error = connect_error(RuntimeConfig::new("ws://127.0.0.1:9", "svc", ""));
    assert!(matches!(error, RuntimeError::InvalidConfig(_)), "{error}");
}
