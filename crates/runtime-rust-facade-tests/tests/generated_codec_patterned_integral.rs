use ams_gra_oms_runtime_rust::{CodecError, OmsJsonCodec};
use ams_gra_oms_runtime_rust_facade_tests::codec_patterned_integral::{
    model as m, service_codec::ServiceCodec,
};
use serde_json::{Value, json};

fn decode(value: &Value) -> Result<m::SerialPayload, CodecError> {
    ServiceCodec.decode_payload(value)
}

#[test]
fn task063_generated_integer_codec_and_raw_json_semantics() {
    fn traits<T: Copy + Eq + Ord + std::hash::Hash>() {}
    traits::<m::Serial>();
    traits::<m::Renamed>();
    for n in [1, 9, 10, 99, 100, 999] {
        let document = json!({"Required":n,"Optional":n,"Bounded":[1,n,999],"Unbounded":[n],"Selection":{"Number":n}});
        let decoded = decode(&document).unwrap();
        assert_eq!(decoded.required.get(), n);
        assert_eq!(ServiceCodec.encode_payload(&decoded).unwrap(), document);
    }
    let valid = json!({"Required":1,"Optional":999,"Bounded":[1],"Unbounded":[999],"Selection":{"Number":1}});
    for text in [
        "0",
        "1000",
        "-1",
        "1.0",
        "1e0",
        "-0",
        "-9223372036854775808",
        "9223372036854775807",
    ] {
        let bad: Value = serde_json::from_str(text).unwrap();
        println!("JSON {text}: as_i64={:?}", bad.as_i64());
        for member in ["Required", "Optional"] {
            let mut d = valid.clone();
            d[member] = bad.clone();
            assert!(decode(&d).is_err(), "{d}");
        }
        for member in ["Bounded", "Unbounded"] {
            let mut d = valid.clone();
            d[member] = json!([bad.clone()]);
            assert!(decode(&d).is_err(), "{d}");
        }
        let mut d = valid.clone();
        d["Selection"] = json!({"Number":bad});
        assert!(decode(&d).is_err());
    }
    for text in ["1.0", "1e0", "-0"] {
        assert_eq!(serde_json::from_str::<Value>(text).unwrap().as_i64(), None);
    }
    for text in ["+1", "001"] {
        assert!(serde_json::from_str::<Value>(text).is_err());
    }
    for n in [i64::MIN, -1, 0, 1000, i64::MAX] {
        assert!(m::Serial::new(n).is_none());
    }
    println!("TASK063 GENERATED INTEGER CODEC AND JSON: PASSED");
}
