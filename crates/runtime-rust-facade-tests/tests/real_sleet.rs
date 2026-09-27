//! Task 049: GENERATED Rust façade -> runtime-rust -> pinned sleet-client ->
//! the UNMODIFIED pinned Sleet server
//! (open-arsenal/ams-gra-hello-world-sk-infra-sleet @ e38f61d8).
//!
//! Needs a Sleet binary: set `AMS_GRA_SLEET_BIN` (see
//! `scripts/run-real-sleet-test.sh`, which builds it from a fresh checkout
//! of the exact revision). Without the variable this test is a skip, and it
//! says so; with it, every failure is a hard failure.
//!
//! Sleet is configured here with the synthetic `runtime-oam.xsd` (OAM
//! namespace, version 000.1.0) and one service allowed only `loop-topic`.

use ams_gra_oms_runtime_rust::{RuntimeConfig, RuntimeEvent, SleetRuntime};
use ams_gra_oms_runtime_rust_facade_tests::codec::{TestCodec, oam_shared};
use ams_gra_oms_runtime_rust_facade_tests::runtime_oam::service_api::function_loop as oam;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
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

fn configure(dir: &Path, port: u16) -> PathBuf {
    let schema = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/service-generate/runtime-oam.xsd")
        .canonicalize()
        .expect("fixture");
    let services = dir.join("services.d");
    std::fs::create_dir_all(&services).expect("services dir");
    std::fs::write(
        services.join("loop.toml"),
        "service_id = \"runtime-oam\"\n\
         service_uuid = \"550e8400-e29b-41d4-a716-446655440049\"\n\
         allowed_topics = [\"loop-topic\"]\n",
    )
    .expect("service config");
    let server = dir.join("server.toml");
    std::fs::write(
        &server,
        format!(
            "bind_addr = \"127.0.0.1:{port}\"\n\
             server_id = \"sleet-task049\"\n\
             system_label = \"Task 049\"\n\
             schema_path = \"{}\"\n\
             schema_version = \"000.1.0\"\n\
             system_uuid = \"550e8400-e29b-41d4-a716-446655440000\"\n\
             services_dir = \"{}\"\n",
            schema.display(),
            services.display()
        ),
    )
    .expect("server config");
    server
}

fn connect(url: &str) -> SleetRuntime<TestCodec> {
    let deadline = Instant::now() + WAIT;
    loop {
        match SleetRuntime::connect(RuntimeConfig::new(url, "runtime-oam", "000.1.0"), TestCodec) {
            Ok(runtime) => return runtime,
            Err(error) if Instant::now() < deadline => {
                let _ = error;
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(error) => panic!("cannot connect to Sleet: {error}"),
        }
    }
}

#[test]
fn task049_generated_facade_round_trips_through_real_sleet() {
    let Some(binary) = std::env::var_os("AMS_GRA_SLEET_BIN") else {
        eprintln!("SKIPPED: AMS_GRA_SLEET_BIN is not set (see scripts/run-real-sleet-test.sh)");
        return;
    };
    let dir =
        std::env::temp_dir().join(format!("ams-gra-oms-task049-sleet-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let port = free_port();
    let config = configure(&dir, port);
    let _server = Server(
        Command::new(binary)
            .arg(&config)
            .env("RUST_LOG", "warn")
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("start Sleet"),
    );
    let mut runtime = connect(&format!("ws://127.0.0.1:{port}"));

    let (seen_tx, seen) = mpsc::channel();
    let subscription = oam::exchange_input_a::subscribe(
        &mut runtime,
        move |m: &oam::exchange_input_a::Payload| {
            seen_tx.send(m.count.get()).expect("test alive");
        },
    )
    .expect("subscribe");
    assert_eq!(subscription.id(), "sub-1");

    // Same payload, same topic: Sleet routes MessageA to the MessageA
    // subscription and does NOT route MessageB to it.
    oam::exchange_output_b::publish(&mut runtime, &oam_shared(7)).expect("publish B");
    oam::exchange_output_a::publish(&mut runtime, &oam_shared(42)).expect("publish A");
    assert_eq!(seen.recv_timeout(WAIT), Ok(42));
    assert!(seen.recv_timeout(Duration::from_millis(500)).is_err());

    // A topic this service is not allowed: non-verbose success, then -ERR.
    oam::exchange_denied::publish(&mut runtime, &oam_shared(1)).expect("written");
    match runtime.recv_event_timeout(WAIT) {
        Some(RuntimeEvent::ServerError { error, details }) => {
            assert_eq!(error, "Illegal-State");
            assert!(details.is_some_and(|d| d.contains("denied-topic")));
        }
        other => panic!("expected ServerError, got {other:?}"),
    }

    subscription.unsubscribe().expect("unsubscribe");
    oam::exchange_output_a::publish(&mut runtime, &oam_shared(43)).expect("publish A");
    assert!(seen.recv_timeout(Duration::from_millis(500)).is_err());
    assert!(runtime.try_recv_event().is_none(), "UNSUB must be accepted");
    runtime.close().expect("close");
    let _ = std::fs::remove_dir_all(&dir);
    eprintln!("REAL SLEET: PASSED");
}
