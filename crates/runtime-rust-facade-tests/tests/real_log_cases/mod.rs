use ams_gra_oms_runtime_rust::{
    MessageDecodeError, OmsJsonCodec, RuntimeConfig, RuntimeEvent, SleetRuntime,
};
use generated::{model as m, service_api::function_log as svc, service_codec::ServiceCodec};
use serde_json::{Value, json};
use sleet_client::{CalClient, ClientError, InitOptions};
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

const WAIT: Duration = Duration::from_secs(10);
const QUIET: Duration = Duration::from_millis(200);
const TOPIC: &str = "task102-log";

fn golden() -> Value {
    serde_json::from_str(GOLDEN).expect("independently authored Task093 golden")
}

fn decode(body: &Value) -> m::LogMT {
    ServiceCodec
        .decode_payload(body)
        .expect("generated Log decoder")
}

fn inspect(value: &m::LogMT) {
    assert_eq!(
        value.securityinformation.classification,
        m::ClassificationEnum::U
    );
    assert!(matches!(
        value.securityinformation.ownerproducer.as_slice(),
        [m::OwnerProducerChoiceType::GovernmentIdentifier(m::OwnerProducerEnum::USA),
         m::OwnerProducerChoiceType::NATOSpecialWord(word)] if word.as_str() == "NATO:EXAMPLE"
    ));
    let header = &value.messageheader;
    assert_eq!(
        header.systemid.uuid.as_str(),
        "12345678-1234-1234-8234-123456789abc"
    );
    assert_eq!(header.timestamp.as_str(), "2026-01-02T03:04:05Z");
    assert_eq!(header.schemaversion.as_str(), VERSION);
    assert_eq!(header.mode, m::MessageModeEnum::LIVE);
    let data = &value.messagedata;
    assert_eq!(data.timestamp.as_str(), "2026-01-02T03:04:06Z");
    assert_eq!(data.serviceuptime.as_ref().unwrap().as_str(), "PT01.2300S");
    assert_eq!(data.severity, m::LogSeverityEnum::INFO);
    assert_eq!(data.label.as_str(), "Task093");
    assert_eq!(
        data.details.as_str(),
        "Independent real Log Binary codec qualification"
    );
}

#[test]
fn golden_and_binary_states() {
    assert_eq!(svc::exchange_receive::MESSAGE_NAME, "Log");
    assert_eq!(svc::exchange_publish::MESSAGE_NAME, "Log");
    assert_eq!(svc::exchange_receive::TOPIC, TOPIC);
    assert_eq!(svc::exchange_publish::TOPIC, TOPIC);
    let body = golden();
    let mut value = decode(&body);
    inspect(&value);
    assert_eq!(data(&value), Some(&[0x00, 0x80, 0xFF][..]));
    assert_eq!(ServiceCodec.encode_payload(&value).unwrap(), body);
    // Construct each optional state in the typed model, not via its decoder.
    for binary in [None, Some(vec![]), Some(vec![0x00, 0x80, 0xFF])] {
        set_data(&mut value, binary.clone());
        let mut expected = golden();
        match &binary {
            None => {
                expected["MessageData"]
                    .as_object_mut()
                    .unwrap()
                    .remove("Data");
            }
            Some(bytes) if bytes.is_empty() => expected["MessageData"]["Data"] = json!(""),
            Some(_) => expected["MessageData"]["Data"] = json!("0080FF"),
        }
        assert_eq!(ServiceCodec.encode_payload(&value).unwrap(), expected);
        assert_eq!(data(&decode(&expected)), binary.as_deref());
    }
    let mut lowercase = golden();
    lowercase["MessageData"]["Data"] = json!("0080ff");
    assert_eq!(
        ServiceCodec.encode_payload(&decode(&lowercase)).unwrap(),
        golden()
    );
    for invalid in [
        json!("0"),
        json!("0G"),
        json!("00 80 FF"),
        json!([0, 128, 255]),
    ] {
        let mut body = golden();
        body["MessageData"]["Data"] = invalid;
        let result: Result<m::LogMT, _> = ServiceCodec.decode_payload(&body);
        assert!(result.unwrap_err().message().contains(".Data"));
    }
    for field in ["Timestamp", "Severity", "Label", "Details"] {
        let mut body = golden();
        body["MessageData"].as_object_mut().unwrap().remove(field);
        let result: Result<m::LogMT, _> = ServiceCodec.decode_payload(&body);
        assert!(result.unwrap_err().message().contains(field));
    }
    eprintln!("RUST-OMS-001 {VERSION} GOLDEN: PASSED");
}

struct Server {
    child: Child,
    directory: PathBuf,
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_dir_all(&self.directory);
    }
}

