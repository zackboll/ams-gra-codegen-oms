use ams_gra_oms_runtime_rust::{CodecError, OmsJsonCodec};
use ams_gra_oms_runtime_rust_facade_tests::codec_alternating_ascii::{
    model as m, service_codec::ServiceCodec,
};
use serde_json::{Value, json};

fn decode(value: &Value) -> Result<m::AlternatingPayload, CodecError> {
    ServiceCodec.decode_payload(value)
}

#[test]
fn task060_compiled_checked_string_codec_preserves_union_lexicals() {
    for (notation, origin, address) in [
        ("UNKN", "-", "0.0.0.0"),
        ("NONE0", "E", "255.255.255.255"),
        ("ABC12", "ZZ", "9.99.199.249"),
    ] {
        let value = m::AlternatingPayload {
            notation: m::NotationType::new(notation).unwrap(),
            origin: m::OriginType::new(origin).unwrap(),
            address: m::AddressType::new(address).unwrap(),
        };
        let expected = json!({"Notation":notation,"Origin":origin,"Address":address});
        assert_eq!(ServiceCodec.encode_payload(&value).unwrap(), expected);
        assert_eq!(
            ServiceCodec
                .encode_payload(&decode(&expected).unwrap())
                .unwrap(),
            expected
        );
        for (member, bad) in [
            ("Notation", json!("UNKX")),
            ("Notation", json!("none")),
            ("Origin", json!("A")),
            ("Origin", json!("E-")),
            ("Address", json!("01.2.3.4")),
            ("Address", json!("256.0.0.1")),
            ("Address", json!("١.2.3.4")),
        ] {
            let mut document = expected.clone();
            document[member] = bad;
            assert!(decode(&document).is_err(), "{document}");
        }
        for member in ["Notation", "Origin", "Address"] {
            for bad in [json!(42), json!(true), json!(null), json!([]), json!({})] {
                let mut document = expected.clone();
                document[member] = bad;
                assert!(decode(&document).is_err(), "{document}");
            }
        }
    }
}
