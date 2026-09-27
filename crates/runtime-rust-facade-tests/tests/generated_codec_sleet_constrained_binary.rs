//! Task 053: GENERATED constrained named Binary carriers -> GENERATED hex
//! codec -> runtime-rust -> pinned sleet-client -> the UNMODIFIED pinned
//! Sleet server (open-arsenal/ams-gra-hello-world-sk-infra-sleet @ e38f61d8)
//! -> GENERATED decoder -> GENERATED checked constructors -> typed handler.
//!
//! A NEW test beside Task 052's unconstrained hexBinary real-Sleet test,
//! which is unchanged. Needs `AMS_GRA_SLEET_BIN` (see
//! `scripts/run-real-sleet-test.sh`); without it the test is a stated skip.
//!
//! # Recorded external Sleet behavior
//!
//! Pinned Sleet (`sleet/src/facets.rs`) measures `xs:length`/`minLength`/
//! `maxLength` of a hexBinary value in lexical CHARACTERS; XML Schema 2E
//! 4.3.1 measures them in OCTETS (two hex characters each). So pinned Sleet
//! REJECTS the spec-correct canonical encoding of a legal `Exact4` (8
//! characters, `length 8 != 4`) and would ACCEPT a 2-octet value. This test
//! uses `constrained-binary-sleet.xsd`, whose required members admit octet
//! counts that also satisfy Sleet's character count, and records the
//! `Exact4` divergence separately. The generated codec is never loosened to
//! match Sleet: the generated carrier is the authority.

use ams_gra_oms_runtime_rust::{
    MessageDecodeError, OmsJsonCodec, RuntimeConfig, RuntimeEvent, SleetRuntime,
};
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

/// Legal octet counts that are ALSO legal Sleet character counts:
/// `Ranged` 2 octets = 4 chars (2..6), `Middle` 4 octets = 8 chars (4..16),
/// `Zero` 0 octets = 0 chars.
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

/// Test-only PROBE codec: delegates to the GENERATED codec both ways; for the
/// marked `Id` only, it overwrites `Ranged` with lexically VALID hex of an
/// ILLEGAL octet count (1 octet) that is a LEGAL Sleet character count (2).
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

fn decode_errors(events: &[RuntimeEvent]) -> Vec<String> {
    events
        .iter()
        .filter_map(|event| match event {
            RuntimeEvent::SubscriptionDecodeError {
                error: MessageDecodeError::Codec(error),
                ..
            } => Some(error.message().to_owned()),
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

    // 2. OBSERVATION: a legal 4-octet Exact4 encodes to 8 characters, which
    //    pinned Sleet rejects (it counts characters). Recorded, not relied on.
    let mut with_exact = sample(54);
    with_exact.exact = Some(m::Exact4::new(vec![0xDE, 0xAD, 0xBE, 0xEF]).expect("4 octets"));
    let publish = blob::exchange_output_sleet_blob::publish(&mut runtime, &with_exact);
    let events = drain(&mut runtime);
    eprintln!(
        "REAL SLEET EXACT4 OBSERVATION: publish = {:?}",
        publish.as_ref().err()
    );
    eprintln!("REAL SLEET EXACT4 OBSERVATION: events = {events:?}");
    assert!(seen.try_recv().is_err(), "not routed by pinned Sleet");
    assert_eq!(
        server_errors(&events),
        [
            "schema validation failed: FacetViolation { path: \"SleetBlobNotice.Exact\", \
          facet: \"length\", detail: \"length 8 != 4\" }"
        ],
        "pinned Sleet outcome changed; update the Task 053 record"
    );
    eprintln!(
        "REAL SLEET EXACT4 OBSERVATION: pinned Sleet counts hexBinary length in CHARACTERS \
         and rejects a spec-legal 4-octet Exact4 (8 characters)"
    );
    subscription.unsubscribe().expect("unsubscribe");
    runtime.close().expect("close");

    // 3. OBSERVATION: 1 octet ("AB", 2 characters) is ILLEGAL for
    //    Between2And6 but satisfies Sleet's 2..6 character check.
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
    assert!(publish.is_ok(), "Sleet accepted the PUB");
    assert_eq!(
        decode_errors(&events),
        ["SleetBlobPayload.Ranged: value rejected by generated Between2And6::new"],
        "pinned Sleet outcome changed; update the Task 053 record"
    );
    eprintln!(
        "REAL SLEET CONSTRAINED BINARY PROBE: pinned Sleet ROUTED {PROBE_SHORT_HEX:?} \
         (1 octet, 2 characters); the GENERATED Between2And6::new rejected it"
    );
    let _ = probe_subscription.unsubscribe();
    let _ = probe.close();
    let _ = std::fs::remove_dir_all(&dir);
    eprintln!("REAL SLEET GENERATED CONSTRAINED BINARY CODEC: PASSED");
}
