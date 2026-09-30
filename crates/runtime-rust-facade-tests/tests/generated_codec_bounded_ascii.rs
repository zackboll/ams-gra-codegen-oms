//! Task 058: GENERATED OMS JSON codecs for bounded-ASCII String carriers.
//!
//! OMSC-SPC-013 Rev B section 6.1.4 case 5: an xs:string-derived simple type
//! is a JSON string whose characters are the XML lexical value (6.1.5.4).
//! The codec writes `carrier.as_str()` verbatim and decodes a JSON string
//! ONLY through the generated checked constructor; it holds no alphabet or
//! length rule of its own, and never trims (whiteSpace = preserve).

use ams_gra_oms_runtime_rust::{CodecError, OmsJsonCodec};
use ams_gra_oms_runtime_rust_facade_tests::codec_bounded_ascii::model as m;
use ams_gra_oms_runtime_rust_facade_tests::codec_bounded_ascii::service_codec::ServiceCodec;
use serde_json::{Value, json};

fn empty() -> m::MarkerType {
    m::MarkerType::new("").expect("the zero-length profile accepts \"\"")
}

fn payload() -> m::BoundedPayload {
    m::BoundedPayload {
        marker: empty(),
        tail: m::TailType::new("AB12 XY ").expect("tail"),
        label: Some(m::LabelType::new(" a-b_C ").expect("label")),
        markers: m::BoundedVec::new(vec![empty(), empty()]).expect("0..3"),
        codes: m::UnboundedVec::new(vec![
            m::CodeType::new("aB3z").expect("code"),
            m::CodeType::new("0000").expect("code"),
        ])
        .expect("0.."),
        pick: m::PickChoice::Launch(m::LaunchType::new("ABC").expect("launch")),
    }
}

fn encode(value: &m::BoundedPayload) -> Value {
    ServiceCodec.encode_payload(value).expect("encode")
}

fn decode(value: &Value) -> Result<m::BoundedPayload, CodecError> {
    ServiceCodec.decode_payload(value)
}

/// The wire value is exactly the stored text, including leading/trailing
/// SPACE and the zero-length string; decoding restores it unchanged.
#[test]
fn task058_bounded_ascii_round_trips_its_stored_text() {
    let encoded = encode(&payload());
    assert_eq!(
        encoded,
        json!({
            "Marker": "",
            "Tail": "AB12 XY ",
            "Label": " a-b_C ",
            "Markers": ["", ""],
            "Codes": ["aB3z", "0000"],
            "Pick": { "Launch": "ABC" }
        })
    );
    let decoded = decode(&encoded).expect("decode");
    assert_eq!(decoded.marker.as_str(), "");
    assert_eq!(decoded.tail.as_str(), "AB12 XY ");
    assert_eq!(decoded.label.as_ref().expect("present").as_str(), " a-b_C ");
    assert_eq!(decoded.markers.as_slice().len(), 2);
    assert_eq!(encode(&decoded), encoded);

    let mut sparse = payload();
    sparse.label = None;
    sparse.markers = m::BoundedVec::new(Vec::new()).expect("0..3");
    sparse.codes = m::UnboundedVec::new(Vec::new()).expect("0..");
    sparse.pick = m::PickChoice::Empty(empty());
    let encoded = encode(&sparse);
    assert_eq!(
        encoded,
        json!({ "Marker": "", "Tail": "AB12 XY ", "Pick": { "Empty": "" } })
    );
    assert!(matches!(
        decode(&encoded).expect("decode").pick,
        m::PickChoice::Empty(_)
    ));
}

/// An invalid lexical value is rejected BY THE GENERATED CONSTRUCTOR at the
/// member's path, in every position; nothing is trimmed into validity.
#[test]
fn task058_invalid_bounded_ascii_lexicals_are_rejected_by_the_model() {
    for (member, bad, path, carrier) in [
        ("Marker", json!(" "), "BoundedPayload.Marker", "MarkerType"),
        ("Tail", json!("ab12 xy "), "BoundedPayload.Tail", "TailType"),
        // Nine characters: trimming would make it valid, preserve must not.
        (
            "Tail",
            json!(" AB12XY  "),
            "BoundedPayload.Tail",
            "TailType",
        ),
        ("Tail", json!("AB12XY"), "BoundedPayload.Tail", "TailType"),
        (
            "Tail",
            json!("AB12\tXY "),
            "BoundedPayload.Tail",
            "TailType",
        ),
        ("Label", json!(""), "BoundedPayload.Label", "LabelType"),
        (
            "Label",
            json!("caf\u{e9}"),
            "BoundedPayload.Label",
            "LabelType",
        ),
        (
            "Markers",
            json!(["", "x"]),
            "BoundedPayload.Markers[1]",
            "MarkerType",
        ),
        (
            "Codes",
            json!(["aB3"]),
            "BoundedPayload.Codes[0]",
            "CodeType",
        ),
        (
            "Pick",
            json!({ "Launch": "abc" }),
            "BoundedPayload.Pick.Launch",
            "LaunchType",
        ),
        (
            "Pick",
            json!({ "Empty": "A" }),
            "BoundedPayload.Pick.Empty",
            "MarkerType",
        ),
    ] {
        let mut document = encode(&payload());
        document[member] = bad.clone();
        let error = decode(&document).expect_err(member);
        assert!(
            error.message().starts_with(path) && error.message().contains(carrier),
            "{member} {bad}: {}",
            error.message()
        );
    }
}

/// A bounded-ASCII String is a JSON string only: numbers, booleans, arrays,
/// objects and null are type mismatches, never coerced (a number `1234`
/// is never a `CodeType`).
#[test]
fn task058_non_string_json_is_rejected() {
    for bad in [
        json!(1234),
        json!(true),
        json!(null),
        json!(["AB12 XY "]),
        json!({}),
    ] {
        for member in ["Marker", "Tail", "Label"] {
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
    let mut document = encode(&payload());
    document["Codes"] = json!([1234]);
    assert!(decode(&document).is_err());
}
