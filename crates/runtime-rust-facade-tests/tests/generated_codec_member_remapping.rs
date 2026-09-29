//! Task 054: GENERATED codecs where the Rust API identifier of a Record
//! field / Choice alternative is escaped but its OMS JSON member key is not.
//!
//! Three distinct identities:
//!
//! * source member identity: `FieldDecl::name` (`"Type"`);
//! * wire member identity: `FieldDecl::wire_name()` -> `"Type"` (OAM) or
//!   `"{urn:test}Type"` (qualified non-OAM);
//! * backend API identifier: `generated_record_field_name` -> `field_type`.
//!
//! Every codec here comes from the real `service-generate --with-codec`
//! (build.rs).

mod common;

use ams_gra_oms_runtime_rust::{OmsJsonCodec, RuntimeConfig, SleetRuntime};
use ams_gra_oms_runtime_rust_facade_tests::member_keywords::model as m;
use ams_gra_oms_runtime_rust_facade_tests::member_keywords::service_codec::ServiceCodec;
use ams_gra_oms_runtime_rust_facade_tests::member_keywords_qualified::model as q;
use ams_gra_oms_runtime_rust_facade_tests::member_keywords_qualified::service_api::function_qualified_keyword_loop as qsvc;
use ams_gra_oms_runtime_rust_facade_tests::member_keywords_qualified::service_codec::ServiceCodec as QualifiedCodec;
use common::{MockPeer, WAIT};
use serde_json::{Value, json};
use std::sync::mpsc;

type Int = m::BoundedI64<-2147483648, 2147483647>;

fn int(value: i64) -> Int {
    Int::new(value).expect("xs:int")
}

/// A KeywordRecord built ONLY through escaped Rust members.
fn keyword_record(pick: m::KeywordChoice) -> m::KeywordRecord {
    m::KeywordRecord {
        field_type: "inherited".to_owned(),
        range: m::BoundedVec::new(vec![int(10), int(20)]).expect("0..2"),
        operator: "op".to_owned(),
        delete: Some(true),
        field_self: int(5),
        ordinary: int(6),
        pick,
    }
}

fn keys(value: &Value) -> Vec<String> {
    let mut keys: Vec<_> = value.as_object().expect("object").keys().cloned().collect();
    keys.sort();
    keys
}

/// OAM: `field_type`/`field_self` encode as `"Type"`/`"Self"`; the escaped
/// Choice variant `AlternativeSelf` encodes as `"Self"`. Decoding the same
/// JSON restores every escaped member. No escaped spelling reaches the wire.
#[test]
fn task054_codec_wire_names_stay_source_names_and_round_trip() {
    let value = keyword_record(m::KeywordChoice::AlternativeSelf(int(3)));
    let encoded = ServiceCodec.encode_payload(&value).expect("encode");
    assert_eq!(
        encoded,
        json!({
            "Type": "inherited",
            "Range": [10, 20],
            "Operator": "op",
            "Delete": true,
            "Self": 5,
            "Ordinary": 6,
            "Pick": { "Self": 3 }
        })
    );
    let text = encoded.to_string();
    for escaped in [
        "field_type",
        "field_self",
        "Field_",
        "AlternativeSelf",
        "Alternative_",
    ] {
        assert!(
            !text.contains(escaped),
            "{escaped} leaked onto the wire: {text}"
        );
    }
    let decoded: m::KeywordRecord = ServiceCodec.decode_payload(&encoded).expect("decode");
    assert_eq!(decoded, value);
    assert_eq!(decoded.field_type, "inherited");
    assert_eq!(decoded.field_self.get(), 5);
    assert!(matches!(decoded.pick, m::KeywordChoice::AlternativeSelf(v) if v.get() == 3));

    // Every other Choice alternative, including Rust-safe `Type`, round-trips.
    for (pick, key) in [
        (
            m::KeywordChoice::Range(m::BoundedVec::new(vec![int(1)]).expect("1..3")),
            "Range",
        ),
        (m::KeywordChoice::Type("t".to_owned()), "Type"),
        (m::KeywordChoice::Delete(false), "Delete"),
        (m::KeywordChoice::Ordinary(int(4)), "Ordinary"),
    ] {
        let value = keyword_record(pick);
        let encoded = ServiceCodec.encode_payload(&value).expect("encode");
        assert_eq!(keys(&encoded["Pick"]), [key]);
        let decoded: m::KeywordRecord = ServiceCodec.decode_payload(&encoded).expect("decode");
        assert_eq!(decoded, value);
    }
    println!("TASK054 CODEC WIRE NAMES: PASSED");
}

