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

/// Task 052: the `codec-hexbinary` service (BlobNotice with direct, named,
/// repeated, and Choice `xs:hexBinary` members) with its GENERATED codec:
/// typed bytes
/// -> generated codec -> generated publish façade -> `SleetRuntime` ->
/// pinned sleet-client -> mock OWP. The PUB body carries CANONICAL uppercase
/// hex; a MSG spelled in lowercase decodes to the same octets; an invalid
/// hexBinary is a decode event, never a handler call.
#[test]
fn task052_hex_binary_generated_codec_round_trips_through_mock_owp() {
    use ams_gra_oms_runtime_rust_facade_tests::codec_hexbinary::model as m;
    use ams_gra_oms_runtime_rust_facade_tests::codec_hexbinary::service_api::function_blob as blob;
    use ams_gra_oms_runtime_rust_facade_tests::codec_hexbinary::service_codec::ServiceCodec as HexCodec;

    let peer = MockPeer::start();
    let mut runtime =
        SleetRuntime::connect(RuntimeConfig::new(&peer.url, "svc-1", "000.1.0"), HexCodec)
            .expect("connect");
    assert!(peer.next_text().starts_with("INIT "));

    let (seen_tx, seen) = mpsc::channel();
    let subscription = blob::exchange_input_blob::subscribe(
        &mut runtime,
        move |message: &blob::exchange_input_blob::Payload| {
            seen_tx.send(message.clone()).expect("test alive");
        },
    )
    .expect("subscribe");
    peer.expect("SUB sub-1 BlobNotice blob-topic");

    let int = |n| m::BoundedI64::new(n).expect("xs:int");
    let value = m::BlobPayload {
        id: int(9),
        data: vec![0x00, 0x0A, 0xEE, 0xFF],
        maybedata: None,
        chunks: m::BoundedVec::new(vec![vec![0xAB]]).expect("1"),
        stream: m::UnboundedVec::new(Vec::new()).expect("0"),
        blob: m::BlobBytes::new(vec![0x0F]),
        alias: None,
        atomic: m::AtomicLike::HexBinaryValue(vec![0x00, 0xA1, 0xFF]),
    };
    blob::exchange_output_blob::publish(&mut runtime, &value).expect("publish");
    let document = pub_body(&peer.next_text(), "blob-topic");
    assert_eq!(
        document,
        serde_json::json!({ "BlobNotice": {
            "Id": 9,
            "Data": "000AEEFF",
            "Chunks": ["AB"],
            "Blob": "0F",
            "Atomic": { "HexBinaryValue": "00A1FF" }
        }})
    );

    // Lowercase hex on the wire decodes to exactly the same octets.
    peer.send(
        r#"MSG sub-1 {"BlobNotice":{"Id":9,"Data":"000aeeff","Blob":"0f","Atomic":{"HexBinaryValue":"00a1ff"}}}"#,
    );
    let received = seen.recv_timeout(WAIT).expect("typed MSG");
    assert_eq!(received.data, [0x00, 0x0A, 0xEE, 0xFF]);
    assert_eq!(received.blob.as_slice(), [0x0F]);
    assert_eq!(
        received.atomic,
        m::AtomicLike::HexBinaryValue(vec![0x00, 0xA1, 0xFF])
    );

    // An odd-length hexBinary is a codec decode event, never a handler call.
    peer.send(
        r#"MSG sub-1 {"BlobNotice":{"Id":9,"Data":"000","Blob":"0F","Atomic":{"IntValue":1}}}"#,
    );
    match runtime.recv_event_timeout(WAIT) {
        Some(RuntimeEvent::SubscriptionDecodeError {
            message_name,
            error: MessageDecodeError::Codec(error),
            ..
        }) => {
            assert_eq!(message_name, "BlobNotice");
            assert_eq!(
                error.message(),
                "BlobPayload.Data: \"000\" is not hexBinary: odd number of hexadecimal digits"
            );
        }
        other => panic!("expected a codec decode-failure event, got {other:?}"),
    }
    assert!(seen.try_recv().is_err());

    subscription.unsubscribe().expect("unsubscribe");
    peer.expect("UNSUB sub-1");
    runtime.close().expect("close");
    peer.expect_closed();
}

