//! Task 050 main acceptance: the REAL UCI 2.5 PositionReport, end to end
//! through GENERATED code only. No handwritten PositionReport codec exists.
//!
//! Requires `AMS_GRA_UCI_2_5_ROOT` at build time (the SHA-256-verified pinned
//! root, see build.rs). Without it these tests are compiled as stated skips.
//! The Sleet round trip additionally needs `AMS_GRA_SLEET_BIN`.
//!
//! Vector: `tests/fixtures/oms-json/sleet-e38f61d8/PositionReport.json`, the
//! byte-identical copy of open-arsenal/ams-gra-hello-world-sk-infra-sleet
//! @ e38f61d8 `tests/fixtures/uci/v2_5/PositionReport.json` (SHA-256
//! 5993d423...b148b; provenance in docs/task-050-rust-oms-json-codecs.md).

#[cfg(not(ams_gra_real_uci))]
#[test]
fn task050_real_position_report_decodes_and_re_encodes() {
    eprintln!("SKIPPED: AMS_GRA_UCI_2_5_ROOT was not set when this crate was built");
}

#[cfg(not(ams_gra_real_uci))]
#[test]
fn task050_real_position_report_round_trips_through_real_sleet() {
    eprintln!("SKIPPED: AMS_GRA_UCI_2_5_ROOT was not set when this crate was built");
}

#[cfg(ams_gra_real_uci)]
mod real {
    use ams_gra_oms_runtime_rust::{
        OAM_NAMESPACE, OmsJsonCodec, RuntimeConfig, SleetRuntime, oms_global_element_name,
        unwrap_global_element, wrap_global_element,
    };
    use ams_gra_oms_runtime_rust_facade_tests::real_uci_position_report::model as m;
    use ams_gra_oms_runtime_rust_facade_tests::real_uci_position_report::service_api::function_loop as svc;
    use ams_gra_oms_runtime_rust_facade_tests::real_uci_position_report::service_codec::ServiceCodec;
    use serde_json::Value;
    use std::net::TcpListener;
    use std::path::Path;
    use std::process::{Child, Command, Stdio};
    use std::sync::mpsc;
    use std::time::{Duration, Instant};

    const WAIT: Duration = Duration::from_secs(10);
    const UUID: &str = "550e8400-e29b-41d4-a716-446655440000";
    const STAMP: &str = "2026-01-01T00:00:00Z";

