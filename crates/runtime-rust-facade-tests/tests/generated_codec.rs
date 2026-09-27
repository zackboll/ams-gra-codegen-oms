//! Task 050: the GENERATED OMS JSON codec (`service-generate --with-codec`)
//! for the synthetic OAM fixture `codec-oam.xsd`.
//!
//! Round trip: typed value -> ServiceCodec.encode_payload -> exact semantic
//! JSON -> decode_payload -> re-encode -> identical JSON. Temporal carriers
//! derive no `PartialEq`, so equality is always checked on re-encoded JSON.

use ams_gra_oms_runtime_rust::{CodecError, OmsJsonCodec};
use ams_gra_oms_runtime_rust_facade_tests::codec_oam::model as m;
use ams_gra_oms_runtime_rust_facade_tests::codec_oam::service_codec::ServiceCodec;
use serde_json::{Value, json};

mod codec_sample;
use codec_sample::{label, sample, sample_json};

fn encode(value: &m::CodecPayload) -> Value {
    ServiceCodec.encode_payload(value).expect("encode")
}

fn decode(value: &Value) -> Result<m::CodecPayload, CodecError> {
    ServiceCodec.decode_payload(value)
}

/// Order-independent JSON text whose numbers keep their exact spelling, so
/// `-0.0` and `0.0` differ (serde_json `==` treats them as equal).
fn canonical(value: &Value) -> String {
    match value {
        Value::Object(map) => {
            let mut keys: Vec<_> = map.keys().collect();
            keys.sort();
            let inner: Vec<String> = keys
                .into_iter()
                .map(|key| format!("{key:?}:{}", canonical(&map[key])))
                .collect();
            format!("{{{}}}", inner.join(","))
        }
        Value::Array(items) => format!(
            "[{}]",
            items.iter().map(canonical).collect::<Vec<_>>().join(",")
        ),
        other => other.to_string(),
    }
}

#[test]
fn task050_generated_codec_round_trips_every_construct() {
    let encoded = encode(&sample());
    assert_eq!(
        canonical(&encoded),
        canonical(&sample_json()),
        "{encoded:#}"
    );
    let decoded = decode(&encoded).expect("decode");
    let again = encode(&decoded);
    assert_eq!(canonical(&again), canonical(&encoded));
    // Typed spot checks after decode.
    assert_eq!(decoded.level.get(), -10);
    assert_eq!(decoded.signal, m::SignalCode::Value5G);
    assert!(decoded.history.as_slice()[2].get().is_nan());
    assert!(decoded.history.as_slice()[1].get().is_sign_negative());
    assert_eq!(decoded.reading, f64::NEG_INFINITY);
    assert_eq!(decoded.observed.as_str(), "2026-01-01T05:30:00+05:30");
    assert!(matches!(decoded.shape, m::ShapeBase::BoxShape(_)));
    assert_eq!(
        decoded.identity.uuid.as_str(),
        "550e8400-e29b-41d4-a716-446655440000"
    );
    // Object member order is irrelevant: decode a reversed-order object.
    let reversed: serde_json::Map<String, Value> = sample_json()
        .as_object()
        .expect("object")
        .iter()
        .rev()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    let from_reversed = decode(&Value::Object(reversed)).expect("any member order");
    assert_eq!(canonical(&encode(&from_reversed)), canonical(&encoded));
}

fn with(member: &str, value: Value) -> Value {
    let mut body = sample_json();
    body.as_object_mut()
        .expect("object")
        .insert(member.to_owned(), value);
    body
}

fn without(member: &str) -> Value {
    let mut body = sample_json();
    body.as_object_mut().expect("object").remove(member);
    body
}

fn rejects(body: &Value, path: &str) {
    let error = decode(body).expect_err("must be a CodecError");
    assert!(
        error.message().starts_with(path),
        "expected a failure at {path}, got: {}",
        error.message()
    );
}

