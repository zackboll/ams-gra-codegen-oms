//! Task 049: MSG dispatch and unsubscribe of the Rust LA-CAL runtime,
//! driven through GENERATED endpoint functions.

mod common;

use ams_gra_oms_runtime_rust::{MessageDecodeError, RuntimeError, RuntimeEvent};
use ams_gra_oms_runtime_rust_facade_tests::runtime_test::model::SharedPayload;
use ams_gra_oms_runtime_rust_facade_tests::runtime_test::service_api::function_track as track;
use common::{MockPeer, QUIET, WAIT, connect};
use std::sync::mpsc;

type Check = fn(&MessageDecodeError) -> bool;

const REJECTED: [(&str, Check); 8] = [
    ("{}", |e| {
        matches!(e, MessageDecodeError::MemberCount { found: 0 })
    }),
    (r#"{"MessageA":{},"MessageB":{}}"#, |e| {
        matches!(e, MessageDecodeError::MemberCount { found: 2 })
    }),
    (r#"{"MessageB":{}}"#, |e| {
        matches!(e, MessageDecodeError::UnexpectedMember { .. })
    }),
    (r#"{"{urn:other}MessageA":{}}"#, |e| {
        matches!(e, MessageDecodeError::UnexpectedMember { .. })
    }),
    // The expected local name without its namespace: another element.
    (r#"{"MessageA":{"Count":1}}"#, |e| {
        matches!(e, MessageDecodeError::UnexpectedMember { .. })
    }),
    // Same payload shape, different global element.
    (r#"{"{urn:test}MessageB":{"Count":1}}"#, |e| {
        matches!(e, MessageDecodeError::UnexpectedMember { .. })
    }),
    // The right element, but a payload the codec rejects.
    (r#"{"{urn:test}MessageA":{"Count":"x"}}"#, |e| {
        matches!(e, MessageDecodeError::Codec(_))
    }),
    ("[1,2]", |e| matches!(e, MessageDecodeError::NotAnObject)),
];

/// Malformed or wrong-message JSON never reaches the typed handler; each
/// is a decode event and the subscription stays active.
#[test]
fn task049_wrong_message_json_is_rejected_and_subscription_survives() {
    let peer = MockPeer::start();
    let mut runtime = connect(&peer);
    let (seen_tx, seen) = mpsc::channel();
    let _handle = track::exchange_input_a::subscribe(&mut runtime, move |m: &SharedPayload| {
        seen_tx.send(m.count.get()).expect("test alive");
    })
    .expect("subscribe");
    peer.expect("SUB sub-1 {urn:test}MessageA input-topic");
    for (json, expected) in REJECTED {
        peer.send(&format!("MSG sub-1 {json}"));
        match runtime.recv_event_timeout(WAIT) {
            Some(RuntimeEvent::SubscriptionDecodeError {
                sid,
                message_name,
                error,
            }) => {
                assert_eq!(sid, "sub-1");
                assert_eq!(message_name, "{urn:test}MessageA");
                assert!(expected(&error), "{json}: {error:?}");
            }
            other => panic!("{json}: expected SubscriptionDecodeError, got {other:?}"),
        }
    }
    assert!(seen.recv_timeout(QUIET).is_err());
    peer.send(r#"MSG sub-1 {"{urn:test}MessageA":{"Count":5}}"#);
    assert_eq!(seen.recv_timeout(WAIT), Ok(5));
    runtime.close().expect("close");
}

/// MSG dispatch is by SID only: an unknown SID invokes no handler, even
/// with the subscribed message name and a valid payload.
#[test]
fn task049_unknown_sid_invokes_no_handler() {
    let peer = MockPeer::start();
    let mut runtime = connect(&peer);
    let (seen_tx, seen) = mpsc::channel();
    let _handle = track::exchange_input_a::subscribe(&mut runtime, move |m: &SharedPayload| {
        seen_tx.send(m.count.get()).expect("test alive");
    })
    .expect("subscribe");
    peer.expect("SUB sub-1 {urn:test}MessageA input-topic");
    peer.send(r#"MSG unknown-id {"{urn:test}MessageA":{"Count":9}}"#);
    match runtime.recv_event_timeout(WAIT) {
        Some(RuntimeEvent::UnknownSubscription { sid, payload }) => {
            assert_eq!(sid, "unknown-id");
            assert_eq!(payload, r#"{"{urn:test}MessageA":{"Count":9}}"#);
        }
        other => panic!("expected UnknownSubscription, got {other:?}"),
    }
    assert!(seen.recv_timeout(QUIET).is_err());
    runtime.close().expect("close");
}

/// After unsubscribe, an MSG for that SID never reaches the old handler.
/// `unsubscribe(self)` consumes the handle, so a second call on the same
/// handle cannot compile; a new subscription never reuses the SID.
#[test]
fn task049_unsubscribe_removes_dispatch() {
    let peer = MockPeer::start();
    let mut runtime = connect(&peer);
    let (seen_tx, seen) = mpsc::channel();
    let handle = track::exchange_input_a::subscribe(&mut runtime, move |m: &SharedPayload| {
        seen_tx.send(m.count.get()).expect("test alive");
    })
    .expect("subscribe");
    peer.expect("SUB sub-1 {urn:test}MessageA input-topic");
    handle.unsubscribe().expect("unsubscribe");
    peer.expect("UNSUB sub-1");
    peer.send(r#"MSG sub-1 {"{urn:test}MessageA":{"Count":1}}"#);
    assert!(matches!(
        runtime.recv_event_timeout(WAIT),
        Some(RuntimeEvent::UnknownSubscription { sid, .. }) if sid == "sub-1"
    ));
    assert!(seen.recv_timeout(QUIET).is_err());
    let next = track::exchange_input_a::subscribe(&mut runtime, |_: &SharedPayload| {})
        .expect("subscribe again");
    assert_eq!(next.id(), "sub-2");
    peer.expect("SUB sub-2 {urn:test}MessageA input-topic");
    runtime.close().expect("close");
    // Once the runtime is closed a handle can only report that.
    assert!(matches!(
        next.unsubscribe(),
        Err(RuntimeError::WorkerStopped)
    ));
}
