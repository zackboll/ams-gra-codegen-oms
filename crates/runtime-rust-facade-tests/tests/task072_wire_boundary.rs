//! Research only: no opaque carrier and no new supported runtime behavior.
use ams_gra_oms_runtime_rust::OmsJsonCodec;
use ams_gra_oms_runtime_rust_facade_tests::codec_shape::{model, service_codec::ServiceCodec};
use serde_json::{Value, json};

fn decode(value: &Value) -> Result<model::ShapePayload, ams_gra_oms_runtime_rust::CodecError> {
    ServiceCodec.decode_payload(value)
}

fn known() -> Value {
    json!({"{urn:shape}Shape": {"$type": "{urn:shape}BoxShape", "{urn:shape}Tag": "t", "{urn:shape}Width": 4}})
}

#[test]
fn discriminator_negative_corpus_is_exact() {
    let prefix = "ShapePayload.{urn:shape}Shape: ";
    let mut absent = known();
    absent["{urn:shape}Shape"]
        .as_object_mut()
        .unwrap()
        .remove("$type");
    assert_eq!(
        decode(&absent).unwrap_err().message(),
        format!("{prefix}abstract ShapeBase requires a $type member")
    );
    for identity in [
        "BoxShape",
        "{urn:other}BoxShape",
        "{urn:future}FutureShape",
        "{broken",
        "",
    ] {
        let mut value = known();
        value["{urn:shape}Shape"]["$type"] = json!(identity);
        assert_eq!(
            decode(&value).unwrap_err().message(),
            format!("{prefix}$type {identity:?} is not a known concrete ShapeBase")
        );
    }
    let mut conflicting = known();
    conflicting["{urn:shape}Shape"]["$type"] = json!("{urn:shape}CircleShape");
    assert_eq!(
        decode(&conflicting).unwrap_err().message(),
        format!("{prefix}unknown member \"{{urn:shape}}Width\"")
    );
    let mut non_string = known();
    non_string["{urn:shape}Shape"]["$type"] = json!(false);
    assert_eq!(
        decode(&non_string).unwrap_err().message(),
        format!("{prefix}expected $type string, found boolean")
    );
}

#[test]
fn structured_seam_has_already_lost_duplicate_and_lexical_information() {
    // Both orders reach the actual generated codec. Last-key-wins is parser
    // evidence, NOT a preservation policy or permission to accept duplicates.
    let accepted: Value = serde_json::from_str(r#"{"{urn:shape}Shape":{"$type":"{urn:future}FutureShape","$type":"{urn:shape}BoxShape","{urn:shape}Tag":"t","{urn:shape}Width":4}}"#).unwrap();
    assert_eq!(accepted, known());
    let typed = decode(&accepted).unwrap();
    assert_eq!(ServiceCodec.encode_payload(&typed).unwrap(), known());
    let rejected: Value = serde_json::from_str(r#"{"{urn:shape}Shape":{"$type":"{urn:shape}BoxShape","$type":"{urn:future}FutureShape","{urn:shape}Tag":"t","{urn:shape}Width":4}}"#).unwrap();
    assert_eq!(
        decode(&rejected).unwrap_err().message(),
        "ShapePayload.{urn:shape}Shape: $type \"{urn:future}FutureShape\" is not a known concrete ShapeBase"
    );
    let raw = " { \"x\" : 1e0, \"y\" : 2 } ";
    let parsed: Value = serde_json::from_str(raw).unwrap();
    let encoded = serde_json::to_string(&parsed).unwrap();
    assert_ne!(encoded, raw);
    assert_eq!(serde_json::from_str::<Value>(&encoded).unwrap(), parsed);
}

#[test]
fn large_and_nested_unknown_values_are_rejected_not_retained() {
    // Sizes are small deterministic stimuli, NOT proposed production limits.
    for payload in [
        json!("x".repeat(4096)),
        (0..32).fold(json!(true), |v, _| json!([v])),
    ] {
        let mut value = known();
        value["{urn:shape}Shape"]["$type"] = json!("{urn:future}FutureShape");
        value["{urn:shape}Shape"]["{urn:future}Secret"] = payload;
        assert_eq!(
            decode(&value).unwrap_err().message(),
            "ShapePayload.{urn:shape}Shape: $type \"{urn:future}FutureShape\" is not a known concrete ShapeBase"
        );
    }
    // This is a parser recursion limit, not a generated-model budget.
    let raw = format!("{}0{}", "[".repeat(256), "]".repeat(256));
    assert!(
        serde_json::from_str::<Value>(&raw)
            .unwrap_err()
            .to_string()
            .contains("recursion limit exceeded")
    );
}
