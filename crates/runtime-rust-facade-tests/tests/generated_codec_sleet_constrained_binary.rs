//! Task 053: GENERATED constrained named Binary carriers -> GENERATED hex
//! codec -> runtime-rust -> pinned sleet-client -> the UNMODIFIED pinned
//! Sleet server (GitLab reviewed candidate @ 94796daf)
//! -> GENERATED decoder -> GENERATED checked constructors -> typed handler.
//!
//! A NEW test beside Task 052's unconstrained hexBinary real-Sleet test,
//! which is unchanged. Needs `AMS_GRA_SLEET_BIN` (see
//! `scripts/run-real-sleet-test.sh`); without it the test is a stated skip.
//!
//! Require exact and ranged lengths in octets, broker rejection before
//! delivery, and valid recovery on the same subscription.

use ams_gra_oms_runtime_rust::{OmsJsonCodec, RuntimeConfig, RuntimeEvent, SleetRuntime};
use ams_gra_oms_runtime_rust_facade_tests::constrained_binary_sleet::model as m;
use ams_gra_oms_runtime_rust_facade_tests::constrained_binary_sleet::service_api::function_sleet_blob as blob;
use ams_gra_oms_runtime_rust_facade_tests::constrained_binary_sleet::service_codec::ServiceCodec;
use std::net::TcpListener;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

const WAIT: Duration = Duration::from_secs(10);
const SERVICE: &str = "constrained-binary-sleet";
const TOPIC: &str = "sleet-blob-topic";

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

/// Representative legal octet counts.
fn sample(id: i64) -> m::SleetBlobPayload {
    m::SleetBlobPayload {
        id: m::BoundedI64::new(id).expect("xs:int"),
        ranged: m::Between2And6::new(vec![0xAB, 0xcd]).expect("2 octets"),
        middle: m::BlobMiddle::new(vec![0x00, 0x0A, 0xEE, 0xFF]).expect("4 octets"),
        zero: m::ZeroBlob::new(Vec::new()).expect("0 octets"),
        exact: None,
        ranges: m::UnboundedVec::new(vec![
            m::Between2And6::new(vec![1, 2]).expect("2"),
            m::Between2And6::new(vec![3, 4, 5]).expect("3"),
        ])
        .expect("0.."),
    }
}

/// Inject a one-octet value that the typed encoder cannot produce.
struct ProbeCodec;

const PROBE_ID: i64 = 777;
const PROBE_SHORT_HEX: &str = "AB";

impl OmsJsonCodec<m::SleetBlobPayload> for ProbeCodec {
    fn encode_payload(
        &self,
        value: &m::SleetBlobPayload,
    ) -> Result<serde_json::Value, ams_gra_oms_runtime_rust::CodecError> {
        let mut encoded = ServiceCodec.encode_payload(value)?;
        if value.id.get() == PROBE_ID {
            encoded["Ranged"] = serde_json::Value::String(PROBE_SHORT_HEX.to_owned());
        }
        Ok(encoded)
    }

    fn decode_payload(
        &self,
        value: &serde_json::Value,
    ) -> Result<m::SleetBlobPayload, ams_gra_oms_runtime_rust::CodecError> {
        ServiceCodec.decode_payload(value)
    }
}