fn start_sleet() -> (Server, String) {
    let binary = std::env::var_os("AMS_GRA_SLEET_BIN").expect("pinned Sleet binary required");
    let directory =
        std::env::temp_dir().join(format!("rust-oms-001-{VERSION}-{}", std::process::id()));
    let services = directory.join("services");
    std::fs::create_dir_all(&services).unwrap();
    std::fs::write(services.join("log.toml"),
        "service_id = \"task102-log\"\nservice_uuid = \"550e8400-e29b-41d4-a716-446655440102\"\nallowed_topics = [\"task102-log\"]\n"
    ).unwrap();
    let port = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let config = directory.join("server.toml");
    std::fs::write(&config, format!(
        "bind_addr = \"127.0.0.1:{port}\"\nserver_id = \"rust-oms-001\"\nsystem_label = \"Rust Log\"\nschema_path = \"{ROOT}\"\nschema_version = \"{VERSION}\"\nservices_dir = \"{}\"\n", services.display()
    )).unwrap();
    let child = Command::new(binary)
        .arg(config)
        .env("RUST_LOG", "warn")
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap();
    (
        Server { child, directory },
        format!("ws://127.0.0.1:{port}"),
    )
}

fn connect(url: &str) -> SleetRuntime<ServiceCodec> {
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        match SleetRuntime::connect(
            RuntimeConfig::new(url, "task102-log", VERSION).with_connect_timeout(WAIT),
            ServiceCodec,
        ) {
            Ok(runtime) => return runtime,
            Err(_) if Instant::now() < deadline => std::thread::sleep(QUIET),
            Err(error) => panic!("Sleet connect/INIT: {error}"),
        }
    }
}

async fn raw_connect(url: &str) -> CalClient {
    tokio::time::timeout(
        WAIT,
        CalClient::connect_with_options(
            url,
            "task102-log",
            InitOptions {
                schema: VERSION.to_owned(),
                ..InitOptions::default()
            },
        ),
    )
    .await
    .unwrap()
    .unwrap()
}

async fn raw_publish(
    client: &mut CalClient,
    topic: &str,
    document: &Value,
) -> Result<(), ClientError> {
    tokio::time::timeout(WAIT, client.publish(topic, document))
        .await
        .expect("PUB response timeout")
}

