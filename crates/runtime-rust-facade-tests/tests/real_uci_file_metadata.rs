//! Task 059: real FileMetadata's generated checked filename and codec.
#[cfg(not(ams_gra_real_uci))]
#[test]
fn task059_real_file_metadata_structured_ascii_codec() {
    eprintln!("SKIPPED: AMS_GRA_UCI_2_5_ROOT was not set at build time");
}

#[cfg(ams_gra_real_uci)]
mod real {
    use ams_gra_oms_runtime_rust::OmsJsonCodec;
    use ams_gra_oms_runtime_rust_facade_tests::real_uci_file_metadata::model as m;
    use ams_gra_oms_runtime_rust_facade_tests::real_uci_file_metadata::service_codec::ServiceCodec;
    use serde_json::{Value, json};

    const UUID: &str = "550e8400-e29b-41d4-a716-446655440000";
    const STAMP: &str = "2026-01-01T00:00:00Z";

    fn body(filename: Value) -> Value {
        json!({
            "SecurityInformation": {
                "Classification": "U",
                "OwnerProducer": [{"GovernmentIdentifier": "USA"}]
            },
            "MessageHeader": {
                "SystemID": {"UUID": UUID},
                "Timestamp": STAMP,
                "SchemaVersion": "002.5.0",
                "Mode": "LIVE"
            },
            "MessageData": {
                "FileMetadataID": {"UUID": UUID},
                "FileDescription": {
                    "FileType": ["OTHER"],
                    "FileFormat": {"NonMIME": {"Key":"A", "SystemName":"A"}}
                },
                "FileName": filename,
                "FileSource": {},
                "CreationSource": "OTHER",
                "UntrustedModification": false,
                "Timestamp": STAMP,
                "SecurityInformation": {
                    "Classification": "U",
                    "OwnerProducer": [{"GovernmentIdentifier": "USA"}]
                }
            }
        })
    }

    #[test]
    fn task059_real_file_metadata_structured_ascii_codec() {
        assert_eq!(
            m::FileNameType::new("report.tar.bz2").unwrap().as_str(),
            "report.tar.bz2"
        );
        assert!(m::FileNameType::new("report.").is_none());
        let document = body(json!("report.tar.bz2"));
        let decoded = ServiceCodec
            .decode_payload(&document)
            .expect("real UCI filename");
        assert_eq!(decoded.messagedata.filename.as_str(), "report.tar.bz2");
        assert_eq!(ServiceCodec.encode_payload(&decoded).unwrap(), document);
        for bad in [
            json!("report."),
            json!("report\t.txt"),
            json!(null),
            json!(42),
        ] {
            let error = ServiceCodec
                .decode_payload(&body(bad))
                .expect_err("invalid filename");
            assert!(
                error
                    .message()
                    .contains("FileMetadataMT.MessageData.FileName")
            );
        }
        println!("REAL FILEMETADATA STRUCTURED ASCII CODEC: PASSED");
    }
}