#[test]
fn task050_enum_codec_uses_exact_wire_values() {
    // Ordinary, leading-digit (Value5G), underscore-removed (SOMEVALUE), and
    // reserved-word (ValueSelf) Rust identifiers all carry the XSD spelling.
    for (variant, wire) in [
        (m::SignalCode::NORMAL, "NORMAL"),
        (m::SignalCode::Value5G, "5G"),
        (m::SignalCode::SOMEVALUE, "SOME_VALUE"),
        (m::SignalCode::ValueSelf, "Self"),
    ] {
        let mut value = sample();
        value.signal = variant;
        let encoded = encode(&value);
        assert_eq!(encoded["Signal"], json!(wire));
        assert_eq!(decode(&encoded).expect("decode").signal, variant);
    }
    // Rust identifiers are NOT wire values.
    for bogus in ["Value5G", "SOMEVALUE", "ValueSelf", "normal", ""] {
        rejects(&with("Signal", json!(bogus)), "CodecPayload.Signal");
    }
}

#[test]
fn task050_choice_codec_selects_exactly_one_member() {
    for (source, expected) in [
        (
            m::SourceChoice::Code(m::SignalCode::NORMAL),
            json!({ "Code": "NORMAL" }),
        ),
        (
            m::SourceChoice::Text("t".to_owned()),
            json!({ "Text": "t" }),
        ),
    ] {
        let mut value = sample();
        value.source = source;
        let encoded = encode(&value);
        assert_eq!(encoded["Source"], expected);
        assert_eq!(
            canonical(&encode(&decode(&encoded).expect("decode"))),
            canonical(&encoded)
        );
    }
    rejects(&with("Source", json!({})), "CodecPayload.Source");
    rejects(
        &with("Source", json!({ "Code": "NORMAL", "Text": "t" })),
        "CodecPayload.Source",
    );
    rejects(
        &with("Source", json!({ "Other": 1 })),
        "CodecPayload.Source",
    );
    rejects(
        &with("Source", json!({ "Levels": [] })),
        "CodecPayload.Source.Levels",
    );
    rejects(
        &with("Source", json!({ "Levels": [1, 2, 3, 4] })),
        "CodecPayload.Source.Levels",
    );
    rejects(
        &with("Source", json!({ "Levels": 1 })),
        "CodecPayload.Source.Levels",
    );
    rejects(
        &with("Source", json!({ "Levels": [11] })),
        "CodecPayload.Source.Levels[0]",
    );
}

#[test]
fn task050_abstract_value_codec_uses_type_member() {
    let encoded = encode(&sample());
    assert_eq!(encoded["Shape"]["$type"], json!("BoxShape"));
    assert_eq!(encoded["Shapes"][0]["$type"], json!("CircleShape"));
    // A concrete Record expected as itself carries no $type merely because
    // it has a base: VersionedIdentity extends IdentityBase.
    assert!(encoded["Identity"].get("$type").is_none());
    assert!(encoded["Identity"].get("IdentityBase").is_none());
    rejects(
        &with(
            "Shape",
            json!({ "Tag": "box", "Width": 1.0, "Height": 1.0 }),
        ),
        "CodecPayload.Shape",
    );
    rejects(
        &with("Shape", json!({ "$type": "TriangleShape", "Tag": "t" })),
        "CodecPayload.Shape",
    );
    rejects(
        &with("Shape", json!({ "$type": "ShapeBase", "Tag": "t" })),
        "CodecPayload.Shape",
    );
    // The descendant is chosen by $type, never by its fields.
    rejects(
        &with(
            "Shape",
            json!({ "$type": "CircleShape", "Tag": "b", "Width": 1.0, "Height": 1.0 }),
        ),
        "CodecPayload.Shape",
    );
    rejects(
        &with("Shape", json!({ "$type": 7, "Tag": "t" })),
        "CodecPayload.Shape",
    );
}

