//! Task 052: GENERATED xs:hexBinary typed value -> GENERATED hex codec ->
//! runtime-rust -> pinned sleet-client -> the UNMODIFIED pinned Sleet server
//! (open-arsenal/ams-gra-hello-world-sk-infra-sleet @ e38f61d8) -> GENERATED
//! decoder -> typed byte values.
//!
//! A NEW test beside the Task 049/050/051 real-Sleet tests. Needs
//! `AMS_GRA_SLEET_BIN` (see `scripts/run-real-sleet-test.sh`); without it the
//! test is a stated skip.
//!
//! Sleet loads `codec-hexbinary.xsd` and validates each PUB document before
//! routing it. Pinned Sleet maps every XSD primitive it does not special-case
//! (including `xs:hexBinary`) to its generic JSON-string check
//! (`sleet/src/validator.rs::SimpleValueKind::from_primitive`), so it checks
//! only that each Binary value is a JSON string, not hexBinary lexical
//! syntax. The second part of this test RECORDS that: an invalid hexBinary
//! string is routed by Sleet and rejected by the GENERATED decoder, which is
//! the authority for hexBinary lexical validity. The codec is never loosened
//! to match Sleet.

use ams_gra_oms_runtime_rust::{MessageDecodeError, RuntimeConfig, RuntimeEvent, SleetRuntime};
use ams_gra_oms_runtime_rust_facade_tests::codec_hexbinary::model as m;
use ams_gra_oms_runtime_rust_facade_tests::codec_hexbinary::service_api::function_blob as blob;
use ams_gra_oms_runtime_rust_facade_tests::codec_hexbinary::service_codec::ServiceCodec;
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

fn int(value: i64) -> m::BoundedI64<-2147483648, 2147483647> {
    m::BoundedI64::new(value).expect("xs:int")
}

fn sample() -> m::BlobPayload {
    m::BlobPayload {
        id: int(52),
        data: vec![0x00, 0x0A, 0xEE, 0xFF],
        maybedata: Some(vec![0xCA, 0xFE]),
        chunks: m::BoundedVec::new(vec![vec![0x00, 0x11], vec![]]).expect("2"),
        stream: m::UnboundedVec::new(vec![vec![0x01, 0x02]]).expect("1"),
        blob: m::BlobBytes::new(vec![0xDE, 0xAD]),
        alias: Some(m::BlobAlias::new(vec![0x5A])),
        atomic: m::AtomicLike::HexBinaryValue(vec![0x00, 0xA1, 0xFF]),
    }
}

/// Test-only PROBE codec. It delegates to the GENERATED `ServiceCodec` in
/// both directions; for the marked `Id` only, it overwrites the encoded
/// `Data` member with a non-hexBinary string the generated encoder can never
/// produce. This is how the test learns whether pinned Sleet validates
/// hexBinary syntax. Decode is exactly the generated decoder.
struct ProbeCodec;

const PROBE_ID: i64 = 666;
const PROBE_INVALID_HEX: &str = "0G";

impl ams_gra_oms_runtime_rust::OmsJsonCodec<m::BlobPayload> for ProbeCodec {
    fn encode_payload(
        &self,
        value: &m::BlobPayload,
    ) -> Result<serde_json::Value, ams_gra_oms_runtime_rust::CodecError> {
        let mut encoded = ServiceCodec.encode_payload(value)?;
        if value.id.get() == PROBE_ID {
            encoded["Data"] = serde_json::Value::String(PROBE_INVALID_HEX.to_owned());
        }
        Ok(encoded)
    }

    fn decode_payload(
        &self,
        value: &serde_json::Value,
    ) -> Result<m::BlobPayload, ams_gra_oms_runtime_rust::CodecError> {
        ServiceCodec.decode_payload(value)
    }
}

