//! Task 052: the REAL UCI 2.5 `SubsystemStream` message, whose closure
//! contains a direct optional `xs:hexBinary`
//! (`SubsystemStreamMDT.SubsystemStreamBinary`, 0..1), end to end through
//! GENERATED code only.
//!
//! SubsystemStream is the smallest UCI 2.5 message whose closure contains a
//! hexBinary value, whose model was READY, and whose ONLY Task 050/051 codec
//! blocker was Binary provenance (see docs/task-052-hexbinary-provenance-codec.md).
//!
//! No UCI vector for this message exists in pinned Sleet, so the document
//! below is test-authored: its SecurityInformation/MessageHeader members are
//! exactly the pinned Sleet PositionReport vector's
//! (`tests/fixtures/oms-json/sleet-e38f61d8/PositionReport.json`), and its
//! MessageData holds the required SubsystemStreamMDT members plus the
//! hexBinary value under test.
//!
//! Requires `AMS_GRA_UCI_2_5_ROOT` at build time; the Sleet round trip also
//! needs `AMS_GRA_SLEET_BIN`. Without them the tests are stated skips.

#[cfg(not(ams_gra_real_uci))]
#[test]
fn task052_real_subsystem_stream_hex_binary_round_trips() {
    eprintln!("SKIPPED: AMS_GRA_UCI_2_5_ROOT was not set when this crate was built");
}

#[cfg(not(ams_gra_real_uci))]
#[test]
fn task052_real_subsystem_stream_round_trips_through_real_sleet() {
    eprintln!("SKIPPED: AMS_GRA_UCI_2_5_ROOT was not set when this crate was built");
}

#[cfg(ams_gra_real_uci)]
mod real {
    use ams_gra_oms_runtime_rust::{OmsJsonCodec, RuntimeConfig, SleetRuntime};
    use ams_gra_oms_runtime_rust_facade_tests::real_uci_subsystem_stream::model as m;
    use ams_gra_oms_runtime_rust_facade_tests::real_uci_subsystem_stream::service_api::function_loop as svc;
    use ams_gra_oms_runtime_rust_facade_tests::real_uci_subsystem_stream::service_codec::ServiceCodec;
    use serde_json::{Value, json};
    use std::net::TcpListener;
    use std::process::{Child, Command, Stdio};
    use std::sync::mpsc;
    use std::time::{Duration, Instant};

    const WAIT: Duration = Duration::from_secs(10);
    const UUID: &str = "550e8400-e29b-41d4-a716-446655440000";
    const STAMP: &str = "2026-01-01T00:00:00Z";