#[test]
fn task050_float_codec_uses_oms_special_strings() {
    let mut value = sample();
    for (number, wire) in [
        (0.0, json!(0.0)),
        (1.25, json!(1.25)),
        (-1.25, json!(-1.25)),
        (f64::MAX, json!(f64::MAX)),
        (f64::MIN, json!(f64::MIN)),
        (f64::NAN, json!("NaN")),
        (f64::INFINITY, json!("Infinity")),
        (f64::NEG_INFINITY, json!("-Infinity")),
    ] {
        value.reading = number;
        let encoded = encode(&value);
        assert_eq!(encoded["Reading"], wire);
        let back = decode(&encoded).expect("decode").reading;
        assert!(back == number || (back.is_nan() && number.is_nan()));
    }
    value.reading = -0.0;
    let encoded = encode(&value);
    assert_eq!(encoded["Reading"].to_string(), "-0.0");
    assert!(decode(&encoded).expect("decode").reading.is_sign_negative());
    // Checked boundary values: the generated Angle constructor decides.
    for legal in [-180.0, 180.0] {
        assert!(decode(&with("Heading", json!(legal))).is_ok());
    }
    for illegal in [
        json!(180.5),
        json!("NaN"),
        json!("Infinity"),
        json!("-Infinity"),
    ] {
        rejects(&with("Heading", illegal), "CodecPayload.Heading");
    }
    // Only the three exact spellings are special values.
    for bogus in [
        "inf",
        "-inf",
        "+Infinity",
        "nan",
        "NAN",
        "infinity",
        "1.0",
        "",
    ] {
        rejects(&with("Reading", json!(bogus)), "CodecPayload.Reading");
        rejects(&with("Gain", json!(bogus)), "CodecPayload.Gain");
    }
    // Float32: special strings decode, and a finite f32 keeps its value.
    for (wire, check) in [
        (json!("NaN"), f32::is_nan as fn(f32) -> bool),
        (json!("Infinity"), |x: f32| x == f32::INFINITY),
        (json!("-Infinity"), |x: f32| x == f32::NEG_INFINITY),
        (json!(0.1), |x: f32| x == 0.1_f32),
    ] {
        let decoded = decode(&with("Gain", wire.clone())).expect("gain");
        assert!(check(decoded.gain.get()), "{wire}");
        assert_eq!(encode(&decoded)["Gain"], wire);
    }
    // A finite JSON number outside f32 range is not a legal float.
    rejects(&with("Gain", json!(1e300)), "CodecPayload.Gain");
}

#[test]
fn task050_record_repetition_and_optional_semantics() {
    let encoded = encode(&sample());
    // Optional None and empty 0..N are omitted; present optional is emitted.
    assert!(encoded.get("Comment").is_none());
    assert!(encoded.get("Tags").is_none());
    assert_eq!(encoded["Identity"]["Name"], json!("unit"));
    let mut value = sample();
    value.comment = Some("c".to_owned());
    value.tags = m::BoundedVec::new(vec![label("a"), label("b"), label("c")]).expect("max");
    let encoded = encode(&value);
    assert_eq!(encoded["Comment"], json!("c"));
    assert_eq!(encoded["Tags"], json!(["a", "b", "c"]));
    // An explicit empty array is a legal zero-occurrence spelling for 0..N.
    assert!(decode(&with("Tags", json!([]))).is_ok());
    assert!(decode(&with("History", json!([]))).is_ok());
    // Upper bound and unbounded.
    let many: Vec<Value> = (0..1000).map(|i| json!(f64::from(i))).collect();
    assert_eq!(
        decode(&with("History", Value::Array(many)))
            .expect("unbounded")
            .history
            .as_slice()
            .len(),
        1000
    );
}