fn start_sleet(binary: &std::ffi::OsStr, dir: &Path) -> (Server, String) {
    let schema = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/service-generate/codec-hexbinary.xsd")
        .canonicalize()
        .expect("fixture");
    let services = dir.join("services.d");
    std::fs::create_dir_all(&services).expect("services dir");
    std::fs::write(
        services.join("hex.toml"),
        "service_id = \"codec-hexbinary\"\n\
         service_uuid = \"550e8400-e29b-41d4-a716-446655440052\"\n\
         allowed_topics = [\"blob-topic\"]\n",
    )
    .expect("service config");
    let port = free_port();
    let config = dir.join("server.toml");
    std::fs::write(
        &config,
        format!(
            "bind_addr = \"127.0.0.1:{port}\"\nserver_id = \"sleet-task052\"\n\
             system_label = \"Task 052\"\nschema_path = \"{}\"\n\
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

/// Connect, retrying while Sleet starts. `codec` builds a fresh (unit)
/// codec for each attempt, because a failed `connect` consumes it.
fn connect<C: Send + Sync + 'static>(url: &str, codec: impl Fn() -> C) -> SleetRuntime<C> {
    let deadline = Instant::now() + WAIT;
    loop {
        match SleetRuntime::connect(
            RuntimeConfig::new(url, "codec-hexbinary", "000.1.0"),
            codec(),
        ) {
            Ok(runtime) => break runtime,
            Err(_) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(100)),
            Err(error) => panic!("cannot connect to Sleet: {error}"),
        }
    }
}

#[test]
fn task052_hex_binary_generated_codec_round_trips_through_real_sleet() {
    let Some(binary) = std::env::var_os("AMS_GRA_SLEET_BIN") else {
        eprintln!("SKIPPED: AMS_GRA_SLEET_BIN is not set (see scripts/run-real-sleet-test.sh)");
        return;
    };
    let dir =
        std::env::temp_dir().join(format!("ams-gra-oms-task052-sleet-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let (_server, url) = start_sleet(&binary, &dir);

    // 1. Generated typed bytes -> generated codec -> Sleet (validates the
    //    document against codec-hexbinary.xsd) -> generated decoder -> typed
    //    handler with identical octets.
    let mut runtime = connect(&url, || ServiceCodec);
    let (seen_tx, seen) = mpsc::channel();
    let subscription = blob::exchange_input_blob::subscribe(
        &mut runtime,
        move |message: &blob::exchange_input_blob::Payload| {
            seen_tx.send(message.clone()).expect("test alive");
        },
    )
    .expect("subscribe");
    let value = sample();
    blob::exchange_output_blob::publish(&mut runtime, &value).expect("publish");
    let received = seen
        .recv_timeout(WAIT)
        .expect("Sleet routed BlobNotice back");
    assert!(
        runtime.try_recv_event().is_none(),
        "Sleet accepted the canonical hexBinary document"
    );
    assert_eq!(received, value, "typed octets survive the real Sleet loop");
    assert_eq!(received.data, [0x00, 0x0A, 0xEE, 0xFF]);
    subscription.unsubscribe().expect("unsubscribe");
    runtime.close().expect("close");
    eprintln!("REAL SLEET HEXBINARY: canonical document accepted and routed");

    // 2. Sleet lexical-validation observation (recorded, not relied on).
    let mut probe = connect(&url, || ProbeCodec);
    let (probe_tx, probe_seen) = mpsc::channel();
    let probe_subscription = blob::exchange_input_blob::subscribe(
        &mut probe,
        move |message: &blob::exchange_input_blob::Payload| {
            probe_tx.send(message.id.get()).expect("test alive");
        },
    )
    .expect("subscribe");
    let mut invalid = sample();
    invalid.id = int(PROBE_ID);
    let publish = blob::exchange_output_blob::publish(&mut probe, &invalid);
    let mut events = Vec::new();
    while let Some(event) = probe.recv_event_timeout(Duration::from_millis(1500)) {
        events.push(event);
    }
    eprintln!(
        "REAL SLEET HEXBINARY PROBE: publish = {:?}",
        publish.as_ref().err()
    );
    eprintln!("REAL SLEET HEXBINARY PROBE: events = {events:?}");
    // Observed with pinned e38f61d8: Sleet validates hexBinary only as a
    // JSON string, so it ROUTES "0G"; the GENERATED decoder then rejects it
    // and no typed handler runs.
    assert!(publish.is_ok(), "Sleet accepted the PUB");
    let decode_errors: Vec<String> = events
        .iter()
        .filter_map(|event| match event {
            RuntimeEvent::SubscriptionDecodeError {
                error: MessageDecodeError::Codec(error),
                ..
            } => Some(error.message().to_owned()),
            _ => None,
        })
        .collect();
    assert_eq!(
        decode_errors,
        [format!(
            "BlobPayload.Data: {PROBE_INVALID_HEX:?} is not hexBinary: only hexadecimal \
             digits [0-9A-Fa-f] are allowed"
        )],
        "pinned Sleet outcome changed; re-run the probe and update the Task 052 record"
    );
    assert!(probe_seen.recv_timeout(Duration::from_millis(200)).is_err());
    eprintln!(
        "REAL SLEET HEXBINARY PROBE: pinned Sleet routed {PROBE_INVALID_HEX:?} (validates \
         hexBinary only as a JSON string); the GENERATED decoder rejected it"
    );
    let _ = probe_subscription.unsubscribe();
    let _ = probe.close();
    let _ = std::fs::remove_dir_all(&dir);
    eprintln!("REAL SLEET GENERATED HEXBINARY CODEC: PASSED");
}
