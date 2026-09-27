//! Task 052: GENERATED xs:hexBinary OMS JSON codecs.
//!
//! Every codec here comes from the real `service-generate --with-codec`
//! (build.rs). The generated decoder is the authority for hexBinary lexical
//! validation (XML Schema 2E 3.2.15 + whiteSpace=collapse); nothing here
//! relies on Sleet for negative lexical cases.

use ams_gra_oms_runtime_rust::{CodecError, OmsJsonCodec};
use ams_gra_oms_runtime_rust_facade_tests::codec_binary::model as bm;
use ams_gra_oms_runtime_rust_facade_tests::codec_binary::service_codec::ServiceCodec as BinaryCodec;
use ams_gra_oms_runtime_rust_facade_tests::codec_hexbinary::model as m;
use ams_gra_oms_runtime_rust_facade_tests::codec_hexbinary::service_codec::ServiceCodec;
use serde_json::{Value, json};

fn int(value: i64) -> m::BoundedI64<-2147483648, 2147483647> {
    m::BoundedI64::new(value).expect("xs:int")
}

/// One Binary value per shape; `data` is the single value under test.
fn payload(data: Vec<u8>) -> m::BlobPayload {
    m::BlobPayload {
        id: int(1),
        data,
        maybedata: None,
        chunks: m::BoundedVec::new(Vec::new()).expect("0..3"),
        stream: m::UnboundedVec::new(Vec::new()).expect("0.."),
        blob: m::BlobBytes::new(vec![0xB1]),
        alias: None,
        atomic: m::AtomicLike::IntValue(int(7)),
    }
}

fn encode(value: &m::BlobPayload) -> Value {
    ServiceCodec.encode_payload(value).expect("encode")
}

fn decode(value: &Value) -> Result<m::BlobPayload, CodecError> {
    ServiceCodec.decode_payload(value)
}

/// A decodable document whose `Data` member is `data`.
fn document(data: Value) -> Value {
    json!({
        "Id": 1,
        "Data": data,
        "Blob": "B1",
        "Atomic": { "IntValue": 7 }
    })
}

fn canonical_hex(octets: &[u8]) -> String {
    octets.iter().map(|octet| format!("{octet:02X}")).collect()
}

/// Canonical encoder corpus: uppercase, two characters per octet, no
/// whitespace, separator, or prefix. Every octet value is covered.
#[test]
fn task052_encoder_emits_canonical_uppercase_hex() {
    for (octets, expected) in [
        (vec![], ""),
        (vec![0x00], "00"),
        (vec![0x0a], "0A"),
        (vec![0xee], "EE"),
        (vec![0x00, 0x0f, 0xab], "000FAB"),
        (vec![0xff], "FF"),
        (vec![0x00, 0x0A, 0xEE, 0xFF], "000AEEFF"),
    ] {
        let encoded = encode(&payload(octets.clone()));
        assert_eq!(encoded["Data"], json!(expected), "{octets:?}");
        assert_eq!(decode(&encoded).expect("decode").data, octets);
    }
    let all: Vec<u8> = (0..=255).collect();
    let encoded = encode(&payload(all.clone()));
    let text = encoded["Data"].as_str().expect("one JSON string");
    assert_eq!(text, canonical_hex(&all));
    assert_eq!(text.len(), 512);
    assert!(text.bytes().all(|b| matches!(b, b'0'..=b'9' | b'A'..=b'F')));
    assert_eq!(decode(&encoded).expect("decode").data, all);
}

