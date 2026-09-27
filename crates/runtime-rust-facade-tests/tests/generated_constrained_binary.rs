//! Task 053: GENERATED constrained named Binary carriers (Rust model) and
//! their GENERATED `xs:hexBinary` codec.
//!
//! Everything here is `service-generate --with-codec` output (build.rs) over
//! `tests/fixtures/service-generate/constrained-binary.xsd`. The generated
//! MODEL `T::new` is the authority on octet count: the codec never restates
//! a bound, and a lexically valid hex value of an illegal octet count is a
//! model rejection at the member path, not a hex lexical error.
//!
//! Rust needs no move workaround (contrast the C++ carrier): moving a Rust
//! value makes the source statically unusable, and `Clone` copies an already
//! valid value. There is no `Default` and no unchecked constructor.

use ams_gra_oms_runtime_rust::{CodecError, OmsJsonCodec};
use ams_gra_oms_runtime_rust_facade_tests::constrained_binary::model as m;
use ams_gra_oms_runtime_rust_facade_tests::constrained_binary::service_codec::ServiceCodec;
use serde_json::{Value, json};

fn octets(count: usize) -> Vec<u8> {
    (0..count).map(|index| index as u8 ^ 0xA5).collect()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|octet| format!("{octet:02X}")).collect()
}

/// Every constrained carrier accepts exactly its octet interval, at both
/// boundaries, and rejects one octet either side.
#[test]
fn task053_rust_carriers_enforce_octet_bounds() {
    fn check<T>(label: &str, new: fn(Vec<u8>) -> Option<T>, min: usize, max: Option<usize>) {
        if min > 0 {
            assert!(new(octets(min - 1)).is_none(), "{label}: below minimum");
        }
        assert!(new(octets(min)).is_some(), "{label}: lower boundary");
        match max {
            Some(max) => {
                assert!(new(octets(max)).is_some(), "{label}: upper boundary");
                assert!(new(octets(max + 1)).is_none(), "{label}: above maximum");
            }
            None => assert!(new(octets(4096)).is_some(), "{label}: unbounded above"),
        }
    }
    check("Exact4", m::Exact4::new, 4, Some(4));
    check("Min2", m::Min2::new, 2, None);
    check("Max6", m::Max6::new, 0, Some(6));
    check("Between2And6", m::Between2And6::new, 2, Some(6));
    check("DerivedExact4", m::DerivedExact4::new, 4, Some(4));
    check("ZeroBlob", m::ZeroBlob::new, 0, Some(0));
    // Three-level named ancestry: BlobBase 4.., BlobMiddle 4..16, BlobExact 8.
    check("BlobBase", m::BlobBase::new, 4, None);
    check("BlobMiddle", m::BlobMiddle::new, 4, Some(16));
    check("BlobExact", m::BlobExact::new, 8, Some(8));
    assert_eq!(m::BlobExact::MIN_OCTETS, 8);
    assert_eq!(m::BlobExact::MAX_OCTETS, 8);
    assert_eq!(m::BlobMiddle::MIN_OCTETS, 4);
    assert_eq!(m::BlobMiddle::MAX_OCTETS, 16);
    assert_eq!(m::BlobBase::MIN_OCTETS, 4);
    println!("RUST CONSTRAINED BINARY BOUNDS: PASSED");
}

/// Accessors, ownership, zero-length, and the unconstrained control.
#[test]
fn task053_rust_carrier_api_and_zero_length() {
    let bytes = vec![0xDE, 0xAD, 0xBE, 0xEF];
    let exact = m::Exact4::new(bytes.clone()).expect("4 octets");
    assert_eq!(exact.as_slice(), bytes.as_slice());
    let copy = exact.clone();
    assert_eq!(copy, exact);
    // Move: `exact` is statically unusable afterwards; nothing to repair.
    let moved = exact;
    assert_eq!(moved.into_vec(), bytes);
    assert_eq!(copy.as_slice(), bytes.as_slice());

    // An empty sequence is a VALID ZeroBlob value; the ONLY way to get one is
    // still `new`, since no `Default` exists.
    let zero = m::ZeroBlob::new(Vec::new()).expect("length 0 accepts the empty value");
    assert!(zero.as_slice().is_empty());
    assert!(m::ZeroBlob::new(vec![0]).is_none());

    // Unconstrained control keeps the infallible Task 025 constructor.
    let plain: m::PlainBytes = m::PlainBytes::new(Vec::new());
    assert!(plain.as_slice().is_empty());
}

fn sample() -> m::ConstrainedBlobPayload {
    m::ConstrainedBlobPayload {
        id: m::BoundedI64::new(53).expect("xs:int"),
        exact: m::Exact4::new(vec![0x00, 0x0A, 0xEE, 0xFF]).expect("4"),
        maybemin: Some(m::Min2::new(vec![0xCA, 0xFE, 0x01]).expect("3")),
        bounded: m::BoundedVec::new(vec![
            m::Max6::new(Vec::new()).expect("0"),
            m::Max6::new(octets(6)).expect("6"),
        ])
        .expect("2 of 0..3"),
        boundedrequired: m::BoundedVec::new(vec![m::Between2And6::new(octets(2)).expect("2")])
            .expect("1 of 1..3"),
        stream: m::UnboundedVec::new(vec![m::DerivedExact4::new(octets(4)).expect("4")])
            .expect("0.."),
        streamrequired: m::UnboundedVec::new(vec![
            m::Exact4::new(octets(4)).expect("4"),
            m::Exact4::new(vec![1, 2, 3, 4]).expect("4"),
        ])
        .expect("1.."),
        zero: m::ZeroBlob::new(Vec::new()).expect("0"),
        base: Some(m::BlobBase::new(octets(9)).expect("9")),
        middle: None,
        hashed: m::BlobExact::new(octets(8)).expect("8"),
        plain: m::PlainBytes::new(vec![0x5A]),
        pick: m::BlobChoice::Ranged(m::Between2And6::new(vec![0xAB, 0xCD]).expect("2")),
    }
}