/// The escaped host spelling is NOT an alias on the wire: a document keyed
/// by `field_type` (or the escaped variant) is rejected as an unknown member.
#[test]
fn task054_escaped_host_names_are_not_wire_aliases() {
    let good = ServiceCodec
        .encode_payload(&keyword_record(m::KeywordChoice::Ordinary(int(1))))
        .expect("encode");
    let mut renamed = good.clone();
    let object = renamed.as_object_mut().expect("object");
    let value = object.remove("Type").expect("Type");
    object.insert("field_type".to_owned(), value);
    let error =
        <ServiceCodec as OmsJsonCodec<m::KeywordRecord>>::decode_payload(&ServiceCodec, &renamed)
            .expect_err("an escaped host name is not a wire key");
    assert!(
        error.message().contains("field_type"),
        "{}",
        error.message()
    );

    let mut variant = good;
    variant["Pick"] = json!({ "AlternativeSelf": 3 });
    assert!(
        <ServiceCodec as OmsJsonCodec<m::KeywordRecord>>::decode_payload(&ServiceCodec, &variant)
            .is_err()
    );
}

fn qualified_value() -> q::QualifiedKeywordPayload {
    q::QualifiedKeywordPayload {
        field_type: "typed".to_owned(),
        ordinary: q::BoundedI64::new(7).expect("xs:int"),
        pick: q::QualifiedPick::AlternativeSelf(q::BoundedI64::new(9).expect("xs:int")),
    }
}

/// Qualified NON-OAM (Task 051 composes with Task 054): `field_type` encodes
/// as `"{urn:test}Type"`, never `"{urn:test}field_type"`.
#[test]
fn task054_qualified_non_oam_escaped_members_use_clark_source_keys() {
    let value = qualified_value();
    let encoded = QualifiedCodec.encode_payload(&value).expect("encode");
    assert_eq!(
        encoded,
        json!({
            "{urn:test}Type": "typed",
            "{urn:test}Ordinary": 7,
            "{urn:test}Pick": { "{urn:test}Self": 9 }
        })
    );
    assert!(!encoded.to_string().contains("field_type"));
    let decoded: q::QualifiedKeywordPayload =
        QualifiedCodec.decode_payload(&encoded).expect("decode");
    assert_eq!(decoded, value);
    println!("TASK054 QUALIFIED CODEC: PASSED");
}

/// The same qualified service through the GENERATED facade, runtime-rust,
/// pinned sleet-client and a deterministic mock OWP peer (pinned Sleet has a
/// known non-OAM limitation, so mock OWP is the evidence here).
#[test]
fn task054_qualified_escaped_members_round_trip_through_mock_owp() {
    let peer = MockPeer::start();
    let mut runtime = SleetRuntime::connect(
        RuntimeConfig::new(&peer.url, "svc-1", "000.1.0"),
        QualifiedCodec,
    )
    .expect("connect");
    assert!(peer.next_text().starts_with("INIT "));

    let (seen_tx, seen) = mpsc::channel();
    let subscription = qsvc::exchange_input_qualified_keyword::subscribe(
        &mut runtime,
        move |message: &qsvc::exchange_input_qualified_keyword::Payload| {
            seen_tx
                .send((message.field_type.clone(), message.pick.clone()))
                .expect("test alive");
        },
    )
    .expect("subscribe");
    peer.expect("SUB sub-1 {urn:test}QualifiedKeywordNotice qualified-keyword-topic");

    qsvc::exchange_output_qualified_keyword::publish(&mut runtime, &qualified_value())
        .expect("publish");
    let frame = peer.next_text();
    let body = frame
        .strip_prefix("PUB qualified-keyword-topic ")
        .unwrap_or_else(|| panic!("expected PUB, got {frame}"));
    let body: Value = serde_json::from_str(body).expect("PUB body is JSON");
    assert_eq!(
        body,
        json!({
            "{urn:test}QualifiedKeywordNotice": {
                "{urn:test}Type": "typed",
                "{urn:test}Ordinary": 7,
                "{urn:test}Pick": { "{urn:test}Self": 9 }
            }
        })
    );

    peer.send(
        r#"MSG sub-1 {"{urn:test}QualifiedKeywordNotice":{"{urn:test}Type":"echo","{urn:test}Ordinary":1,"{urn:test}Pick":{"{urn:test}Self":2}}}"#,
    );
    let (field_type, pick) = seen.recv_timeout(WAIT).expect("handler called");
    assert_eq!(field_type, "echo");
    assert!(matches!(pick, q::QualifiedPick::AlternativeSelf(v) if v.get() == 2));

    subscription.unsubscribe().expect("unsubscribe");
    peer.expect("UNSUB sub-1");
    runtime.close().expect("close");
    peer.expect_closed();
    println!("TASK054 QUALIFIED MOCK OWP: PASSED");
}
