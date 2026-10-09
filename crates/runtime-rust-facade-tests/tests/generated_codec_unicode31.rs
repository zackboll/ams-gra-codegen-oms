use ams_gra_oms_runtime_rust::{CodecError, OmsJsonCodec};
use ams_gra_oms_runtime_rust_facade_tests::codec_unicode31::{
    model as m, service_codec::ServiceCodec,
};
use serde_json::{Value, json};

fn decode(v: &Value) -> Result<m::UnicodePayload, CodecError> {
    ServiceCodec.decode_payload(v)
}

#[test]
fn task066_checked_unicode_codec_preserves_text_and_rejects_neighbors() {
    for (stamp, day, location) in [
        ("200001010000", "20000101", "+12.123456+012.123456"),
        ("2١0001010000", "2١000101", "+1١.123456+012.123456"),
        ("2𝟎0001010000", "2𝟎000101", "+1𝟎.123456+012.123456"),
    ] {
        let expected = json!({"Stamp":stamp,"Day":day,"Location":location,"Optional":day,"Repeated":[day,day],"Selection":{"Date":day}});
        let value = decode(&expected).unwrap();
        assert_eq!(value.day.as_str(), day);
        assert_eq!(value.stamp.as_str(), stamp);
        assert_eq!(value.location.as_str(), location);
        assert_eq!(ServiceCodec.encode_payload(&value).unwrap(), expected);
        let mut position = expected.clone();
        position["Selection"] = json!({"Position":location});
        let v = decode(&position).unwrap();
        assert_eq!(ServiceCodec.encode_payload(&v).unwrap(), position);
        for member in [
            "Stamp",
            "Day",
            "Location",
            "Optional",
            "Repeated",
            "Selection",
        ] {
            for bad in [
                json!("invalid"),
                json!("2A000101"),
                json!("١0000101"),
                json!("2𞥐000101"),
                json!(42),
                json!(1.5),
                json!(true),
                json!(null),
                json!([]),
                json!({}),
            ] {
                let mut changed = expected.clone();
                changed[member] = match member {
                    "Repeated" => json!([bad]),
                    "Selection" => json!({"Date":bad}),
                    _ => bad,
                };
                // Optional null is the codec's absence representation, not an
                // invalid raw String entering a checked carrier.
                if member == "Optional" && changed[member].is_null() {
                    continue;
                }
                assert!(decode(&changed).is_err(), "{member}: {changed}");
            }
        }
    }
    println!("TASK066 CHECKED UNICODE JSON CODEC: PASSED");
}