fn encode(value: &m::ConstrainedBlobPayload) -> Value {
    ServiceCodec.encode_payload(value).expect("encode")
}

fn decode(value: &Value) -> Result<m::ConstrainedBlobPayload, CodecError> {
    ServiceCodec.decode_payload(value)
}

/// Structural composition (required, optional, bounded 0..N and 1..N,
/// unbounded 0.. and 1.., Choice) round trips through the generated codec,
/// with canonical uppercase hex.
#[test]
fn task053_generated_codec_round_trips_every_occurrence_shape() {
    let value = sample();
    let encoded = encode(&value);
    assert_eq!(
        encoded,
        json!({
            "Id": 53,
            "Exact": "000AEEFF",
            "MaybeMin": "CAFE01",
            "Bounded": ["", hex(&octets(6))],
            "BoundedRequired": [hex(&octets(2))],
            "Stream": [hex(&octets(4))],
            "StreamRequired": [hex(&octets(4)), "01020304"],
            "Zero": "",
            "Base": hex(&octets(9)),
            "Hashed": hex(&octets(8)),
            "Plain": "5A",
            "Pick": { "Ranged": "ABCD" }
        })
    );
    assert_eq!(decode(&encoded).expect("decode"), value);

    // Lowercase and collapse-whitespace spellings decode to the same carrier.
    let mut lower = encoded.clone();
    lower["Exact"] = json!(" 000aeeff\t");
    lower["Pick"] = json!({ "Fixed": "deadbeef" });
    let decoded = decode(&lower).expect("lowercase decodes");
    assert_eq!(decoded.exact.as_slice(), [0x00, 0x0A, 0xEE, 0xFF]);
    assert_eq!(
        decoded.pick,
        m::BlobChoice::Fixed(m::Exact4::new(vec![0xDE, 0xAD, 0xBE, 0xEF]).expect("4"))
    );
    println!("GENERATED CONSTRAINED BINARY CODEC: PASSED");
}

/// Lexically VALID hex of an illegal octet count is rejected by the
/// generated model constructor, at the semantic member path, and is NOT
/// reported as a hexBinary lexical error. Boundaries are accepted.
#[test]
fn task053_codec_rejects_illegal_octet_counts_through_the_model() {
    let base = encode(&sample());
    let with = |member: &str, value: Value| {
        let mut document = base.clone();
        document[member] = value;
        decode(&document)
    };
    for (member, carrier, bad) in [
        ("Exact", "Exact4", hex(&octets(3))),
        ("Exact", "Exact4", hex(&octets(5))),
        ("MaybeMin", "Min2", hex(&octets(1))),
        ("Zero", "ZeroBlob", "00".to_owned()),
        ("Hashed", "BlobExact", hex(&octets(7))),
        ("Hashed", "BlobExact", hex(&octets(9))),
        ("Base", "BlobBase", hex(&octets(3))),
    ] {
        let error = with(member, json!(bad)).expect_err("illegal octet count");
        assert_eq!(
            error.message(),
            format!("ConstrainedBlobPayload.{member}: value rejected by generated {carrier}::new"),
            "{member} = {bad:?}"
        );
        assert!(!error.message().contains("is not hexBinary"));
    }
    // Repeated members identify the element index.
    let error = with("Bounded", json!(["", hex(&octets(7))])).expect_err("7 > 6");
    assert_eq!(
        error.message(),
        "ConstrainedBlobPayload.Bounded[1]: value rejected by generated Max6::new"
    );
    let error = with("Stream", json!([hex(&octets(5))])).expect_err("5 != 4");
    assert_eq!(
        error.message(),
        "ConstrainedBlobPayload.Stream[0]: value rejected by generated DerivedExact4::new"
    );
    let error = with("Pick", json!({ "Ranged": "AB" })).expect_err("1 < 2");
    assert!(
        error
            .message()
            .ends_with("value rejected by generated Between2And6::new"),
        "{}",
        error.message()
    );
    // Exact boundaries are accepted.
    for (member, good) in [
        ("MaybeMin", hex(&octets(2))),
        ("Base", hex(&octets(4))),
        ("Middle", hex(&octets(4))),
        ("Middle", hex(&octets(16))),
    ] {
        with(member, json!(good)).unwrap_or_else(|error| panic!("{member}: {}", error.message()));
    }
    let error = with("Middle", json!(hex(&octets(17)))).expect_err("17 > 16");
    assert_eq!(
        error.message(),
        "ConstrainedBlobPayload.Middle: value rejected by generated BlobMiddle::new"
    );
    // A genuine lexical error is still a lexical error.
    let error = with("Exact", json!("0G0A0B0C")).expect_err("lexical");
    assert!(
        error.message().contains("is not hexBinary"),
        "{}",
        error.message()
    );
    println!("GENERATED CONSTRAINED BINARY CODEC NEGATIVES: PASSED");
}