/// Valid lexical corpus, including mixed case and the fixed whitespace
/// collapse: leading/trailing TAB/LF/CR/SPACE disappear.
#[test]
fn task052_decoder_accepts_xsd_hex_binary_lexical_forms() {
    for (text, octets) in [
        ("", vec![]),
        ("00", vec![0x00]),
        ("ee", vec![0xEE]),
        ("EE", vec![0xEE]),
        ("0aFf", vec![0x0A, 0xFF]),
        ("000aeeff", vec![0x00, 0x0A, 0xEE, 0xFF]),
        ("  0A  ", vec![0x0A]),
        ("\t0A\r\n", vec![0x0A]),
        (" \t\n\r ", vec![]),
    ] {
        let decoded = decode(&document(json!(text)))
            .unwrap_or_else(|error| panic!("{text:?} must decode: {}", error.message()));
        assert_eq!(decoded.data, octets, "{text:?}");
        // Re-encoding is canonical, whatever lexical form arrived.
        assert_eq!(encode(&decoded)["Data"], json!(canonical_hex(&octets)));
    }
}

/// Invalid lexical corpus. An internal space survives collapse and is
/// rejected; odd length, non-hex, prefixes, separators, and non-ASCII
/// look-alikes (fullwidth and Arabic-Indic digits, NBSP) are rejected.
#[test]
fn task052_decoder_rejects_invalid_hex_binary() {
    let alphabet = "only hexadecimal digits [0-9A-Fa-f] are allowed";
    let odd = "odd number of hexadecimal digits";
    for (text, reason) in [
        ("0", odd),
        ("ABC", odd),
        ("  ABC ", odd),
        ("GG", alphabet),
        ("0xAA", alphabet),
        ("AA-BB", alphabet),
        ("AA BB", alphabet),
        ("AA\tBB", alphabet),
        ("AA\u{a0}", alphabet),
        ("\u{ff10}\u{ff10}", alphabet),
        ("\u{0660}\u{0661}", alphabet),
        ("+0A", alphabet),
    ] {
        let error = decode(&document(json!(text))).expect_err(text);
        assert_eq!(
            error.message(),
            format!("BlobPayload.Data: {text:?} is not hexBinary: {reason}"),
            "{text:?}"
        );
    }
}

/// JSON type strictness: one Binary value is exactly one JSON string. An
/// octet array is NOT accepted as a convenience.
#[test]
fn task052_decoder_accepts_only_json_strings() {
    for (value, kind) in [
        (json!(null), "null"),
        (json!(true), "boolean"),
        (json!(170), "number"),
        (json!([0, 10, 238, 255]), "array"),
        (json!({ "hex": "AA" }), "object"),
    ] {
        let error = decode(&document(value)).expect_err(kind);
        assert_eq!(
            error.message(),
            format!("BlobPayload.Data: expected hexBinary string, found {kind}")
        );
    }
}

/// The full value used by the shape tests: every Binary shape populated.
fn full() -> (m::BlobPayload, Value) {
    let value = m::BlobPayload {
        id: int(2),
        data: vec![0x00, 0x0A, 0xEE, 0xFF],
        maybedata: Some(vec![0xCA, 0xFE]),
        chunks: m::BoundedVec::new(vec![vec![0x00, 0x11], vec![0xAA, 0xBB], vec![]])
            .expect("3 values"),
        stream: m::UnboundedVec::new(vec![vec![0x01], vec![0x02, 0x03, 0x04, 0x05, 0x06]])
            .expect("2 values"),
        blob: m::BlobBytes::new(vec![0xDE, 0xAD, 0xBE, 0xEF]),
        alias: Some(m::BlobAlias::new(vec![0x5A])),
        atomic: m::AtomicLike::HexBinaryValue(vec![0x00, 0xA1, 0xFF]),
    };
    let json = json!({
        "Id": 2,
        "Data": "000AEEFF",
        "MaybeData": "CAFE",
        "Chunks": ["0011", "AABB", ""],
        "Stream": ["01", "0203040506"],
        "Blob": "DEADBEEF",
        "Alias": "5A",
        "Atomic": { "HexBinaryValue": "00A1FF" }
    });
    (value, json)
}