#[test]
fn pinned_sleet_roundtrip_and_recovery() {
    if std::env::var_os("AMS_GRA_SLEET_BIN").is_none() {
        assert!(
            std::env::var_os("AMS_GRA_REQUIRE_REAL_UCI_LOG").is_none(),
            "explicit qualification needs Sleet"
        );
        eprintln!("SKIPPED: AMS_GRA_SLEET_BIN not configured");
        return;
    }
    let (_server, url) = start_sleet();
    let mut runtime = connect(&url);
    let (tx, seen) = mpsc::channel();
    let subscription = svc::exchange_receive::subscribe(
        &mut runtime,
        move |value: &svc::exchange_receive::Payload| {
            tx.send(value.clone()).unwrap();
        },
    )
    .unwrap();
    let executor = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let mut publisher = executor.block_on(raw_connect(&url));
    let mut observer = executor.block_on(raw_connect(&url));
    executor.block_on(async {
        tokio::time::timeout(WAIT, observer.subscribe("wire", "Log", TOPIC, None))
            .await
            .unwrap()
            .unwrap();
    });
    // Observer records actual OWP MSG JSON, independently of generated decoding.
    let receive_wire = |observer: &mut CalClient| {
        executor.block_on(async {
            let message = tokio::time::timeout(WAIT, observer.recv())
                .await
                .unwrap()
                .unwrap();
            assert_eq!(message.sid, "wire");
            serde_json::from_str::<Value>(&message.payload).unwrap()
        })
    };
    let mut value = decode(&golden());
    for binary in [None, Some(vec![]), Some(vec![0x00, 0x80, 0xFF])] {
        set_data(&mut value, binary.clone());
        let mut expected = golden();
        match &binary {
            None => {
                expected["MessageData"]
                    .as_object_mut()
                    .unwrap()
                    .remove("Data");
            }
            Some(bytes) if bytes.is_empty() => expected["MessageData"]["Data"] = json!(""),
            Some(_) => expected["MessageData"]["Data"] = json!("0080FF"),
        }
        svc::exchange_publish::publish(&mut runtime, &value).unwrap();
        assert_eq!(receive_wire(&mut observer), json!({"Log": expected}));
        let received = seen.recv_timeout(WAIT).unwrap();
        inspect(&received);
        assert_eq!(data(&received), binary.as_deref());
        assert_eq!(ServiceCodec.encode_payload(&received).unwrap(), expected);
        assert!(runtime.try_recv_event().is_none());
        eprintln!("RUST-OMS-001 {VERSION} typed MSG Data={binary:?}: PASSED");
    }
    // Pinned Sleet admits malformed hex strings but rejects structural errors.
    // Assert the exact boundary, never downgrade any rejection to success.
    let mut cases = Vec::new();
    for (name, data, delivered) in [
        ("odd hex", json!("0"), true),
        ("invalid digits", json!("0G"), true),
        ("internal whitespace", json!("00 80 FF"), true),
        ("binary JSON type", json!([0, 128, 255]), false),
    ] {
        let mut body = golden();
        body["MessageData"]["Data"] = data;
        cases.push((
            name.to_owned(),
            json!({"Log": body}),
            delivered,
            "Data".to_owned(),
        ));
    }
    for field in ["Timestamp", "Severity", "Label", "Details"] {
        let mut body = golden();
        body["MessageData"].as_object_mut().unwrap().remove(field);
        cases.push((
            format!("missing {field}"),
            json!({"Log": body}),
            false,
            field.to_owned(),
        ));
    }
    for field in ["MessageHeader", "SecurityInformation", "MessageData"] {
        let mut body = golden();
        body.as_object_mut().unwrap().remove(field);
        cases.push((
            format!("missing {field}"),
            json!({"Log": body}),
            false,
            field.to_owned(),
        ));
    }
    cases.push((
        "wrong global element".to_owned(),
        json!({"LogMT": golden()}),
        false,
        "LogMT".to_owned(),
    ));
    for (name, document, delivered, diagnostic) in cases {
        let result = executor.block_on(raw_publish(&mut publisher, TOPIC, &document));
        if delivered {
            result.expect("pinned Sleet admits hex strings");
            assert_eq!(
                receive_wire(&mut observer),
                document,
                "actual malformed MSG"
            );
            match runtime
                .recv_event_timeout(WAIT)
                .expect("observable decoder rejection")
            {
                RuntimeEvent::SubscriptionDecodeError {
                    sid,
                    message_name,
                    error: MessageDecodeError::Codec(error),
                } => {
                    assert!(!sid.is_empty());
                    assert_eq!(message_name, "Log");
                    assert!(error.message().contains(&diagnostic), "{error}");
                    eprintln!(
                        "RUST-OMS-001 {VERSION} {name}: Sleet MSG -> generated decoder rejected: {error}"
                    );
                }
                other => panic!("wrong rejection boundary: {other:?}"),
            }
        } else {
            let error = result.expect_err("pinned Sleet must reject invalid structure");
            assert!(matches!(error, ClientError::Server { .. }), "{error}");
            assert!(error.to_string().contains(&diagnostic), "{error}");
            eprintln!("RUST-OMS-001 {VERSION} {name}: Sleet rejected: {error}");
            executor.block_on(async {
                assert!(
                    tokio::time::timeout(QUIET, observer.recv()).await.is_err(),
                    "no rejected MSG"
                );
            });
            assert!(runtime.recv_event_timeout(QUIET).is_none());
        }
        assert!(
            seen.recv_timeout(QUIET).is_err(),
            "invalid typed delivery: {name}"
        );
        svc::exchange_publish::publish(&mut runtime, &value).unwrap();
        assert_eq!(receive_wire(&mut observer), json!({"Log": golden()}));
        let recovered = seen
            .recv_timeout(WAIT)
            .expect("same active subscription recovers");
        inspect(&recovered);
        assert_eq!(data(&recovered), Some(&[0x00, 0x80, 0xFF][..]));
        assert_eq!(ServiceCodec.encode_payload(&recovered).unwrap(), golden());
        assert!(runtime.try_recv_event().is_none());
        eprintln!("RUST-OMS-001 {VERSION} {name}: same-subscription recovery PASSED");
    }
    for topic in ["invalid topic", "unauthorized-topic"] {
        let error = executor
            .block_on(raw_publish(
                &mut publisher,
                topic,
                &json!({"Log": golden()}),
            ))
            .unwrap_err();
        if topic == "invalid topic" {
            assert!(matches!(error, ClientError::InvalidInput(_)), "{error}");
        } else {
            assert!(matches!(error, ClientError::Server { .. }), "{error}");
        }
        assert!(error.to_string().contains(topic), "{error}");
        eprintln!("RUST-OMS-001 {VERSION} topic {topic:?}: {error}");
        assert!(seen.recv_timeout(QUIET).is_err());
    }
    let denied = executor
        .block_on(async {
            tokio::time::timeout(
                WAIT,
                publisher.subscribe("denied", "Log", "unauthorized-topic", None),
            )
            .await
            .unwrap()
        })
        .unwrap_err();
    assert!(matches!(denied, ClientError::Server { .. }), "{denied}");
    eprintln!("RUST-OMS-001 {VERSION} unauthorized SUB: {denied}");
    svc::exchange_publish::publish(&mut runtime, &value).unwrap();
    assert_eq!(receive_wire(&mut observer), json!({"Log": golden()}));
    let recovered = seen.recv_timeout(WAIT).unwrap();
    inspect(&recovered);
    assert_eq!(ServiceCodec.encode_payload(&recovered).unwrap(), golden());
    subscription
        .unsubscribe()
        .expect("generated subscription clean UNSUB");
    svc::exchange_publish::publish(&mut runtime, &value).unwrap();
    assert_eq!(receive_wire(&mut observer), json!({"Log": golden()}));
    assert!(seen.recv_timeout(QUIET).is_err(), "no callback after UNSUB");
    runtime.close().expect("runtime worker clean shutdown");
    executor.block_on(async {
        tokio::time::timeout(WAIT, observer.unsubscribe("wire"))
            .await
            .unwrap()
            .unwrap();
    });
    eprintln!("RUST-OMS-001 {VERSION} SLEET AND RECOVERY: PASSED");
}