    fn vector() -> String {
        std::fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../tests/fixtures/oms-json/sleet-e38f61d8/PositionReport.json"),
        )
        .expect("pinned vector")
    }

    fn element() -> String {
        // Derived from the GENERATED endpoint, never hard-coded by the test.
        oms_global_element_name(
            svc::exchange_input_position::MESSAGE_NAMESPACE,
            svc::exchange_input_position::MESSAGE_NAME,
        )
    }

    fn decode_vector() -> m::PositionReportMT {
        let body = unwrap_global_element(&element(), &vector()).expect("envelope");
        ServiceCodec
            .decode_payload(&body)
            .expect("generated codec decodes the vector")
    }

    fn assert_representative(report: &m::PositionReportMT) {
        let security = &report.securityinformation;
        assert_eq!(security.classification, m::ClassificationEnum::U);
        assert!(matches!(
            security.ownerproducer.as_slice(),
            [m::OwnerProducerChoiceType::GovernmentIdentifier(
                m::OwnerProducerEnum::USA
            )]
        ));
        let header = &report.messageheader;
        assert_eq!(header.systemid.uuid.as_str(), UUID);
        assert_eq!(header.timestamp.as_str(), STAMP);
        assert_eq!(header.schemaversion.as_str(), "002.5.0");
        assert_eq!(header.mode, m::MessageModeEnum::LIVE);
        let data = &report.messagedata;
        assert_eq!(data.systemid.uuid.as_str(), UUID);
        assert_eq!(data.source, m::SystemSourceEnum::ACTUAL);
        assert_eq!(data.currentoperatingdomain, m::EnvironmentEnum::AIR);
        let position = &data.inertialstate.position;
        assert_eq!(position.latitude.get(), 1.0);
        assert_eq!(position.longitude.get(), 1.0);
        assert_eq!(position.altitude.get(), 1.0);
        assert_eq!(position.timestamp.as_str(), STAMP);
    }

    #[test]
    fn task050_real_position_report_decodes_and_re_encodes() {
        assert_eq!(
            svc::exchange_input_position::MESSAGE_NAMESPACE,
            OAM_NAMESPACE
        );
        let report = decode_vector();
        assert_representative(&report);
        let encoded = wrap_global_element(
            element(),
            ServiceCodec.encode_payload(&report).expect("encode"),
        );
        let original: Value = serde_json::from_str(&vector()).expect("vector JSON");
        // Parsed-value equality: member order is irrelevant.
        assert_eq!(encoded, original);
        println!("REAL POSITIONREPORT CODEC: PASSED");
    }

    struct Server(Child);

    impl Drop for Server {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }

    /// XSD -> generated model -> generated codec -> generated facade ->
    /// runtime-rust -> sleet-client -> UNMODIFIED Sleet (loaded with the SAME
    /// pinned UCI 2.5 root, which it validates the document against) ->
    /// generated codec -> typed PositionReportMT handler.
    #[test]
    fn task050_real_position_report_round_trips_through_real_sleet() {
        let Some(binary) = std::env::var_os("AMS_GRA_SLEET_BIN") else {
            eprintln!("SKIPPED: AMS_GRA_SLEET_BIN is not set (see scripts/run-real-sleet-test.sh)");
            return;
        };
        let root = env!("AMS_GRA_UCI_2_5_ROOT_BUILD");
        let dir = std::env::temp_dir().join(format!(
            "ams-gra-oms-task050-uci-sleet-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let services = dir.join("services.d");
        std::fs::create_dir_all(&services).expect("services dir");
        let topic = svc::exchange_output_position::TOPIC;
        std::fs::write(
            services.join("position.toml"),
            format!(
                "service_id = \"position-report-loop\"\n\
                 service_uuid = \"550e8400-e29b-41d4-a716-446655440051\"\n\
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
                "bind_addr = \"127.0.0.1:{port}\"\nserver_id = \"sleet-task050-uci\"\n\
                 system_label = \"Task 050\"\nschema_path = \"{root}\"\n\
                 schema_version = \"002.5.0\"\n\
                 system_uuid = \"550e8400-e29b-41d4-a716-446655440000\"\n\
                 services_dir = \"{}\"\n",
                services.display()
            ),
        )
        .expect("server config");
        let _server = Server(
            Command::new(binary)
                .arg(&config)
                .env("RUST_LOG", "warn")
                .stdout(Stdio::null())
                .stderr(Stdio::inherit())
                .spawn()
                .expect("start Sleet"),
        );
        let url = format!("ws://127.0.0.1:{port}");
        // Loading all of UCI 2.5 takes Sleet a moment; retry until it listens.
        let deadline = Instant::now() + Duration::from_secs(60);
        let mut runtime = loop {
            match SleetRuntime::connect(
                RuntimeConfig::new(&url, "position-report-loop", "002.5.0"),
                ServiceCodec,
            ) {
                Ok(runtime) => break runtime,
                Err(_) if Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(200));
                }
                Err(error) => panic!("cannot connect to Sleet: {error}"),
            }
        };

        // 1. Pinned JSON -> generated typed value, through the generated codec.
        let report = decode_vector();
        // 3. Subscribe through the GENERATED input endpoint.
        let (seen_tx, seen) = mpsc::channel();
        let subscription = svc::exchange_input_position::subscribe(
            &mut runtime,
            move |message: &svc::exchange_input_position::Payload| {
                seen_tx.send(message.clone()).expect("test alive");
            },
        )
        .expect("subscribe");
        // 4. Publish the typed value through the GENERATED output endpoint.
        svc::exchange_output_position::publish(&mut runtime, &report).expect("publish");
        // 5-7. Sleet validates and routes it back; the handler gets it typed.
        let received = seen
            .recv_timeout(WAIT)
            .expect("Sleet routed PositionReport back");
        assert_representative(&received);
        assert!(
            runtime.try_recv_event().is_none(),
            "Sleet accepted the document"
        );
        // 8-9.
        subscription.unsubscribe().expect("unsubscribe");
        runtime.close().expect("close");
        let _ = std::fs::remove_dir_all(&dir);
        eprintln!("REAL UCI POSITIONREPORT THROUGH REAL SLEET: PASSED");
    }
}