/// Task 053: the `constrained-binary` service. A typed constrained carrier
/// is encoded as canonical uppercase hex; lowercase hex of a LEGAL octet
/// count reaches the typed handler as the checked carrier; lexically valid
/// hex of an ILLEGAL octet count is a decode event from the GENERATED model
/// constructor, never a handler call.
#[test]
fn task053_constrained_binary_round_trips_through_mock_owp() {
    use ams_gra_oms_runtime_rust_facade_tests::constrained_binary::model as m;
    use ams_gra_oms_runtime_rust_facade_tests::constrained_binary::service_api::function_constrained_blob as blob;
    use ams_gra_oms_runtime_rust_facade_tests::constrained_binary::service_codec::ServiceCodec as BlobCodec;

    let peer = MockPeer::start();
    let mut runtime =
        SleetRuntime::connect(RuntimeConfig::new(&peer.url, "svc-1", "000.1.0"), BlobCodec)
            .expect("connect");
    assert!(peer.next_text().starts_with("INIT "));

    let (seen_tx, seen) = mpsc::channel();
    let subscription = blob::exchange_input_constrained_blob::subscribe(
        &mut runtime,
        move |message: &blob::exchange_input_constrained_blob::Payload| {
            seen_tx.send(message.clone()).expect("test alive");
        },
    )
    .expect("subscribe");
    peer.expect("SUB sub-1 ConstrainedBlobNotice constrained-blob-topic");

    let four = |bytes: [u8; 4]| m::Exact4::new(bytes.to_vec()).expect("4 octets");
    let value = m::ConstrainedBlobPayload {
        id: m::BoundedI64::new(53).expect("xs:int"),
        exact: four([0x00, 0x0A, 0xEE, 0xFF]),
        maybemin: None,
        bounded: m::BoundedVec::new(Vec::new()).expect("0 of 0..3"),
        boundedrequired: m::BoundedVec::new(vec![
            m::Between2And6::new(vec![0xAB, 0xCD]).expect("2"),
        ])
        .expect("1 of 1..3"),
        stream: m::UnboundedVec::new(Vec::new()).expect("0.."),
        streamrequired: m::UnboundedVec::new(vec![four([1, 2, 3, 4])]).expect("1.."),
        zero: m::ZeroBlob::new(Vec::new()).expect("0"),
        base: None,
        middle: None,
        hashed: m::BlobExact::new(vec![0xEE; 8]).expect("8"),
        plain: m::PlainBytes::new(vec![0x0F]),
        pick: m::BlobChoice::Fixed(four([0xDE, 0xAD, 0xBE, 0xEF])),
    };
    blob::exchange_output_constrained_blob::publish(&mut runtime, &value).expect("publish");
    let document = pub_body(&peer.next_text(), "constrained-blob-topic");
    assert_eq!(
        document,
        serde_json::json!({ "ConstrainedBlobNotice": {
            "Id": 53,
            "Exact": "000AEEFF",
            "BoundedRequired": ["ABCD"],
            "StreamRequired": ["01020304"],
            "Zero": "",
            "Hashed": "EEEEEEEEEEEEEEEE",
            "Plain": "0F",
            "Pick": { "Fixed": "DEADBEEF" }
        }})
    );

    // Lowercase hex of LEGAL octet counts reaches the typed handler.
    peer.send(
        r#"MSG sub-1 {"ConstrainedBlobNotice":{"Id":53,"Exact":"000aeeff","BoundedRequired":["abcd"],"StreamRequired":["01020304"],"Zero":"","Hashed":"eeeeeeeeeeeeeeee","Plain":"0f","Pick":{"Fixed":"deadbeef"}}}"#,
    );
    let received = seen.recv_timeout(WAIT).expect("typed MSG");
    assert_eq!(received, value);
    assert_eq!(received.exact.as_slice(), [0x00, 0x0A, 0xEE, 0xFF]);

    // Lexically VALID hex, ILLEGAL octet count (3 for Exact4): the generated
    // model constructor rejects it; the handler never runs.
    peer.send(
        r#"MSG sub-1 {"ConstrainedBlobNotice":{"Id":53,"Exact":"000aee","BoundedRequired":["abcd"],"StreamRequired":["01020304"],"Zero":"","Hashed":"eeeeeeeeeeeeeeee","Plain":"0f","Pick":{"Fixed":"deadbeef"}}}"#,
    );
    match runtime.recv_event_timeout(WAIT) {
        Some(RuntimeEvent::SubscriptionDecodeError {
            message_name,
            error: MessageDecodeError::Codec(error),
            ..
        }) => {
            assert_eq!(message_name, "ConstrainedBlobNotice");
            assert_eq!(
                error.message(),
                "ConstrainedBlobPayload.Exact: value rejected by generated Exact4::new"
            );
        }
        other => panic!("expected a codec decode-failure event, got {other:?}"),
    }
    assert!(seen.try_recv().is_err());

    subscription.unsubscribe().expect("unsubscribe");
    peer.expect("UNSUB sub-1");
    runtime.close().expect("close");
    peer.expect_closed();
    println!("MOCK OWP CONSTRAINED BINARY: PASSED");
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

/// Task 057: the `codec-duration` service. A typed duration is published as
/// its stored lexical spelling; a MSG with collapsible whitespace reaches the
/// typed handler collapsed; an invalid duration lexical is a decode event from
/// the GENERATED model constructor, never a handler call.
#[test]
fn task057_duration_generated_codec_round_trips_through_mock_owp() {
    use ams_gra_oms_runtime_rust_facade_tests::codec_duration::model as m;
    use ams_gra_oms_runtime_rust_facade_tests::codec_duration::service_api::function_span as span;
    use ams_gra_oms_runtime_rust_facade_tests::codec_duration::service_codec::ServiceCodec as DurationCodec;

    let peer = MockPeer::start();
    let mut runtime = SleetRuntime::connect(
        RuntimeConfig::new(&peer.url, "svc-1", "000.1.0"),
        DurationCodec,
    )
    .expect("connect");
    assert!(peer.next_text().starts_with("INIT "));

    let (seen_tx, seen) = mpsc::channel();
    let subscription = span::exchange_input_span::subscribe(
        &mut runtime,
        move |message: &span::exchange_input_span::Payload| {
            seen_tx.send(message.clone()).expect("test alive");
        },
    )
    .expect("subscribe");
    peer.expect("SUB sub-1 DurationNotice span-topic");

    let d = |text| m::XmlSchemaDuration::new(text).expect("duration");
    let value = m::DurationPayload {
        named: m::SpanType::new("P1DT2H").expect("named"),
        step: d("PT0.25S"),
        maybestep: None,
        steps: m::BoundedVec::new(vec![d("-P1D")]).expect("1"),
        history: m::UnboundedVec::new(Vec::new()).expect("0"),
        pick: m::IntervalChoice::Every(d("P12M")),
    };
    span::exchange_output_span::publish(&mut runtime, &value).expect("publish");
    let document = pub_body(&peer.next_text(), "span-topic");
    assert_eq!(
        document,
        serde_json::json!({ "DurationNotice": {
            "Named": "P1DT2H",
            "Step": "PT0.25S",
            "Steps": ["-P1D"],
            "Pick": { "Every": "P12M" }
        }})
    );

    peer.send(
        r#"MSG sub-1 {"DurationNotice":{"Named":" P1Y ","Step":"PT1S","MaybeStep":"\tP0D\n","Pick":{"Count":2}}}"#,
    );
    let received = seen.recv_timeout(WAIT).expect("typed MSG");
    assert_eq!(received.named.as_str(), "P1Y");
    assert_eq!(
        received.maybestep.as_ref().expect("present").as_str(),
        "P0D"
    );

    peer.send(r#"MSG sub-1 {"DurationNotice":{"Named":"P1Y","Step":"PT1H1H","Pick":{"Count":2}}}"#);
    match runtime.recv_event_timeout(WAIT) {
        Some(RuntimeEvent::SubscriptionDecodeError {
            message_name,
            error: MessageDecodeError::Codec(error),
            ..
        }) => {
            assert_eq!(message_name, "DurationNotice");
            assert!(
                error.message().starts_with("DurationPayload.Step")
                    && error.message().contains("XmlSchemaDuration"),
                "{}",
                error.message()
            );
        }
        other => panic!("expected a codec decode-failure event, got {other:?}"),
    }
    assert!(seen.try_recv().is_err());

    subscription.unsubscribe().expect("unsubscribe");
    peer.expect("UNSUB sub-1");
    runtime.close().expect("close");
    peer.expect_closed();
}

/// Task 058: the `codec-bounded-ascii` service. Typed bounded-ASCII carriers
/// (including the zero-length `EmptyType` shape) are published as their
/// stored text; a MSG reaches the typed handler with SPACE preserved; an
/// invalid lexical is a decode event from the GENERATED constructor, never a
/// handler call.
#[test]
fn task058_bounded_ascii_generated_codec_round_trips_through_mock_owp() {
    use ams_gra_oms_runtime_rust_facade_tests::codec_bounded_ascii::model as m;
    use ams_gra_oms_runtime_rust_facade_tests::codec_bounded_ascii::service_api::function_tail as tail;
    use ams_gra_oms_runtime_rust_facade_tests::codec_bounded_ascii::service_codec::ServiceCodec as BoundedCodec;

    let peer = MockPeer::start();
    let mut runtime = SleetRuntime::connect(
        RuntimeConfig::new(&peer.url, "svc-1", "000.1.0"),
        BoundedCodec,
    )
    .expect("connect");
    assert!(peer.next_text().starts_with("INIT "));

    let (seen_tx, seen) = mpsc::channel();
    let subscription = tail::exchange_input_tail::subscribe(
        &mut runtime,
        move |message: &tail::exchange_input_tail::Payload| {
            seen_tx.send(message.clone()).expect("test alive");
        },
    )
    .expect("subscribe");
    peer.expect("SUB sub-1 BoundedNotice tail-topic");

    let empty = m::MarkerType::new("").expect("zero-length");
    let value = m::BoundedPayload {
        marker: empty.clone(),
        tail: m::TailType::new("N123AB  ").expect("tail"),
        label: None,
        markers: m::BoundedVec::new(vec![empty.clone()]).expect("1"),
        codes: m::UnboundedVec::new(Vec::new()).expect("0"),
        pick: m::PickChoice::Empty(empty),
    };
    tail::exchange_output_tail::publish(&mut runtime, &value).expect("publish");
    let document = pub_body(&peer.next_text(), "tail-topic");
    assert_eq!(
        document,
        serde_json::json!({ "BoundedNotice": {
            "Marker": "",
            "Tail": "N123AB  ",
            "Markers": [""],
            "Pick": { "Empty": "" }
        }})
    );

    peer.send(
        r#"MSG sub-1 {"BoundedNotice":{"Marker":"","Tail":" 7 7 7 7","Label":"a b","Pick":{"Launch":"Z"}}}"#,
    );
    let received = seen.recv_timeout(WAIT).expect("typed MSG");
    assert_eq!(received.tail.as_str(), " 7 7 7 7");
    assert_eq!(received.label.as_ref().expect("present").as_str(), "a b");

    peer.send(
        r#"MSG sub-1 {"BoundedNotice":{"Marker":"","Tail":"n123ab  ","Pick":{"Launch":"Z"}}}"#,
    );
    match runtime.recv_event_timeout(WAIT) {
        Some(RuntimeEvent::SubscriptionDecodeError {
            message_name,
            error: MessageDecodeError::Codec(error),
            ..
        }) => {
            assert_eq!(message_name, "BoundedNotice");
            assert!(
                error.message().starts_with("BoundedPayload.Tail")
                    && error.message().contains("TailType"),
                "{}",
                error.message()
            );
        }
        other => panic!("expected a codec decode-failure event, got {other:?}"),
    }
    assert!(seen.try_recv().is_err());

    subscription.unsubscribe().expect("unsubscribe");
    peer.expect("UNSUB sub-1");
    runtime.close().expect("close");
    peer.expect_closed();
}
