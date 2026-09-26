//! Task 049: GENERATED Rust façade -> runtime-rust -> pinned sleet-client ->
//! a deterministic mock OWP peer, asserting the exact wire frames.
//!
//! Every operation below is a generated endpoint function; the application
//! code never supplies a topic, namespace, message name, group, SID, OWP
//! command, or JSON string.

mod common;

use ams_gra_oms_runtime_rust::{RuntimeConfig, RuntimeEvent, SleetRuntime};
use ams_gra_oms_runtime_rust_facade_tests::codec::{TestCodec, oam_shared, shared};
use ams_gra_oms_runtime_rust_facade_tests::runtime_oam::service_api::function_loop as oam;
use ams_gra_oms_runtime_rust_facade_tests::runtime_test;
use ams_gra_oms_runtime_rust_facade_tests::runtime_test::service_api::function_track as track;
use common::{MockPeer, WAIT};
use serde_json::Value;
use std::sync::mpsc;
use std::time::Duration;

fn connect(peer: &MockPeer, schema: &str) -> SleetRuntime<TestCodec> {
    let runtime = SleetRuntime::connect(RuntimeConfig::new(&peer.url, "svc-1", schema), TestCodec)
        .expect("connect");
    assert_eq!(peer.offered_protocol().as_deref(), Some("owp"));
    let init = peer.next_text();
    let body: Value =
        serde_json::from_str(init.strip_prefix("INIT ").expect("INIT frame")).expect("INIT JSON");
    assert_eq!(
        body,
        serde_json::json!({
            "versions": ["1.0"],
            "schema": schema,
            "verbose": false,
            "service_id": "svc-1",
        })
    );
    runtime
}

/// The central Task 049 path, in the order the task specifies:
/// INIT, SUB, PUB, MSG -> typed handler, UNSUB, close.
#[test]
fn task049_generated_facade_round_trips_through_mock_owp() {
    let peer = MockPeer::start();
    let mut runtime = connect(&peer, "000.1.0");

    let (seen_tx, seen) = mpsc::channel();
    let subscription = track::exchange_input_a::subscribe(
        &mut runtime,
        move |message: &track::exchange_input_a::Payload| {
            seen_tx.send(message.count.get()).expect("test alive");
        },
    )
    .expect("subscribe");
    assert_eq!(subscription.id(), "sub-1");
    peer.expect("SUB sub-1 {urn:test}MessageA input-topic");

    track::exchange_output_b::publish(&mut runtime, &shared(7)).expect("publish");
    peer.expect(r#"PUB output-topic {"{urn:test}MessageB":{"Count":7}}"#);

    peer.send(r#"MSG sub-1 {"{urn:test}MessageA":{"Count":42}}"#);
    assert_eq!(seen.recv_timeout(WAIT), Ok(42));

    subscription.unsubscribe().expect("unsubscribe");
    peer.expect("UNSUB sub-1");

    assert!(runtime.try_recv_event().is_none());
    runtime.close().expect("close");
    peer.expect_closed();
}

/// Same payload type, same topic, different global message: the PUB frames
/// differ exactly at the global JSON member.
#[test]
fn task049_same_payload_different_message_differs_on_the_wire() {
    let peer = MockPeer::start();
    let mut runtime = connect(&peer, "000.1.0");
    let value: runtime_test::model::SharedPayload = shared(7);
    track::exchange_output_a::publish(&mut runtime, &value).expect("publish A");
    track::exchange_output_b::publish(&mut runtime, &value).expect("publish B");
    peer.expect(r#"PUB output-topic {"{urn:test}MessageA":{"Count":7}}"#);
    peer.expect(r#"PUB output-topic {"{urn:test}MessageB":{"Count":7}}"#);
    // A second payload type served by the same codec provider.
    track::exchange_output_c::publish(
        &mut runtime,
        &runtime_test::model::OtherPayload {
            label: "x y".to_owned(),
        },
    )
    .expect("publish C");
    peer.expect(r#"PUB output-topic {"{urn:test}MessageC":{"Label":"x y"}}"#);
    runtime.close().expect("close");
}

/// An authored OWP-legal group is sent verbatim as the fourth SUB field;
/// no group means no fourth field. SIDs are monotonic per connection.
#[test]
fn task049_subscription_group_is_on_the_wire_or_absent() {
    let peer = MockPeer::start();
    let mut runtime = connect(&peer, "000.1.0");
    let grouped = track::exchange_input_a_grouped::subscribe(
        &mut runtime,
        |_: &runtime_test::model::SharedPayload| {},
    )
    .expect("grouped");
    peer.expect("SUB sub-1 {urn:test}MessageA input-topic worker-group-1");
    let plain = track::exchange_input_a::subscribe(
        &mut runtime,
        |_: &runtime_test::model::SharedPayload| {},
    )
    .expect("plain");
    peer.expect("SUB sub-2 {urn:test}MessageA input-topic");
    assert_eq!((grouped.id(), plain.id()), ("sub-1", "sub-2"));
    runtime.close().expect("close");
}

/// The OAM namespace special case: bare local names in SUB and in the
/// global JSON member, both from the one shared formatter.
#[test]
fn task049_oam_namespace_uses_bare_local_names() {
    let peer = MockPeer::start();
    let mut runtime = connect(&peer, "000.1.0");
    let (seen_tx, seen) = mpsc::channel();
    let subscription = oam::exchange_input_a::subscribe(
        &mut runtime,
        move |m: &oam::exchange_input_a::Payload| {
            seen_tx.send(m.count.get()).expect("test alive");
        },
    )
    .expect("subscribe");
    peer.expect("SUB sub-1 MessageA loop-topic");
    oam::exchange_output_a::publish(&mut runtime, &oam_shared(3)).expect("publish");
    peer.expect(r#"PUB loop-topic {"MessageA":{"Count":3}}"#);
    peer.send(r#"MSG sub-1 {"MessageA":{"Count":3}}"#);
    assert_eq!(seen.recv_timeout(WAIT), Ok(3));
    // The Clark-notation spelling is a DIFFERENT element in OAM.
    peer.send(r#"MSG sub-1 {"{https://www.vdl.afrl.af.mil/programs/oam}MessageA":{"Count":4}}"#);
    assert!(matches!(
        runtime.recv_event_timeout(WAIT),
        Some(RuntimeEvent::SubscriptionDecodeError { .. })
    ));
    assert!(seen.recv_timeout(Duration::from_millis(200)).is_err());
    subscription.unsubscribe().expect("unsubscribe");
    peer.expect("UNSUB sub-1");
    runtime.close().expect("close");
}