/// Every rejection must be a `CodecError` naming the semantic path, and no
/// invalid value may reach the model as though it were valid.
#[test]
fn task050_strict_negative_decode_corpus() {
    let cases: Vec<(&str, Value, &str)> = vec![
        ("missing required field", without("Level"), "CodecPayload"),
        (
            "missing inherited required field",
            with("Identity", json!({ "Name": "x" })),
            "CodecPayload.Identity",
        ),
        (
            "unknown Record member",
            with("Extra", json!(1)),
            "CodecPayload",
        ),
        (
            "nested base object",
            with(
                "Identity",
                json!({ "UUID": "550e8400-e29b-41d4-a716-446655440000", "IdentityBase": {} }),
            ),
            "CodecPayload.Identity",
        ),
        (
            "null for non-nillable field",
            with("Note", Value::Null),
            "CodecPayload.Note",
        ),
        (
            "null for optional field",
            with("Comment", Value::Null),
            "CodecPayload.Comment",
        ),
        (
            "scalar where array required",
            with("Signals", json!("NORMAL")),
            "CodecPayload.Signals",
        ),
        (
            "array where scalar required",
            with("Signal", json!(["NORMAL"])),
            "CodecPayload.Signal",
        ),
        (
            "repeated below minimum",
            with("Signals", json!(["NORMAL"])),
            "CodecPayload.Signals",
        ),
        (
            "positive minimum absent",
            without("Signals"),
            "CodecPayload.Signals",
        ),
        (
            "repeated above maximum",
            with("Tags", json!(["a", "b", "c", "d"])),
            "CodecPayload.Tags",
        ),
        (
            "unbounded below minimum",
            with("Shapes", json!([])),
            "CodecPayload.Shapes",
        ),
        (
            "invalid enum wire value",
            with("Signal", json!("LOUD")),
            "CodecPayload.Signal",
        ),
        (
            "integer outside checked domain",
            with("Level", json!(11)),
            "CodecPayload.Level",
        ),
        (
            "integer outside u8 domain",
            with("Percent", json!(101)),
            "CodecPayload.Percent",
        ),
        (
            "integer outside unsignedShort",
            with("Count", json!(65536)),
            "CodecPayload.Count",
        ),
        (
            "negative unsigned",
            with("Count", json!(-1)),
            "CodecPayload.Count",
        ),
        (
            "fractional integer",
            with("Level", json!(1.5)),
            "CodecPayload.Level",
        ),
        (
            "integer as string",
            with("Level", json!("1")),
            "CodecPayload.Level",
        ),
        (
            "u64 beyond i64",
            with("Offset", json!(u64::MAX)),
            "CodecPayload.Offset",
        ),
        (
            "invalid constrained String",
            with("Callsign", json!("")),
            "CodecPayload.Callsign",
        ),
        (
            "constrained String too long",
            with("Callsign", json!("x".repeat(33))),
            "CodecPayload.Callsign",
        ),
        (
            "invalid UUID",
            with("Identity", json!({ "UUID": "not-a-uuid" })),
            "CodecPayload.Identity.UUID",
        ),
        (
            "invalid direct DateTime",
            with("Observed", json!("2026-13-01T00:00:00Z")),
            "CodecPayload.Observed",
        ),
        (
            "invalid named Zulu DateTime",
            with("Created", json!("2026-01-01T00:00:00+01:00")),
            "CodecPayload.Created",
        ),
        (
            "garbage named Zulu DateTime",
            with("Created", json!("garbageZ")),
            "CodecPayload.Created",
        ),
        (
            "invalid float special string",
            with("Reading", json!("inf")),
            "CodecPayload.Reading",
        ),
        (
            "boolean as string",
            with("Enabled", json!("true")),
            "CodecPayload.Enabled",
        ),
        (
            "Choice with zero members",
            with("Source", json!({})),
            "CodecPayload.Source",
        ),
        (
            "Choice with two members",
            with("Source", json!({ "Code": "NORMAL", "Text": "t" })),
            "CodecPayload.Source",
        ),
        (
            "unknown Choice member",
            with("Source", json!({ "Nope": "t" })),
            "CodecPayload.Source",
        ),
        (
            "abstract value missing $type",
            with("Shape", json!({ "Tag": "t", "Radius": 1.0 })),
            "CodecPayload.Shape",
        ),
        (
            "abstract value unknown $type",
            with("Shape", json!({ "$type": "Nope", "Tag": "t" })),
            "CodecPayload.Shape",
        ),
        (
            "abstract value in array missing $type",
            with("Shapes", json!([{ "Tag": "t", "Radius": 1.0 }])),
            "CodecPayload.Shapes[0]",
        ),
        (
            "$type on a concrete non-polymorphic Record",
            with(
                "Identity",
                json!({ "$type": "IdentityBase", "UUID": "550e8400-e29b-41d4-a716-446655440000" }),
            ),
            "CodecPayload.Identity",
        ),
        ("payload is not an object", json!([]), "CodecPayload"),
    ];
    for (name, body, path) in &cases {
        let error = decode(body)
            .err()
            .unwrap_or_else(|| panic!("{name}: accepted"));
        assert!(
            error.message().starts_with(path),
            "{name}: expected path {path}, got {}",
            error.message()
        );
    }
    println!("NEGATIVE CORPUS: {} cases rejected", cases.len());
}
