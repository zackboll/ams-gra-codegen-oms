//! Task 050: GENERATED facade + GENERATED `ServiceCodec` -> runtime-rust ->
//! pinned sleet-client -> deterministic mock OWP peer. Task 049's
//! `mock_owp.rs` is unchanged and keeps using the handwritten test codec.
//! Task 051 adds the same path for the qualified NON-OAM `runtime-test.xsd`.

mod codec_sample;
mod common;

use ams_gra_oms_runtime_rust::{MessageDecodeError, RuntimeConfig, RuntimeEvent, SleetRuntime};
use ams_gra_oms_runtime_rust_facade_tests::codec_oam::service_api::function_loop as svc;
use ams_gra_oms_runtime_rust_facade_tests::codec_oam::service_codec::ServiceCodec;
use common::{MockPeer, WAIT};
use serde_json::Value;
use std::sync::mpsc;

/// Order-independent comparison of the PUB body against the exact expected
/// OMS JSON document (member order is not semantic).
fn pub_body(frame: &str, topic: &str) -> Value {
    let body = frame
        .strip_prefix(&format!("PUB {topic} "))
        .unwrap_or_else(|| panic!("expected PUB on {topic}, got {frame}"));
    serde_json::from_str(body).expect("PUB body is JSON")
}

/// Task 051: qualified NON-OAM `runtime-test.xsd` with its GENERATED codec:
/// typed payload -> generated `service_codec.rs` -> generated publish façade
/// -> `SleetRuntime<ServiceCodec>` -> pinned sleet-client -> mock OWP. No
/// handwritten codec. The runtime owns `{urn:test}MessageB`; the generated
/// codec owns `{urn:test}Count`.
#[test]
fn task051_non_oam_generated_codec_round_trips_through_mock_owp() {
    use ams_gra_oms_runtime_rust_facade_tests::runtime_test_codec::model as m;
    use ams_gra_oms_runtime_rust_facade_tests::runtime_test_codec::service_api::function_track as track;
    use ams_gra_oms_runtime_rust_facade_tests::runtime_test_codec::service_codec::ServiceCodec as NonOamCodec;

    let peer = MockPeer::start();
    let mut runtime = SleetRuntime::connect(
        RuntimeConfig::new(&peer.url, "svc-1", "000.1.0"),
        NonOamCodec,
    )
    .expect("connect");
    assert!(peer.next_text().starts_with("INIT "));

    let (seen_tx, seen) = mpsc::channel();
    let subscription = track::exchange_input_a::subscribe(
        &mut runtime,
        move |message: &track::exchange_input_a::Payload| {
            seen_tx.send(message.count.get()).expect("test alive");
        },
    )
    .expect("subscribe");
    peer.expect("SUB sub-1 {urn:test}MessageA input-topic");

    let value = m::SharedPayload {
        count: m::BoundedI64::new(7).expect("xs:int"),
    };
    track::exchange_output_b::publish(&mut runtime, &value).expect("publish");
    // Exact frame: serde_json preserves this one-member-per-object order.
    peer.expect(r#"PUB output-topic {"{urn:test}MessageB":{"{urn:test}Count":7}}"#);

    peer.send(r#"MSG sub-1 {"{urn:test}MessageA":{"{urn:test}Count":42}}"#);
    assert_eq!(seen.recv_timeout(WAIT), Ok(42));

    // The Task 049 handwritten-codec spelling (bare "Count") is NOT an alias
    // for the generated codec: a decode event, never a handler call.
    peer.send(r#"MSG sub-1 {"{urn:test}MessageA":{"Count":43}}"#);
    match runtime.recv_event_timeout(WAIT) {
        Some(RuntimeEvent::SubscriptionDecodeError {
            message_name,
            error: MessageDecodeError::Codec(error),
            ..
        }) => {
            assert_eq!(message_name, "{urn:test}MessageA");
            assert_eq!(error.message(), "SharedPayload: unknown member \"Count\"");
        }
        other => panic!("expected a codec decode-failure event, got {other:?}"),
    }
    assert!(seen.try_recv().is_err());

    subscription.unsubscribe().expect("unsubscribe");
    peer.expect("UNSUB sub-1");
    runtime.close().expect("close");
    peer.expect_closed();
}

#[test]
fn task050_generated_codec_round_trips_through_mock_owp() {
    let peer = MockPeer::start();
    let mut runtime = SleetRuntime::connect(
        RuntimeConfig::new(&peer.url, "svc-1", "000.1.0"),
        ServiceCodec,
    )
    .expect("connect");
    assert!(peer.next_text().starts_with("INIT "));

    let (seen_tx, seen) = mpsc::channel();
    let subscription = svc::exchange_input_notice::subscribe(
        &mut runtime,
        move |message: &svc::exchange_input_notice::Payload| {
            seen_tx.send(message.clone()).expect("test alive");
        },
    )
    .expect("subscribe");
    // OAM namespace: bare global element name.
    peer.expect("SUB sub-1 CodecNotice codec-topic");

    let value = codec_sample::sample();
    svc::exchange_output_echo::publish(&mut runtime, &value).expect("publish");
    let frame = peer.next_text();
    let document = pub_body(&frame, "codec-topic");
    assert_eq!(
        document,
        serde_json::json!({ "CodecEcho": codec_sample::sample_json() })
    );

    // Deliver the same body as CodecNotice: the generated codec decodes it and
    // the typed handler sees the generated model value.
    let body = serde_json::to_string(&codec_sample::sample_json()).expect("JSON");
    peer.send(&format!("MSG sub-1 {{\"CodecNotice\":{body}}}"));
    let received = seen.recv_timeout(WAIT).expect("typed MSG");
    assert_eq!(received.level.get(), -10);
    assert_eq!(received.signal, codec_sample::m::SignalCode::Value5G);

    // An invalid body is a decode event, never a handler call.
    peer.send(r#"MSG sub-1 {"CodecNotice":{"Enabled":"yes"}}"#);
    match runtime.recv_event_timeout(WAIT) {
        Some(event) => assert!(format!("{event:?}").contains("CodecPayload"), "{event:?}"),
        None => panic!("expected a decode-failure event"),
    }
    assert!(seen.try_recv().is_err());

    subscription.unsubscribe().expect("unsubscribe");
    peer.expect("UNSUB sub-1");
    runtime.close().expect("close");
    peer.expect_closed();
}
