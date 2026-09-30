//! Task 058: the REAL UCI 2.5 `AMTI_SettingsCommand` message through the
//! GENERATED model and OMS JSON codec.
//!
//! `AMTI_SettingsCommand` is the smallest real UCI message made READY by the
//! bounded-ASCII String profiles: its only earlier blocker was `EmptyType`
//! (`[a-zA-Z]{0}`, `length 0`), reached through the optional
//! `AMTI_SettingsCommandMDT.UnassignAll`. The document below is test-authored;
//! its SecurityInformation/MessageHeader members are exactly those used by the
//! Task 052 real SubsystemStream test.
//!
//! Requires `AMS_GRA_UCI_2_5_ROOT` at build time; otherwise a stated skip.

#[cfg(not(ams_gra_real_uci))]
#[test]
fn task058_real_amti_settings_empty_type_round_trips() {
    eprintln!("SKIPPED: AMS_GRA_UCI_2_5_ROOT was not set when this crate was built");
}

#[cfg(ams_gra_real_uci)]
mod real {
    use ams_gra_oms_runtime_rust::OmsJsonCodec;
    use ams_gra_oms_runtime_rust_facade_tests::real_uci_amti_settings::model as m;
    use ams_gra_oms_runtime_rust_facade_tests::real_uci_amti_settings::service_codec::ServiceCodec;
    use serde_json::{Value, json};

    const UUID: &str = "550e8400-e29b-41d4-a716-446655440000";
    const STAMP: &str = "2026-01-01T00:00:00Z";

    /// The payload body with `unassign` as the optional `UnassignAll`.
    fn body(unassign: Option<Value>) -> Value {
        let mut data = json!({
            "CommandID": { "UUID": UUID },
            "CommandState": "NEW",
            "SubsystemID": { "UUID": UUID }
        });
        if let Some(value) = unassign {
            data["UnassignAll"] = value;
        }
        json!({
            "SecurityInformation": {
                "Classification": "U",
                "OwnerProducer": [{ "GovernmentIdentifier": "USA" }]
            },
            "MessageHeader": {
                "SystemID": { "UUID": UUID },
                "Timestamp": STAMP,
                "SchemaVersion": "002.5.0",
                "Mode": "LIVE"
            },
            "MessageData": data
        })
    }

    /// Present zero-length `EmptyType`: decoded through the generated checked
    /// constructor, re-encoded as the empty JSON string; absent stays absent;
    /// any non-empty or non-string value is rejected at its real path.
    #[test]
    fn task058_real_amti_settings_empty_type_round_trips() {
        let present = body(Some(json!("")));
        let decoded = ServiceCodec
            .decode_payload(&present)
            .expect("the generated codec decodes the real AMTI_SettingsCommand");
        let unassign = decoded
            .messagedata
            .unassignall
            .as_ref()
            .expect("UnassignAll present");
        assert_eq!(unassign.as_str(), "");
        assert_eq!(*unassign, m::EmptyType::new("").expect("zero-length"));
        assert_eq!(
            ServiceCodec.encode_payload(&decoded).expect("encode"),
            present
        );

        let absent = body(None);
        let decoded = ServiceCodec.decode_payload(&absent).expect("decode");
        assert!(decoded.messagedata.unassignall.is_none());
        assert_eq!(
            ServiceCodec.encode_payload(&decoded).expect("encode"),
            absent
        );

        for bad in [json!(" "), json!("x"), json!(0), json!(null), json!([])] {
            let error = ServiceCodec
                .decode_payload(&body(Some(bad.clone())))
                .expect_err("invalid EmptyType lexical");
            assert!(
                error
                    .message()
                    .starts_with("AMTI_SettingsCommandMT.MessageData.UnassignAll"),
                "{bad}: {}",
                error.message()
            );
        }
        assert!(m::EmptyType::new("A").is_none());
        println!("REAL AMTI_SETTINGSCOMMAND EMPTYTYPE CODEC: PASSED");
    }
}
