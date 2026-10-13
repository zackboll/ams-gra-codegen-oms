//! Focused client-policy evidence against the unmodified pinned broker.
mod codec_sample;
use ams_gra_oms_runtime_rust::{
    CodecError, DuplicateMemberPolicy, MessageDecodeError, OmsJsonCodec, RuntimeConfig,
    RuntimeEvent, SleetRuntime,
};
use ams_gra_oms_runtime_rust_facade_tests::codec_oam::{
    model::CodecPayload, service_api::function_loop as svc, service_codec::ServiceCodec,
};
use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use std::{
    net::TcpListener,
    path::Path,
    process::{Child, Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};
use tokio_tungstenite::{
    connect_async,
    tungstenite::{Message, client::IntoClientRequest},
};

const WAIT: Duration = Duration::from_secs(10);
struct Server(Child);
impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
struct Counted(Arc<AtomicUsize>);
impl OmsJsonCodec<CodecPayload> for Counted {
    fn encode_payload(&self, v: &CodecPayload) -> Result<Value, CodecError> {
        ServiceCodec.encode_payload(v)
    }
    fn decode_payload(&self, v: &Value) -> Result<CodecPayload, CodecError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        ServiceCodec.decode_payload(v)
    }
}

#[test]
#[ignore = "requires AMS_GRA_SLEET_BIN; real-Sleet wrapper requires exact execution"]
fn task077_real_sleet_client_policy() {
    let binary =
        std::env::var_os("AMS_GRA_SLEET_BIN").expect("Task077 requires pinned Sleet binary");
    assert!(Path::new(&binary).is_file(), "missing pinned Sleet binary");
    let dir = std::env::temp_dir().join(format!("ams-task077-sleet-{}", std::process::id()));
    let services = dir.join("services");
    std::fs::create_dir_all(&services).unwrap();
    std::fs::write(services.join("codec.toml"), "service_id = \"codec-oam\"\nservice_uuid = \"550e8400-e29b-41d4-a716-446655440077\"\nallowed_topics = [\"codec-topic\"]\n").unwrap();
    let port = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let schema = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/service-generate/codec-oam.xsd")
        .canonicalize()
        .unwrap();
    let config = dir.join("server.toml");
    std::fs::write(&config, format!("bind_addr = \"127.0.0.1:{port}\"\nserver_id = \"task077\"\nsystem_label = \"Task077\"\nschema_path = \"{}\"\nschema_version = \"000.1.0\"\nservices_dir = \"{}\"\n", schema.display(), services.display())).unwrap();
    let server = Server(
        Command::new(binary)
            .arg(config)
            .env("RUST_LOG", "warn")
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("start pinned Sleet"),
    );
    let url = format!("ws://127.0.0.1:{port}");
    let legacy_count = Arc::new(AtomicUsize::new(0));
    let strict_count = Arc::new(AtomicUsize::new(0));
    let deadline = Instant::now() + WAIT;
    let mut legacy = loop {
        match SleetRuntime::connect(
            RuntimeConfig::new(&url, "codec-oam", "000.1.0"),
            Counted(legacy_count.clone()),
        ) {
            Ok(r) => break r,
            Err(_) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(25)),
            Err(e) => panic!("connect: {e}"),
        }
    };
    let (ltx, lrx) = mpsc::channel();
    let lsub = svc::exchange_input_notice::subscribe(&mut legacy, move |v: &CodecPayload| {
        ltx.send(v.level.get()).unwrap();
    })
    .unwrap();
    let value = codec_sample::sample();
    // Same-connection SUB then PUB proves legacy readiness without a sleep.
    svc::exchange_output_notice::publish(&mut legacy, &value).unwrap();
    assert_eq!(lrx.recv_timeout(WAIT).unwrap(), -10);
    let mut strict = SleetRuntime::connect(
        RuntimeConfig::new(&url, "codec-oam", "000.1.0")
            .with_duplicate_member_policy(DuplicateMemberPolicy::Reject),
        Counted(strict_count.clone()),
    )
    .unwrap();
    let handlers = Arc::new(AtomicUsize::new(0));
    let h = handlers.clone();
    let (stx, srx) = mpsc::channel();
    let ssub = svc::exchange_input_notice::subscribe(&mut strict, move |v: &CodecPayload| {
        h.fetch_add(1, Ordering::SeqCst);
        stx.send(v.level.get()).unwrap();
    })
    .unwrap();
    // Same-connection ordering proves strict readiness; both receive this control.
    svc::exchange_output_notice::publish(&mut strict, &value).unwrap();
    assert_eq!(srx.recv_timeout(WAIT).unwrap(), -10);
    assert_eq!(lrx.recv_timeout(WAIT).unwrap(), -10);
    let executor = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let mut ws = executor.block_on(async {
        let mut request = url.as_str().into_client_request().unwrap();
        request.headers_mut().insert("Sec-WebSocket-Protocol", "owp".parse().unwrap());
        let (mut ws, _) = tokio::time::timeout(WAIT, connect_async(request)).await.unwrap().unwrap();
        ws.send(Message::Text(r#"INIT {"versions":["1.0"],"schema":"000.1.0","service_id":"codec-oam","verbose":false}"#.into())).await.unwrap();
        let info = tokio::time::timeout(WAIT, ws.next()).await.unwrap().unwrap().unwrap();
        assert!(info.to_text().unwrap().starts_with("INFO "));
        ws
    });
    let body = serde_json::to_string(&ServiceCodec.encode_payload(&value).unwrap()).unwrap();
    assert!(body.contains("\"Level\":-10"));
    for duplicate_body in [
        body.replacen("\"Level\":-10", "\"Level\":9,\"Level\":-10", 1),
        body.replacen(
            "\"$type\":\"BoxShape\"",
            "\"$type\":\"CircleShape\",\"\\u0024type\":\"BoxShape\"",
            1,
        ),
    ] {
        assert_ne!(duplicate_body, body);
        let before = strict_count.load(Ordering::SeqCst);
        let handled = handlers.load(Ordering::SeqCst);
        let old = legacy_count.load(Ordering::SeqCst);
        // No Value round-trip: raw duplicate bytes are sent intact.
        executor.block_on(async {
            tokio::time::timeout(
                WAIT,
                ws.send(Message::Text(
                    format!("PUB codec-topic {{\"CodecNotice\":{duplicate_body}}}").into(),
                )),
            )
            .await
            .unwrap()
            .unwrap();
        });
        assert_eq!(lrx.recv_timeout(WAIT).unwrap(), -10);
        match strict.recv_event_timeout(WAIT) {
            Some(RuntimeEvent::SubscriptionDecodeError {
                sid,
                message_name,
                error: MessageDecodeError::DuplicateMember,
            }) => {
                assert_eq!(sid, ssub.id());
                assert_eq!(message_name, "CodecNotice");
            }
            e => panic!("expected duplicate rejection: {e:?}"),
        }
        assert_eq!(strict_count.load(Ordering::SeqCst), before);
        assert_eq!(handlers.load(Ordering::SeqCst), handled);
        assert_eq!(legacy_count.load(Ordering::SeqCst), old + 1);
        assert!(srx.try_recv().is_err());
        svc::exchange_output_notice::publish(&mut strict, &value).unwrap();
        assert_eq!(srx.recv_timeout(WAIT).unwrap(), -10);
        assert_eq!(lrx.recv_timeout(WAIT).unwrap(), -10);
    }
    assert!(legacy.try_recv_event().is_none());
    assert!(strict.try_recv_event().is_none());
    lsub.unsubscribe().unwrap();
    ssub.unsubscribe().unwrap();
    legacy.close().unwrap();
    strict.close().unwrap();
    executor.block_on(async {
        tokio::time::timeout(WAIT, ws.close(None))
            .await
            .unwrap()
            .unwrap();
    });
    drop(server);
    std::fs::remove_dir_all(dir).unwrap();
    println!("TASK077 REAL SLEET CLIENT POLICY: PASSED");
}
