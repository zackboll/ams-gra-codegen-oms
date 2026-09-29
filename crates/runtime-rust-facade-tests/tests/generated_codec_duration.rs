//! Task 057: GENERATED xs:duration OMS JSON codecs.
//!
//! Evidence (OMSC-SPC-013 Rev B, pinned `open-arsenal/oms` @ 726272bd):
//! section 6.1.4 maps a simple type by its {primitive type definition} only;
//! xs:duration matches none of cases 1-4, so case 5 "Otherwise string"
//! applies, and section 6.1.5.4 makes the string's characters the XML
//! lexical value. The codec therefore writes `carrier.as_str()` verbatim and
//! decodes a JSON string ONLY through the generated checked constructor --
//! the codec contains no duration grammar of its own.

use ams_gra_oms_runtime_rust::{CodecError, OmsJsonCodec};
use ams_gra_oms_runtime_rust_facade_tests::codec_duration::model as m;
use ams_gra_oms_runtime_rust_facade_tests::codec_duration::service_codec::ServiceCodec;
use serde_json::{Value, json};

fn direct(text: &str) -> m::XmlSchemaDuration {
    m::XmlSchemaDuration::new(text).expect("valid duration")
}

fn payload() -> m::DurationPayload {
    m::DurationPayload {
        named: m::SpanType::new("P1Y2M3DT4H5M6.789S").expect("named"),
        step: direct("PT30S"),
        maybestep: Some(direct("-P1D")),
        steps: m::BoundedVec::new(vec![direct("P0D"), direct("PT0.5S")]).expect("0..3"),
        history: m::UnboundedVec::new(vec![direct("P12M"), direct("P1Y")]).expect("0.."),
        pick: m::IntervalChoice::Every(direct("PT1H")),
    }
}

fn encode(value: &m::DurationPayload) -> Value {
    ServiceCodec.encode_payload(value).expect("encode")
}

fn decode(value: &Value) -> Result<m::DurationPayload, CodecError> {
    ServiceCodec.decode_payload(value)
}

/// Section 22: the wire spelling is exactly the carrier's stored spelling
/// (never canonicalized: `P12M` stays `P12M`), and decoding restores it.
#[test]
fn task057_duration_round_trips_with_its_stored_spelling() {
    let encoded = encode(&payload());
    assert_eq!(
        encoded,
        json!({
            "Named": "P1Y2M3DT4H5M6.789S",
            "Step": "PT30S",
            "MaybeStep": "-P1D",
            "Steps": ["P0D", "PT0.5S"],
            "History": ["P12M", "P1Y"],
            "Pick": { "Every": "PT1H" }
        })
    );
    let decoded = decode(&encoded).expect("decode");
    assert_eq!(decoded.named.as_str(), "P1Y2M3DT4H5M6.789S");
    assert_eq!(
        decoded.maybestep.as_ref().expect("present").as_str(),
        "-P1D"
    );
    let history: Vec<_> = decoded
        .history
        .as_slice()
        .iter()
        .map(|d| d.as_str())
        .collect();
    assert_eq!(history, ["P12M", "P1Y"]);
    match &decoded.pick {
        m::IntervalChoice::Every(every) => assert_eq!(every.as_str(), "PT1H"),
        other => panic!("unexpected {other:?}"),
    }
    assert_eq!(encode(&decoded), encoded);

    // Absent optional / empty repeated members produce no member at all.
    let mut sparse = payload();
    sparse.maybestep = None;
    sparse.steps = m::BoundedVec::new(Vec::new()).expect("0..3");
    sparse.history = m::UnboundedVec::new(Vec::new()).expect("0..");
    sparse.pick = m::IntervalChoice::Count(m::BoundedI64::new(3).expect("xs:int"));
    let encoded = encode(&sparse);
    assert_eq!(
        encoded,
        json!({ "Named": "P1Y2M3DT4H5M6.789S", "Step": "PT30S", "Pick": { "Count": 3 } })
    );
    assert!(decode(&encoded).expect("decode").maybestep.is_none());
}

/// The generated model constructor applies whiteSpace=collapse ONCE; the
/// codec does no second normalization, so the re-encoded wire value is the
/// collapsed stored spelling.
#[test]
fn task057_decode_collapses_once_through_the_model_constructor() {
    let mut document = encode(&payload());
    document["Step"] = json!(" \t PT30S \n");
    document["Steps"] = json!(["\r\nP0D "]);
    let decoded = decode(&document).expect("collapsible whitespace is legal");
    assert_eq!(decoded.step.as_str(), "PT30S");
    assert_eq!(decoded.steps.as_slice()[0].as_str(), "P0D");
    let reencoded = encode(&decoded);
    assert_eq!(reencoded["Step"], json!("PT30S"));
    assert_eq!(reencoded["Steps"], json!(["P0D"]));
}

/// Invalid lexical forms are rejected BY THE GENERATED CONSTRUCTOR at the
/// member's path, in every position: named, required, optional, bounded,
/// unbounded, and Choice.
#[test]
fn task057_invalid_duration_lexicals_are_rejected_by_the_model() {
    for (member, bad, path, carrier) in [
        ("Named", json!("P"), "DurationPayload.Named", "SpanType"),
        (
            "Step",
            json!("PT"),
            "DurationPayload.Step",
            "XmlSchemaDuration",
        ),
        (
            "MaybeStep",
            json!("+P1D"),
            "DurationPayload.MaybeStep",
            "XmlSchemaDuration",
        ),
        (
            "Steps",
            json!(["P1D", "P1H"]),
            "DurationPayload.Steps[1]",
            "XmlSchemaDuration",
        ),
        (
            "History",
            json!(["PT1.S"]),
            "DurationPayload.History[0]",
            "XmlSchemaDuration",
        ),
        (
            "Pick",
            json!({ "Every": "p1d" }),
            "DurationPayload.Pick.Every",
            "XmlSchemaDuration",
        ),
    ] {
        let mut document = encode(&payload());
        document[member] = bad;
        let error = decode(&document).expect_err(member);
        assert!(
            error.message().starts_with(path) && error.message().contains(carrier),
            "{member}: {}",
            error.message()
        );
    }
}

/// A duration is a JSON string only: numbers, booleans, arrays, objects and
/// null are type mismatches, never coerced.
#[test]
fn task057_non_string_json_is_rejected() {
    for bad in [
        json!(86400),
        json!(1.5),
        json!(true),
        json!(null),
        json!(["P1D"]),
        json!({}),
    ] {
        for member in ["Named", "Step", "MaybeStep"] {
            let mut document = encode(&payload());
            document[member] = bad.clone();
            let error = decode(&document).expect_err("non-string");
            assert!(
                error.message().contains("string"),
                "{member} {bad}: {}",
                error.message()
            );
        }
    }
}