    /// The payload body (no global element) with `binary` as the
    /// SubsystemStreamBinary member.
    fn body(binary: &str) -> Value {
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
            "MessageData": {
                "SubsystemID": { "UUID": UUID },
                "SubsystemSetting": "DEBUG_REPORTING",
                "SubsystemStreamBinary": binary
            }
        })
    }

    fn decode(value: &Value) -> m::SubsystemStreamMT {
        ServiceCodec
            .decode_payload(value)
            .expect("generated codec decodes the real SubsystemStream")
    }

    /// Real UCI hexBinary field: lowercase in, typed octets, canonical
    /// uppercase out; every other member re-encodes identically.
    #[test]
    fn task052_real_subsystem_stream_hex_binary_round_trips() {
        let report = decode(&body("000aeeff"));
        let data = &report.messagedata;
        assert_eq!(
            data.subsystemstreambinary.as_deref(),
            Some(&[0x00, 0x0A, 0xEE, 0xFF][..])
        );
        assert_eq!(data.subsystemid.uuid.as_str(), UUID);
        assert_eq!(
            data.subsystemsetting,
            m::SubsystemSettingEnum::DEBUGREPORTING
        );
        assert_eq!(report.messageheader.timestamp.as_str(), STAMP);
        let encoded = ServiceCodec.encode_payload(&report).expect("encode");
        assert_eq!(encoded, body("000AEEFF"));

        // Absent optional Binary: no member either way.
        let mut absent = body("");
        absent["MessageData"]
            .as_object_mut()
            .expect("object")
            .remove("SubsystemStreamBinary");
        let decoded = decode(&absent);
        assert_eq!(decoded.messagedata.subsystemstreambinary, None);
        assert_eq!(
            ServiceCodec.encode_payload(&decoded).expect("encode"),
            absent
        );

        // The generated decoder rejects invalid hexBinary with a real path.
        let error = ServiceCodec
            .decode_payload(&body("ABC"))
            .map(|_: m::SubsystemStreamMT| ())
            .expect_err("odd length");
        assert_eq!(
            error.message(),
            "SubsystemStreamMT.MessageData.SubsystemStreamBinary: \"ABC\" is not hexBinary: \
             odd number of hexadecimal digits"
        );
        println!("REAL SUBSYSTEMSTREAM HEXBINARY CODEC: PASSED");
    }

    struct Server(Child);

    impl Drop for Server {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }

    fn start_sleet(binary: &std::ffi::OsStr, dir: &std::path::Path) -> (Server, String) {
        let root = env!("AMS_GRA_UCI_2_5_ROOT_BUILD");
        let services = dir.join("services.d");
        std::fs::create_dir_all(&services).expect("services dir");
        let topic = svc::exchange_output_stream::TOPIC;
        std::fs::write(
            services.join("stream.toml"),
            format!(
                "service_id = \"subsystem-stream-loop\"\n\
                 service_uuid = \"550e8400-e29b-41d4-a716-446655440052\"\n\
                 allowed_topics = [\"{topic}\"]\n"
            ),
        )
        .expect("service config");
        let port = TcpListener::bind("127.0.0.1:0")
            .and_then(|listener| listener.local_addr())
            .expect("free port")
            .port();
        let config = dir.join("server.toml");
        std::fs::write(
            &config,
            format!(
                "bind_addr = \"127.0.0.1:{port}\"\nserver_id = \"sleet-task052-uci\"\n\
                 system_label = \"Task 052\"\nschema_path = \"{root}\"\n\
                 schema_version = \"002.5.0\"\n\
                 system_uuid = \"550e8400-e29b-41d4-a716-446655440000\"\n\
                 services_dir = \"{}\"\n",
                services.display()
            ),
        )
        .expect("server config");
        let server = Server(
            Command::new(binary)
                .arg(&config)
                .env("RUST_LOG", "warn")
                .stdout(Stdio::null())
                .stderr(Stdio::inherit())
                .spawn()
                .expect("start Sleet"),
        );
        (server, format!("ws://127.0.0.1:{port}"))
    }

    /// Real UCI 2.5 root in Sleet -> generated typed SubsystemStream with a
    /// hexBinary value -> generated codec -> SleetRuntime -> UNMODIFIED Sleet
    /// (validates the document against the pinned root) -> generated decoder
    /// -> typed handler with identical octets.
    #[test]
    fn task052_real_subsystem_stream_round_trips_through_real_sleet() {
        let Some(binary) = std::env::var_os("AMS_GRA_SLEET_BIN") else {
            eprintln!("SKIPPED: AMS_GRA_SLEET_BIN is not set (see scripts/run-real-sleet-test.sh)");
            return;
        };
        let dir = std::env::temp_dir().join(format!(
            "ams-gra-oms-task052-uci-sleet-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let (_server, url) = start_sleet(&binary, &dir);
        // Loading all of UCI 2.5 takes Sleet a moment; retry until it listens.
        let deadline = Instant::now() + Duration::from_secs(60);
        let mut runtime = loop {
            match SleetRuntime::connect(
                RuntimeConfig::new(&url, "subsystem-stream-loop", "002.5.0"),
                ServiceCodec,
            ) {
                Ok(runtime) => break runtime,
                Err(_) if Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(200));
                }
                Err(error) => panic!("cannot connect to Sleet: {error}"),
            }
        };

        let value = decode(&body("00a1ff"));
        let (seen_tx, seen) = mpsc::channel();
        let subscription = svc::exchange_input_stream::subscribe(
            &mut runtime,
            move |message: &svc::exchange_input_stream::Payload| {
                seen_tx.send(message.clone()).expect("test alive");
            },
        )
        .expect("subscribe");
        svc::exchange_output_stream::publish(&mut runtime, &value).expect("publish");
        let received = seen
            .recv_timeout(WAIT)
            .expect("Sleet routed SubsystemStream back");
        assert!(
            runtime.try_recv_event().is_none(),
            "Sleet accepted the document"
        );
        assert_eq!(
            received.messagedata.subsystemstreambinary.as_deref(),
            Some(&[0x00, 0xA1, 0xFF][..])
        );
        assert_eq!(
            ServiceCodec.encode_payload(&received).expect("encode"),
            body("00A1FF")
        );
        subscription.unsubscribe().expect("unsubscribe");
        runtime.close().expect("close");
        let _ = std::fs::remove_dir_all(&dir);
        eprintln!("REAL UCI SUBSYSTEMSTREAM THROUGH REAL SLEET: PASSED");
    }
}
