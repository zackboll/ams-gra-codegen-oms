//! Research only: production seam observations and a proposed decision table.
//! The table is NOT a validator, registry, quarantine carrier, or supported API.
use ams_gra_oms_runtime_rust::{OmsJsonCodec, unwrap_global_element};
use ams_gra_oms_runtime_rust_facade_tests::codec_shape::{model, service_codec::ServiceCodec};
use serde_json::{Value, json};

#[test]
fn generated_checked_model_does_not_admit_asserted_authority() {
    let known = json!({"{urn:shape}Shape": {
        "$type": "{urn:shape}BoxShape", "{urn:shape}Tag": "t", "{urn:shape}Width": 4
    }});
    let typed: model::ShapePayload = ServiceCodec.decode_payload(&known).unwrap();
    assert_eq!(ServiceCodec.encode_payload(&typed).unwrap(), known);
    for name in ["{urn:private}Future", "{urn:other}BoxShape"] {
        let mut unknown = known.clone();
        unknown["{urn:shape}Shape"]["$type"] = json!(name);
        assert!(
            <ServiceCodec as OmsJsonCodec<model::ShapePayload>>::decode_payload(
                &ServiceCodec,
                &unknown
            )
            .is_err()
        );
    }
    let mut extra = known;
    extra["{urn:shape}Shape"]["Secret"] = json!({"nested": [{"value": 1}]});
    assert!(
        <ServiceCodec as OmsJsonCodec<model::ShapePayload>>::decode_payload(&ServiceCodec, &extra)
            .is_err()
    );
}

#[test]
fn value_seam_loses_duplicate_discriminators_and_lexical_information() {
    let raw = r#"{"Msg":{"$type":"{urn:trusted}T","$type":"{urn:untrusted}T","z":1e0,"a":"\u0061","nested":{"unknown":true}}}"#;
    let payload = unwrap_global_element("Msg", raw).unwrap();
    assert_eq!(payload["$type"], "{urn:untrusted}T");
    assert_eq!(payload["a"], "a");
    assert_eq!(payload["nested"]["unknown"], true);
    let encoded = serde_json::to_string(&payload).unwrap();
    assert!(!encoded.contains("1e0"));
    assert!(!encoded.contains("\\u0061"));
    assert!(encoded.find("\"a\"").unwrap() < encoded.find("\"z\"").unwrap());
    // Even duplicate global elements collapse before the one-member check.
    assert_eq!(
        unwrap_global_element("Msg", r#"{"Msg":1,"Msg":2}"#).unwrap(),
        2
    );
    let rounded: Value = serde_json::from_str("9007199254740993.0").unwrap();
    // Pin the actual locked parser result, not an assumed rounding algorithm.
    assert_eq!(rounded.as_f64().unwrap(), 9_007_199_254_740_994.0);
}

#[test]
fn malformed_unicode_and_recursive_resource_boundaries_are_not_authority() {
    let invalid_utf8 = vec![u8::MAX];
    assert!(std::str::from_utf8(&invalid_utf8).is_err());
    for raw in [r#"{"Msg":"\uD800"}"#, r#"{"Msg":"\uDC00"}"#, "{", "NaN"] {
        assert!(unwrap_global_element("Msg", raw).is_err());
    }
    assert_eq!(
        unwrap_global_element("Msg", r#"{"Msg":"\uD83D\uDE00"}"#).unwrap(),
        "😀"
    );
    let shallow = format!("{{\"Msg\":{}0{}}}", "[".repeat(8), "]".repeat(8));
    assert!(unwrap_global_element("Msg", &shallow).is_ok());
    let deep = format!("{{\"Msg\":{}0{}}}", "[".repeat(256), "]".repeat(256));
    assert!(unwrap_global_element("Msg", &deep).is_err());
    // Demonstrates absence of an envelope-level deployment size budget,
    // not a proposed limit and not protection against aggregate exhaustion.
    let large = format!("{{\"Msg\":\"{}\"}}", "x".repeat(65_537));
    assert!(unwrap_global_element("Msg", &large).is_ok());
}

#[derive(Clone, Copy)]
struct ProposedEvidence<'a> {
    service: &'a str,
    schema: &'a str,
    context: &'a str,
    epoch: u64,
    expires: u64,
    validated: bool,
}

// Pure truth-table predicate: assumes validation as an input, never performs it.
fn proposed_publish_gate(e: ProposedEvidence<'_>, now: u64, epoch: u64, grant: bool) -> bool {
    e.validated
        && e.service == "list-owner"
        && e.schema == "reviewed-private-bundle"
        && e.context == "deployment-A"
        && e.epoch == epoch
        && now < e.expires
        && grant
}

#[test]
fn proposed_private_trust_and_operation_gate_is_fail_closed() {
    let trusted = ProposedEvidence {
        service: "list-owner",
        schema: "reviewed-private-bundle",
        context: "deployment-A",
        epoch: 7,
        expires: 100,
        validated: true,
    };
    assert!(proposed_publish_gate(trusted, 99, 7, true));
    assert!(!proposed_publish_gate(trusted, 99, 7, false));
    assert!(!proposed_publish_gate(trusted, 100, 7, true));
    assert!(!proposed_publish_gate(trusted, 99, 8, true));
    for untrusted in [
        ProposedEvidence {
            validated: false,
            ..trusted
        },
        ProposedEvidence {
            schema: "untrusted-private-bundle",
            ..trusted
        },
        ProposedEvidence {
            service: "other-service",
            ..trusted
        },
        ProposedEvidence {
            context: "deployment-B",
            ..trusted
        },
    ] {
        assert!(!proposed_publish_gate(untrusted, 99, 7, true));
    }
}

#[test]
fn proposed_budget_requires_aggregate_and_depth_checks() {
    // Synthetic policy thresholds, deliberately not production defaults.
    let budget = |bytes: usize, depth: usize, retained: usize| {
        bytes <= 1024 && depth <= 8 && retained.checked_add(bytes).is_some_and(|n| n <= 2048)
    };
    assert!(budget(1024, 8, 1024));
    assert!(!budget(1025, 8, 0));
    assert!(!budget(1, 9, 0));
    assert!(!budget(1024, 8, 1025));
    assert!(!budget(1, 1, usize::MAX));
}
