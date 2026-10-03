//! OMSC-SPC-013 Rev B §6.1.4 case 5: Time maps to JSON string; §6.1.5.4
//! string characters are lexical input. Only the checked constructor normalizes.
use ams_gra_oms_runtime_rust::{CodecError, OmsJsonCodec};
use ams_gra_oms_runtime_rust_facade_tests::codec_time_zulu::{
    model as m, service_codec::ServiceCodec,
};
use serde_json::{Value, json};

fn decode(value: &Value) -> Result<m::TimePayload, CodecError> {
    ServiceCodec.decode_payload(value)
}

#[test]
fn task061_compiled_time_codec_round_trips_and_rejects_invalid_values() {
    let document = json!({"Required":"12:34:56.5000Z", "Optional":"24:00:00.000Z", "Bounded":["00:00:00Z","23:59:60.5Z"], "Unbounded":["12:34:56Z"], "Selection":{"Time":"23:59:59Z"}, "Item":{"$type":"ConcreteClock","Time":"00:00:00Z"}});
    let decoded = decode(&document).unwrap();
    assert_eq!(ServiceCodec.encode_payload(&decoded).unwrap(), document);
    assert_eq!(decoded.required.as_str(), "12:34:56.5000Z");
    let mut padded = document.clone();
    padded["Required"] = json!("\t\n 12:34:56.5000Z\r ");
    assert_eq!(
        ServiceCodec
            .encode_payload(&decode(&padded).unwrap())
            .unwrap(),
        document
    );
    for bad in [
        json!("garbageZ"),
        json!("12:34:56"),
        json!("12:34:56+00:00"),
        json!("24:00:00.001Z"),
        json!("12:34:61Z"),
        json!(42),
        json!(true),
        json!(null),
        json!([]),
        json!({}),
    ] {
        for member in ["Required", "Optional"] {
            let mut d = document.clone();
            d[member] = bad.clone();
            assert!(decode(&d).is_err(), "{d}");
        }
        let mut d = document.clone();
        d["Bounded"] = json!(["00:00:00Z", bad.clone()]);
        assert!(decode(&d).is_err());
        let mut d = document.clone();
        d["Unbounded"] = json!([bad.clone()]);
        assert!(decode(&d).is_err());
        let mut d = document.clone();
        d["Selection"] = json!({"Time":bad.clone()});
        assert!(decode(&d).is_err());
        let mut d = document.clone();
        d["Item"]["Time"] = bad;
        assert!(decode(&d).is_err());
    }
    println!("TASK061 COMPILED TIME CODEC: PASSED");
}
