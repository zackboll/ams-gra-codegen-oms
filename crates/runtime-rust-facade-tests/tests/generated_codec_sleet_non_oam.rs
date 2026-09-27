//! Task 051 COMPATIBILITY PROBE (not a conformance gate): the qualified
//! NON-OAM `runtime-test.xsd` with its GENERATED codec against the
//! UNMODIFIED pinned Sleet (open-arsenal/ams-gra-hello-world-sk-infra-sleet
//! @ e38f61d8).
//!
//! OMSC-SPC-013 Rev B section 6.1 requires Clark-notation names outside the
//! OAM namespace: SUB `{urn:test}MessageA`, document
//! `{"{urn:test}MessageB":{"{urn:test}Count":7}}`. The pinned Sleet schema
//! parser keys global elements and particle members by BARE local name
//! (`sleet/src/schema/parser.rs::push_element_start`,
//! `sleet/src/validator.rs::validate_oms_json`), so this probe RECORDS the
//! observed outcome and asserts only that the generated codec is not
//! changed to satisfy it. The required Task 051 runtime proof is the mock
//! OWP test in `generated_codec_mock_owp.rs`. Needs `AMS_GRA_SLEET_BIN`.

use ams_gra_oms_runtime_rust::{RuntimeConfig, RuntimeEvent, SleetRuntime};
use ams_gra_oms_runtime_rust_facade_tests::runtime_test_codec::model as m;
use ams_gra_oms_runtime_rust_facade_tests::runtime_test_codec::service_api::function_track as track;
use ams_gra_oms_runtime_rust_facade_tests::runtime_test_codec::service_codec::ServiceCodec;
use std::net::TcpListener;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

const WAIT: Duration = Duration::from_secs(10);

struct Server(Child);

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .and_then(|listener| listener.local_addr())
        .expect("free port")
        .port()
}

#[test]
fn task051_non_oam_generated_codec_probe_against_real_sleet() {
    let Some(binary) = std::env::var_os("AMS_GRA_SLEET_BIN") else {
        eprintln!("SKIPPED: AMS_GRA_SLEET_BIN is not set (see scripts/run-real-sleet-test.sh)");
        return;
    };
    let schema = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/service-generate/runtime-test.xsd")
        .canonicalize()
        .expect("fixture");
    let dir =
        std::env::temp_dir().join(format!("ams-gra-oms-task051-sleet-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let services = dir.join("services.d");
    std::fs::create_dir_all(&services).expect("services dir");
    std::fs::write(
        services.join("probe.toml"),
        "service_id = \"svc-051\"\n\
         service_uuid = \"550e8400-e29b-41d4-a716-446655440051\"\n\
         allowed_topics = [\"input-topic\", \"output-topic\"]\n",
    )
    .expect("service config");
    let port = free_port();
    let config = dir.join("server.toml");
    std::fs::write(
        &config,
        format!(
            "bind_addr = \"127.0.0.1:{port}\"\nserver_id = \"sleet-task051\"\n\
             system_label = \"Task 051\"\nschema_path = \"{}\"\n\
             schema_version = \"000.1.0\"\n\
             system_uuid = \"550e8400-e29b-41d4-a716-446655440000\"\n\
             services_dir = \"{}\"\n",
            schema.display(),
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
    let deadline = Instant::now() + WAIT;
    let mut runtime = loop {
        match SleetRuntime::connect(RuntimeConfig::new(&url, "svc-051", "000.1.0"), ServiceCodec) {
            Ok(runtime) => break runtime,
            Err(_) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(100)),
            Err(error) => panic!("cannot connect to Sleet: {error}"),
        }
    };

    let (seen_tx, seen) = mpsc::channel();
    let subscription = track::exchange_input_a::subscribe(
        &mut runtime,
        move |message: &track::exchange_input_a::Payload| {
            seen_tx.send(message.count.get()).expect("test alive");
        },
    );
    let value = m::SharedPayload {
        count: m::BoundedI64::new(7).expect("xs:int"),
    };
    let publish = track::exchange_output_b::publish(&mut runtime, &value);
    let mut events = Vec::new();
    while let Some(event) = runtime.recv_event_timeout(Duration::from_millis(1500)) {
        events.push(event);
    }
    let delivered = seen.recv_timeout(Duration::from_millis(200)).ok();
    eprintln!(
        "SLEET NON-OAM PROBE: subscribe = {:?}",
        subscription.as_ref().err()
    );
    eprintln!(
        "SLEET NON-OAM PROBE: publish = {:?}",
        publish.as_ref().err()
    );
    eprintln!("SLEET NON-OAM PROBE: events = {events:?}");
    eprintln!("SLEET NON-OAM PROBE: delivered = {delivered:?}");
    // Observed and recorded outcome (docs/task-051-member-qname-provenance.md):
    // pinned Sleet rejects the spec-correct Clark-form GLOBAL element name
    // before any payload member is examined, for both SUB and PUB.
    let servers: Vec<(&str, &str)> = events
        .iter()
        .filter_map(|event| match event {
            RuntimeEvent::ServerError { error, details } => {
                Some((error.as_str(), details.as_deref().unwrap_or("")))
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        servers,
        [
            (
                "Illegal-Argument",
                "unknown message name {urn:test}MessageA"
            ),
            (
                "Invalid-Message",
                "schema validation failed: UnknownGlobalElement { name: \"{urn:test}MessageB\" }"
            ),
        ],
        "pinned Sleet outcome changed; re-run the probe and update the Task 051 record"
    );
    assert_eq!(delivered, None);
    // The generated codec is NOT bent to fit: its keys stay Clark notation.
    let body = ams_gra_oms_runtime_rust::OmsJsonCodec::<m::SharedPayload>::encode_payload(
        &ServiceCodec,
        &value,
    )
    .expect("encode");
    assert_eq!(body, serde_json::json!({ "{urn:test}Count": 7 }));
    eprintln!(
        "SLEET NON-OAM PROBE: REJECTED by pinned Sleet (implementation limitation; \
         the generated codec follows OMSC-SPC-013 and is unchanged)"
    );
    if let Ok(subscription) = subscription {
        let _ = subscription.unsubscribe();
    }
    let _ = runtime.close();
    let _ = std::fs::remove_dir_all(&dir);
    eprintln!("SLEET NON-OAM PROBE: RECORDED");
}