/// Optional, bounded-repeated, and unbounded-repeated direct Binary; named
/// and named-on-named Binary through the generated wrapper constructors;
/// the Choice alternative. The OUTER cardinality counts Binary values; each
/// inner string is the octets of ONE value.
#[test]
fn task052_optional_repeated_named_and_choice_binary_round_trip() {
    let (value, expected) = full();
    let encoded = encode(&value);
    assert_eq!(encoded, expected);
    assert_eq!(decode(&encoded).expect("decode"), value);

    // Optional absent -> member omitted; zero repetitions -> no member.
    let sparse = encode(&payload(vec![]));
    let object = sparse.as_object().expect("object");
    for absent in ["MaybeData", "Chunks", "Stream", "Alias"] {
        assert!(!object.contains_key(absent), "{absent}");
    }
    assert_eq!(decode(&sparse).expect("decode"), payload(vec![]));

    // The Choice decodes to HexBinaryValue from any lexical case.
    let mut lowercase = expected.clone();
    lowercase["Atomic"] = json!({ "HexBinaryValue": "00a1ff" });
    assert_eq!(
        decode(&lowercase).expect("decode").atomic,
        m::AtomicLike::HexBinaryValue(vec![0x00, 0xA1, 0xFF])
    );
    // The other alternative still decodes as itself.
    let mut int_choice = expected;
    int_choice["Atomic"] = json!({ "IntValue": -3 });
    assert_eq!(
        decode(&int_choice).expect("decode").atomic,
        m::AtomicLike::IntValue(int(-3))
    );
}

/// Cardinality and path diagnostics for the Binary shapes.
#[test]
fn task052_binary_shape_diagnostics() {
    let (_, expected) = full();
    // 4 Binary VALUES exceed Chunks 0..3 (octets per value are irrelevant).
    let mut too_many = expected.clone();
    too_many["Chunks"] = json!(["00", "01", "02", "03"]);
    assert_eq!(
        decode(&too_many).expect_err("4 > 3").message(),
        "BlobPayload.Chunks: 4 occurrence(s) rejected by the generated sequence constructor"
    );
    // A repeated Binary is an array of strings; a single string is not.
    let mut not_array = expected.clone();
    not_array["Stream"] = json!("0102");
    assert_eq!(
        decode(&not_array).expect_err("array").message(),
        "BlobPayload.Stream: expected array, found string"
    );
    for (member, bad, message) in [
        (
            "Chunks",
            json!(["00", "0"]),
            "BlobPayload.Chunks[1]: \"0\" is not hexBinary: odd number of hexadecimal digits",
        ),
        (
            "Blob",
            json!("XY"),
            "BlobPayload.Blob: \"XY\" is not hexBinary: only hexadecimal digits [0-9A-Fa-f] are allowed",
        ),
        (
            "Alias",
            json!(7),
            "BlobPayload.Alias: expected hexBinary string, found number",
        ),
        (
            "Atomic",
            json!({ "HexBinaryValue": [0] }),
            "BlobPayload.Atomic.HexBinaryValue: expected hexBinary string, found array",
        ),
    ] {
        let mut bad_document = expected.clone();
        bad_document[member] = bad;
        assert_eq!(
            decode(&bad_document).expect_err(member).message(),
            message,
            "{member}"
        );
    }
}

/// The flipped Task 050 `codec-binary` control: `Data` is the only Binary.
#[test]
fn task052_codec_binary_control_round_trips() {
    let value = bm::BlobPayload {
        id: bm::BoundedI64::new(5).expect("xs:int"),
        data: vec![0x00, 0x0A, 0xEE, 0xFF],
    };
    let encoded = BinaryCodec.encode_payload(&value).expect("encode");
    assert_eq!(encoded, json!({ "Id": 5, "Data": "000AEEFF" }));
    let decoded: bm::BlobPayload = BinaryCodec
        .decode_payload(&json!({ "Id": 5, "Data": "000aeeff" }))
        .expect("decode");
    assert_eq!(decoded, value);
}
