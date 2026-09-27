//! Task 050: GENERATED typed value -> GENERATED `ServiceCodec` -> runtime-rust
//! -> pinned sleet-client -> the UNMODIFIED pinned Sleet server
//! (open-arsenal/ams-gra-hello-world-sk-infra-sleet @ e38f61d8) -> GENERATED
//! `ServiceCodec` -> typed handler.
//!
//! This is a NEW test beside Task 049's `real_sleet.rs`, which keeps proving
//! the runtime with a handwritten codec. Needs `AMS_GRA_SLEET_BIN` (see
//! `scripts/run-real-sleet-test.sh`); without it the test is a stated skip.
//!
//! Sleet validates every published OMS JSON document against the schema
//! before routing it, so a successful loop-back also proves the generated
//! encoding is accepted by an independent OMS JSON validator.

use ams_gra_oms_runtime_rust::{RuntimeConfig, SleetRuntime};
use ams_gra_oms_runtime_rust_facade_tests::codec_oam::service_api::function_loop as svc;
use ams_gra_oms_runtime_rust_facade_tests::codec_oam::service_codec::ServiceCodec;
use std::net::TcpListener;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

mod codec_sample;

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
fn task050_generated_codec_round_trips_through_real_sleet() {
    let Some(binary) = std::env::var_os("AMS_GRA_SLEET_BIN") else {
        eprintln!("SKIPPED: AMS_GRA_SLEET_BIN is not set (see scripts/run-real-sleet-test.sh)");
        return;
    };
    let schema = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/service-generate/codec-oam.xsd")
        .canonicalize()
        .expect("fixture");
    let dir =
        std::env::temp_dir().join(format!("ams-gra-oms-task050-sleet-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let services = dir.join("services.d");
    std::fs::create_dir_all(&services).expect("services dir");
    std::fs::write(
        services.join("codec.toml"),
        "service_id = \"codec-oam\"\n\
         service_uuid = \"550e8400-e29b-41d4-a716-446655440050\"\n\
         allowed_topics = [\"codec-topic\"]\n",
    )
    .expect("service config");
    let port = free_port();
    let config = dir.join("server.toml");
    std::fs::write(
        &config,
        format!(
            "bind_addr = \"127.0.0.1:{port}\"\nserver_id = \"sleet-task050\"\n\
             system_label = \"Task 050\"\nschema_path = \"{}\"\n\
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
        match SleetRuntime::connect(
            RuntimeConfig::new(&url, "codec-oam", "000.1.0"),
            ServiceCodec,
        ) {
            Ok(runtime) => break runtime,
            Err(_) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(100)),
            Err(error) => panic!("cannot connect to Sleet: {error}"),
        }
    };

    let (seen_tx, seen) = mpsc::channel();
    let subscription = svc::exchange_input_notice::subscribe(
        &mut runtime,
        move |message: &svc::exchange_input_notice::Payload| {
            seen_tx.send(message.clone()).expect("test alive");
        },
    )
    .expect("subscribe");

    // CodecEcho (same payload type, other global element) must not reach the
    // CodecNotice subscription; CodecNotice must.
    let value = codec_sample::sample();
    svc::exchange_output_echo::publish(&mut runtime, &value).expect("publish echo");
    svc::exchange_output_notice::publish(&mut runtime, &value).expect("publish notice");
    let received = seen
        .recv_timeout(WAIT)
        .expect("Sleet routed CodecNotice back");
    assert!(seen.recv_timeout(Duration::from_millis(500)).is_err());
    assert!(
        runtime.try_recv_event().is_none(),
        "Sleet accepted both documents"
    );

    // The typed handler received the generated model value.
    assert_eq!(received.level.get(), -10);
    assert_eq!(received.signal, codec_sample::m::SignalCode::Value5G);
    assert!(received.history.as_slice()[2].get().is_nan());
    assert_eq!(received.observed.as_str(), "2026-01-01T05:30:00+05:30");
    assert!(matches!(
        received.shape,
        codec_sample::m::ShapeBase::BoxShape(_)
    ));

    subscription.unsubscribe().expect("unsubscribe");
    runtime.close().expect("close");
    let _ = std::fs::remove_dir_all(&dir);
    eprintln!("REAL SLEET GENERATED CODEC: PASSED");
}
