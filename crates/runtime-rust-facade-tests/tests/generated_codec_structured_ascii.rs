//! Task 059: the generated OMS JSON codec uses checked structured carriers.
use ams_gra_oms_runtime_rust::{CodecError, OmsJsonCodec};
use ams_gra_oms_runtime_rust_facade_tests::codec_structured_ascii::model as m;
use ams_gra_oms_runtime_rust_facade_tests::codec_structured_ascii::service_codec::ServiceCodec;
use serde_json::{Value, json};

fn payload() -> m::StructuredPayload {
    m::StructuredPayload {
        imo: m::ImoType::new("IMO0000001").unwrap(),
        prf: m::PrfType::new("1178").unwrap(),
        filename: Some(m::FileNameType::new("a.tar.bz2").unwrap()),
        imos: m::BoundedVec::new(vec![m::ImoType::new("IMO9999999").unwrap()]).unwrap(),
        octals: m::UnboundedVec::new(vec![m::OctalType::new("0777").unwrap()]).unwrap(),
        pick: m::StructuredPick::Model(m::ModelType::new("aB3").unwrap()),
    }
}

fn decode(value: &Value) -> Result<m::StructuredPayload, CodecError> {
    ServiceCodec.decode_payload(value)
}

#[test]
fn task059_structured_ascii_codec_round_trip_and_rejections() {
    let encoded = ServiceCodec.encode_payload(&payload()).unwrap();
    assert_eq!(
        encoded,
        json!({
            "Imo": "IMO0000001", "Prf": "1178", "FileName": "a.tar.bz2",
            "Imos": ["IMO9999999"], "Octals": ["0777"],
            "Pick": {"Model": "aB3"}
        })
    );
    let decoded = decode(&encoded).unwrap();
    assert_eq!(ServiceCodec.encode_payload(&decoded).unwrap(), encoded);
    for (member, bad) in [
        ("Imo", json!("imo0000001")),
        ("Prf", json!("11178")),
        ("FileName", json!("a.")),
        ("FileName", json!("a\t.b")),
        ("Imo", json!(42)),
        ("Prf", json!(null)),
        ("Octals", json!(["8"])),
    ] {
        let mut document = encoded.clone();
        document[member] = bad;
        assert!(decode(&document).is_err(), "{member}: {document}");
    }
}
