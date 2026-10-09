//! Task071 production CLI output consumed by the unmodified pinned server.
use ams_gra_oms_runtime_api::SubscribeAdapter;
use ams_gra_oms_runtime_rust::{RuntimeConfig, RuntimeEvent, SleetRuntime};
use ams_gra_oms_runtime_rust_facade_tests::codec::{TestCodec, oam_shared};
use ams_gra_oms_runtime_rust_facade_tests::runtime_oam::service_api::function_loop as oam;
use std::ffi::OsString;
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

fn rejection(runtime: &SleetRuntime<TestCodec>, expected: &str) {
    match runtime.recv_event_timeout(WAIT) {
        Some(RuntimeEvent::ServerError { error, details }) => {
            assert_eq!(error, "Illegal-State");
            assert!(details.is_some_and(|d| d.contains(expected)));
        }
        other => panic!("expected asynchronous rejection, got {other:?}"),
    }
}

#[test]
fn task071_real_sleet_policy() {
    let Some(binary) = std::env::var_os("AMS_GRA_SLEET_BIN") else {
        eprintln!("SKIPPED: AMS_GRA_SLEET_BIN missing");
        return;
    };
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let dir = std::env::temp_dir().join(format!("task071-policy-{}", std::process::id()));
    std::fs::create_dir_all(dir.join("services")).unwrap();
    let schema = root.join("tests/fixtures/service-generate/runtime-oam.xsd");
    let mut policy = Vec::new();
    ams_gra_codegen_oms::run(
        [
            OsString::from("service-routes"),
            OsString::from("--schema"),
            schema.clone().into(),
            OsString::from("--contract"),
            root.join("tests/fixtures/service-routes/policy.yaml")
                .into(),
            OsString::from("--format"),
            OsString::from("sleet-toml"),
            OsString::from("--service-id"),
            OsString::from("runtime-oam"),
            OsString::from("--service-uuid"),
            OsString::from("550e8400-e29b-41d4-a716-446655440071"),
        ],
        &mut policy,
    )
    .unwrap();
    assert_eq!(
        String::from_utf8(policy.clone())
            .unwrap()
            .matches("[[topic_bindings]]")
            .count(),
        1
    );
    std::fs::write(dir.join("services/policy.toml"), policy).unwrap();
    // The actual pinned parser must also accept the zero-route shape.
    let mut empty = Vec::new();
    ams_gra_codegen_oms::run(
        [
            OsString::from("service-routes"),
            OsString::from("--schema"),
            schema.clone().into(),
            OsString::from("--contract"),
            root.join("tests/fixtures/service-routes/empty.yaml").into(),
            OsString::from("--format"),
            OsString::from("sleet-toml"),
            OsString::from("--service-id"),
            OsString::from("empty"),
            OsString::from("--service-uuid"),
            OsString::from("550e8400-e29b-41d4-a716-446655440072"),
        ],
        &mut empty,
    )
    .unwrap();
    std::fs::write(dir.join("services/empty.toml"), empty).unwrap();
    let port = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let config = dir.join("server.toml");
    std::fs::write(&config, format!("bind_addr = \"127.0.0.1:{port}\"\nserver_id = \"task071\"\nsystem_label = \"Task071\"\nschema_path = \"{}\"\nschema_version = \"000.1.0\"\nservices_dir = \"{}\"\n", schema.display(), dir.join("services").display())).unwrap();
    let server = Server(
        Command::new(binary)
            .arg(config)
            .env("RUST_LOG", "warn")
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap(),
    );
    let deadline = Instant::now() + WAIT;
    let mut runtime = loop {
        match SleetRuntime::connect(
            RuntimeConfig::new(format!("ws://127.0.0.1:{port}"), "runtime-oam", "000.1.0")
                .with_connect_timeout(Duration::from_millis(500)),
            TestCodec,
        ) {
            Ok(runtime) => break runtime,
            Err(_) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(100)),
            Err(e) => panic!("server/config/INIT failed: {e}"),
        }
    };
    SleetRuntime::connect(
        RuntimeConfig::new(format!("ws://127.0.0.1:{port}"), "empty", "000.1.0")
            .with_connect_timeout(WAIT),
        TestCodec,
    )
    .expect("empty generated service config and INIT accepted")
    .close()
    .unwrap();
    let (tx, rx) = mpsc::channel();
    let subscription = oam::exchange_input_a::subscribe(&mut runtime, move |m| {
        tx.send(m.count.get()).unwrap();
    })
    .unwrap();
    oam::exchange_output_a::publish(&mut runtime, &oam_shared(71)).unwrap();
    assert_eq!(rx.recv_timeout(WAIT), Ok(71));
    let denied = runtime
        .subscribe(
            "https://www.vdl.afrl.af.mil/programs/oam",
            "MessageB",
            "loop-topic",
            None,
            |_: &oam::exchange_input_a::Payload| {},
        )
        .unwrap();
    rejection(&runtime, "MessageB");
    oam::exchange_output_b::publish(&mut runtime, &oam_shared(72)).unwrap();
    rejection(&runtime, "MessageB");
    oam::exchange_denied::publish(&mut runtime, &oam_shared(73)).unwrap();
    rejection(&runtime, "denied-topic");
    oam::exchange_output_a::publish(&mut runtime, &oam_shared(74)).unwrap();
    assert_eq!(rx.recv_timeout(WAIT), Ok(74));
    assert!(rx.recv_timeout(Duration::from_millis(300)).is_err());
    drop(denied);
    subscription.unsubscribe().unwrap();
    runtime.close().unwrap();
    drop(server);
    std::fs::remove_dir_all(dir).unwrap();
    eprintln!("TASK071 REAL SLEET POLICY: PASSED");
}
