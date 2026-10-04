use ams_gra_oms_runtime_rust::{CodecError, OmsJsonCodec};
use ams_gra_oms_runtime_rust_facade_tests::codec_ipv6::{model as m, service_codec::ServiceCodec};
use serde_json::{Value, json};

fn decode(value: &Value) -> Result<m::Ipv6Payload, CodecError> {
    ServiceCodec.decode_payload(value)
}

#[test]
fn task062_compiled_checked_ipv6_codec_preserves_spelling_and_rejects_invalid() {
    for address in [
        "::",
        "Fe80::aBcD",
        "1::2::3",
        "::ffff:001.009.099.199",
        "1:",
    ] {
        let expected = json!({"Address":address,"Optional":address,"Repeated":[address,address]});
        let value = decode(&expected).unwrap();
        assert_eq!(ServiceCodec.encode_payload(&value).unwrap(), expected);
        assert_eq!(value.address.as_str(), address);
        assert_eq!(value.optional.unwrap().as_str(), address);
        assert!(
            value
                .repeated
                .as_slice()
                .iter()
                .all(|v| v.as_str() == address)
        );
        for member in ["Address", "Optional", "Repeated"] {
            for bad in [
                json!("::12345"),
                json!("::256.0.0.1"),
                json!("[::1]"),
                json!("fe80::1%eth0"),
                json!("::\n"),
                json!(42),
                json!(true),
                json!({}),
            ] {
                let mut changed = expected.clone();
                changed[member] = if member == "Repeated" {
                    json!([bad])
                } else {
                    bad
                };
                assert!(decode(&changed).is_err(), "{changed}");
            }
        }
    }
    println!("TASK062 COMPILED IPV6 JSON CODEC: PASSED");
}