fn start_sleet(binary: &std::ffi::OsStr, dir: &Path) -> (Server, String) {
    let schema = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/service-generate/constrained-binary-sleet.xsd")
        .canonicalize()
        .expect("fixture");
    let services = dir.join("services.d");
    std::fs::create_dir_all(&services).expect("services dir");
    std::fs::write(
        services.join("constrained.toml"),
        format!(
            "service_id = \"{SERVICE}\"\n\
             service_uuid = \"550e8400-e29b-41d4-a716-446655440053\"\n\
             allowed_topics = [\"{TOPIC}\"]\n"
        ),
    )
    .expect("service config");
    let port = free_port();
    let config = dir.join("server.toml");
    std::fs::write(
        &config,
        format!(
            "bind_addr = \"127.0.0.1:{port}\"\nserver_id = \"sleet-task053\"\n\
             system_label = \"Task 053\"\nschema_path = \"{}\"\n\
             schema_version = \"000.1.0\"\n\
             system_uuid = \"550e8400-e29b-41d4-a716-446655440000\"\n\
             services_dir = \"{}\"\n",
            schema.display(),
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

fn connect<C: Send + Sync + 'static>(url: &str, codec: impl Fn() -> C) -> SleetRuntime<C> {
    let deadline = Instant::now() + WAIT;
    loop {
        match SleetRuntime::connect(RuntimeConfig::new(url, SERVICE, "000.1.0"), codec()) {
            Ok(runtime) => break runtime,
            Err(_) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(100)),
            Err(error) => panic!("cannot connect to Sleet: {error}"),
        }
    }
}

fn drain<C>(runtime: &mut SleetRuntime<C>) -> Vec<RuntimeEvent> {
    let mut events = Vec::new();
    while let Some(event) = runtime.recv_event_timeout(Duration::from_millis(1500)) {
        events.push(event);
    }
    events
}

fn server_errors(events: &[RuntimeEvent]) -> Vec<String> {
    events
        .iter()
        .filter_map(|event| match event {
            RuntimeEvent::ServerError { details, .. } => details.clone(),
            _ => None,
        })
        .collect()
}

#[test]
fn task053_constrained_binary_generated_codec_round_trips_through_real_sleet() {
    let Some(binary) = std::env::var_os("AMS_GRA_SLEET_BIN") else {
        eprintln!("SKIPPED: AMS_GRA_SLEET_BIN is not set (see scripts/run-real-sleet-test.sh)");
        return;
    };
    let dir =
        std::env::temp_dir().join(format!("ams-gra-oms-task053-sleet-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let (_server, url) = start_sleet(&binary, &dir);

    // 1. Legal octet counts: typed carriers -> generated canonical hex ->
    //    Sleet -> generated decoder + checked constructors -> typed handler.
    let mut runtime = connect(&url, || ServiceCodec);
    let (seen_tx, seen) = mpsc::channel();
    let subscription = blob::exchange_input_sleet_blob::subscribe(
        &mut runtime,
        move |message: &blob::exchange_input_sleet_blob::Payload| {
            seen_tx.send(message.clone()).expect("test alive");
        },
    )
    .expect("subscribe");
    let value = sample(53);
    let encoded = ServiceCodec.encode_payload(&value).expect("encode");
    assert_eq!(
        encoded["Ranged"],
        serde_json::json!("ABCD"),
        "canonical uppercase"
    );
    assert_eq!(encoded["Middle"], serde_json::json!("000AEEFF"));
    assert_eq!(encoded["Zero"], serde_json::json!(""));
    blob::exchange_output_sleet_blob::publish(&mut runtime, &value).expect("publish");
    let received = match seen.recv_timeout(WAIT) {
        Ok(received) => received,
        Err(error) => panic!(
            "Sleet did not route SleetBlobNotice back ({error:?}): {:?}",
            drain(&mut runtime)
        ),
    };
    assert!(
        runtime.try_recv_event().is_none(),
        "Sleet accepted the document"
    );
    assert_eq!(
        received, value,
        "constrained carriers survive the real Sleet loop"
    );
    assert_eq!(received.ranged.as_slice(), [0xAB, 0xCD]);
    assert_eq!(received.middle.as_slice(), [0x00, 0x0A, 0xEE, 0xFF]);
    eprintln!("REAL SLEET CONSTRAINED BINARY: legal document accepted and routed");

    // Exact and upper ranged boundaries must survive with identical bytes.
    let mut with_exact = sample(54);
    with_exact.exact = Some(m::Exact4::new(vec![0xDE, 0xAD, 0xBE, 0xEF]).expect("4 octets"));
    with_exact.ranged = m::Between2And6::new(vec![0xAB; 6]).unwrap();
    blob::exchange_output_sleet_blob::publish(&mut runtime, &with_exact).unwrap();
    assert_eq!(seen.recv_timeout(WAIT).unwrap(), with_exact);
    assert!(drain(&mut runtime).is_empty());
    subscription.unsubscribe().expect("unsubscribe");
    runtime.close().expect("close");

    // A short range must fail at the broker, not at the generated decoder.
    let mut probe = connect(&url, || ProbeCodec);
    let (probe_tx, probe_seen) = mpsc::channel();
    let probe_subscription = blob::exchange_input_sleet_blob::subscribe(
        &mut probe,
        move |message: &blob::exchange_input_sleet_blob::Payload| {
            probe_tx
                .send(message.ranged.as_slice().len())
                .expect("test alive");
        },
    )
    .expect("subscribe");
    let publish = blob::exchange_output_sleet_blob::publish(&mut probe, &sample(PROBE_ID));
    let events = drain(&mut probe);
    eprintln!(
        "REAL SLEET CONSTRAINED BINARY PROBE: publish = {:?}",
        publish.as_ref().err()
    );
    eprintln!("REAL SLEET CONSTRAINED BINARY PROBE: events = {events:?}");
    // The invariant, independent of Sleet: no handler sees a 1-octet value.
    assert!(
        probe_seen.recv_timeout(Duration::from_millis(200)).is_err(),
        "a 1-octet Between2And6 must never reach a typed handler"
    );
    assert!(publish.is_ok(), "non-verbose PUB frame written");
    assert_eq!(events.len(), 1, "one broker error, no decoder error");
    let errors = server_errors(&events);
    assert_eq!(errors.len(), 1);
    assert!(errors[0].contains("FacetViolation"), "{errors:?}");
    assert!(errors[0].contains("Ranged"), "{errors:?}");
    blob::exchange_output_sleet_blob::publish(&mut probe, &sample(55)).unwrap();
    assert_eq!(probe_seen.recv_timeout(WAIT).unwrap(), 2);
    assert!(drain(&mut probe).is_empty());
    probe_subscription
        .unsubscribe()
        .expect("unsubscribe after recovery");
    probe.close().expect("close after recovery");
    let _ = std::fs::remove_dir_all(&dir);
    eprintln!("REAL SLEET GENERATED CONSTRAINED BINARY CODEC: PASSED");
}
