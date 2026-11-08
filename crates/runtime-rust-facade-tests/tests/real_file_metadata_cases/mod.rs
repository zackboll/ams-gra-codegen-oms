use ams_gra_oms_runtime_rust::{
    MessageDecodeError, OmsJsonCodec, RuntimeConfig, RuntimeEvent, SleetRuntime,
};
use generated::{model as m, service_api::function_f as svc, service_codec::ServiceCodec};
use serde_json::{Value, json};
use sleet_client::{CalClient, ClientError, InitOptions};
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

const WAIT: Duration = Duration::from_secs(10);
const QUIET: Duration = Duration::from_millis(200);
const TOPIC: &str = "file.metadata";
const MESSAGE: &str = "FileMetadata";
const BYTES: [u8; 32] = [
    0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F,
    0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1A, 0x1B, 0x1C, 0x1D, 0x1E, 0x1F,
];
const HEX: &str = "000102030405060708090A0B0C0D0E0F101112131415161718191A1B1C1D1E1F";

include!("../real_sleet_support/mod.rs");

fn document(text: &str) -> Value {
    serde_json::from_str(text).expect("independently authored OMS JSON")
}

fn decode(body: &Value) -> m::FileMetadataMT {
    ServiceCodec
        .decode_payload(body)
        .expect("authentic FileMetadata")
}

fn inspect(value: &m::FileMetadataMT, expected: &Value) {
    assert_eq!(value.messageheader.schemaversion.as_str(), VERSION);
    assert_eq!(
        value.securityinformation.classification,
        m::ClassificationEnum::U
    );
    let data = &value.messagedata;
    assert_eq!(data.filename.as_str(), expected["MessageData"]["FileName"]);
    assert_eq!(
        data.untrustedmodification,
        expected["MessageData"]["UntrustedModification"]
            .as_bool()
            .unwrap()
    );
    assert_eq!(
        data.timestamp.as_str(),
        expected["MessageData"]["Timestamp"]
    );
    assert_eq!(data.creationsource, m::CreationSourceEnum::OTHER);
    assert_eq!(
        data.securityinformation.classification,
        m::ClassificationEnum::U
    );
    assert_eq!(
        data.filemetadataid.uuid.as_str(),
        expected["MessageData"]["FileMetadataID"]["UUID"]
    );
    assert_eq!(
        data.filedescription.filetype.as_slice(),
        [m::FileTypeEnum::OTHER]
    );
    match &data.filedescription.fileformat {
        m::FileFormatType::NonMIME(key) => {
            assert_eq!(key.key.as_str(), "META");
            assert_eq!(key.systemname.as_str(), "Independent");
        }
        other => panic!("expected authored NonMIME description: {other:?}"),
    }
    assert_eq!(data.filesource.systemid.as_slice().len(), 1);
    assert_eq!(
        data.filesource.systemid.as_slice()[0].uuid.as_str(),
        "550e8400-e29b-41d4-a716-446655440030"
    );
    assert!(data.filesource.subsystemid.as_slice().is_empty());
    assert!(data.filesource.serviceid.as_slice().is_empty());
    assert!(data.filesource.capabilityid.as_slice().is_empty());
    if expected["MessageData"].get("SHA_2_Hash").is_some() {
        assert_eq!(data.sha_2_hash.as_ref().unwrap().as_slice(), BYTES);
        assert_eq!(expected["MessageData"]["SHA_2_Hash"], HEX);
    } else {
        assert!(data.sha_2_hash.is_none());
    }
    // Compare all remaining metadata (IDs, description, source, markings and
    // release-specific FileSize) with authored input, not a codec roundtrip.
    assert_eq!(ServiceCodec.encode_payload(value).unwrap(), *expected);
}

fn negatives() -> Vec<(String, Value, String, bool)> {
    document(NEGATIVE)
        .as_array()
        .unwrap()
        .iter()
        .map(|case| {
            let mut body = document(NORMAL);
            body["MessageData"][case["field"].as_str().unwrap()] = case["value"].clone();
            (
                case["name"].as_str().unwrap().to_owned(),
                body,
                case["diagnostic"].as_str().unwrap().to_owned(),
                case["boundary"] == "decoder",
            )
        })
        .collect()
}

/// Observe live TCP connections before stopping the broker, so killing the
/// server cannot conceal a leaked runtime/client connection. TIME_WAIT is not
/// a live connection or worker leak. Qualification runs on Linux.
#[cfg(target_os = "linux")]
fn active_connections(url: &str) -> usize {
    let port: u16 = url.rsplit(':').next().unwrap().parse().unwrap();
    let suffix = format!(":{port:04X}");
    std::fs::read_to_string("/proc/net/tcp")
        .unwrap()
        .lines()
        .skip(1)
        .filter(|line| {
            let columns: Vec<_> = line.split_whitespace().collect();
            columns[3] == "01" && (columns[1].ends_with(&suffix) || columns[2].ends_with(&suffix))
        })
        .count()
}

#[test]
fn checked_model_and_codec() {
    for text in [NORMAL, SECOND, HASH] {
        let body = document(text);
        inspect(&decode(&body), &body);
    }
    for name in [
        "Report_01.txt".to_owned(),
        "archive.tar.bz2".to_owned(),
        format!("{}.txt", "A".repeat(251)),
    ] {
        assert_eq!(m::FileNameType::new(&name).unwrap().as_str(), name);
        let mut body = document(NORMAL);
        body["MessageData"]["FileName"] = json!(name);
        inspect(&decode(&body), &body);
    }
    for (name, body, _, _) in negatives() {
        let result: Result<m::FileMetadataMT, _> = ServiceCodec.decode_payload(&body);
        let error = result.expect_err(&name);
        let member = if name.contains("hash") || name.contains("hex") {
            "SHA_2_Hash"
        } else {
            "FileName"
        };
        assert!(error.message().contains(member), "{name}: {error}");
        if member == "FileName"
            && let Some(text) = body["MessageData"][member].as_str()
        {
            assert!(m::FileNameType::new(text).is_none(), "{name}");
        }
    }
    for member in [
        "FileName",
        "FileMetadataID",
        "FileDescription",
        "FileSource",
        "CreationSource",
        "UntrustedModification",
        "Timestamp",
        "SecurityInformation",
    ] {
        let mut body = document(NORMAL);
        body["MessageData"].as_object_mut().unwrap().remove(member);
        let result: Result<m::FileMetadataMT, _> = ServiceCodec.decode_payload(&body);
        assert!(result.unwrap_err().message().contains(member));
    }
    assert_eq!(
        m::SHA2256HashType::new(BYTES.to_vec()).unwrap().as_slice(),
        BYTES
    );
    for length in [0, 16, 31, 33, 64] {
        assert!(m::SHA2256HashType::new(vec![0; length]).is_none());
    }
    let mut lowercase = document(HASH);
    lowercase["MessageData"]["SHA_2_Hash"] = json!(HEX.to_ascii_lowercase());
    inspect(&decode(&lowercase), &document(HASH));
    assert_eq!(svc::exchange_e::MESSAGE_NAME, MESSAGE);
    assert_eq!(svc::exchange_publish::MESSAGE_NAME, MESSAGE);
    eprintln!("RUST-OMS-010 {VERSION} MODEL CODEC: PASSED");
}

#[test]
fn pinned_sleet_interoperability() {
    if std::env::var_os("AMS_GRA_SLEET_BIN").is_none() {
        assert!(
            std::env::var_os("AMS_GRA_REQUIRE_REAL_UCI_FILE_METADATA").is_none(),
            "explicit qualification needs pinned Sleet"
        );
        eprintln!("SKIPPED: pinned Sleet not configured");
        return;
    }
    let (mut server, url) = start_sleet();
    let mut runtime = connect(&url);
    let (mut worker_exit, exit) = crate::common::worker_join::WorkerExit::new(true);
    let (tx, seen) = mpsc::channel();
    let subscription =
        svc::exchange_e::subscribe(&mut runtime, move |value: &m::FileMetadataMT| {
            let worker = worker_exit.arm();
            tx.send((worker, value.clone())).unwrap();
        })
        .unwrap();
    let executor = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let mut publisher = executor.block_on(raw_connect(&url));
    let mut observer = executor.block_on(raw_connect(&url));
    #[cfg(target_os = "linux")]
    assert!(active_connections(&url) > 0, "live connection witness");
    let wrong_version = if VERSION == "002.5.0" {
        "002.6.0"
    } else {
        "002.5.0"
    };
    let error = executor
        .block_on(async {
            tokio::time::timeout(
                WAIT,
                CalClient::connect_with_options(
                    &url,
                    TOPIC,
                    InitOptions {
                        schema: wrong_version.to_owned(),
                        ..InitOptions::default()
                    },
                ),
            )
            .await
            .unwrap()
        })
        .err()
        .expect("broker rejects incorrect INIT schema version");
    assert!(matches!(error, ClientError::Server { .. }), "{error}");
    assert!(error.to_string().contains("does not match"), "{error}");
    eprintln!("RUST-OMS-010 {VERSION} incorrect INIT schema: broker rejected connection: {error}");
    executor.block_on(async {
        observer
            .subscribe("wire", MESSAGE, TOPIC, None)
            .await
            .unwrap();
    });
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
    let normal = document(NORMAL);
    svc::exchange_publish::publish(&mut runtime, &decode(&normal)).unwrap();
    assert_eq!(receive_wire(&mut observer), json!({MESSAGE: normal}));
    let (worker, first) = seen.recv_timeout(WAIT).unwrap();
    inspect(&first, &normal);
    let second = document(SECOND);
    executor
        .block_on(raw_publish(
            &mut publisher,
            TOPIC,
            &json!({MESSAGE: second}),
        ))
        .unwrap();
    assert_eq!(receive_wire(&mut observer), json!({MESSAGE: second}));
    inspect(&seen.recv_timeout(WAIT).unwrap().1, &second);
    let mut maximum = normal.clone();
    maximum["MessageData"]["FileName"] = json!(format!("{}.txt", "A".repeat(251)));
    svc::exchange_publish::publish(&mut runtime, &decode(&maximum)).unwrap();
    assert_eq!(receive_wire(&mut observer), json!({MESSAGE: maximum}));
    inspect(&seen.recv_timeout(WAIT).unwrap().1, &maximum);

    // Sleet's existing hexBinary length-facet defect counts lexical characters,
    // not decoded bytes. A valid 32-byte hash is locally accepted, broker rejected.
    let hashed = document(HASH);
    let mut checked_hash = decode(&hashed);
    checked_hash.messagedata.sha_2_hash = Some(m::SHA2256HashType::new(BYTES.to_vec()).unwrap());
    // Production runtime is deliberately non-verbose: a successful write is
    // not broker acceptance. Rejection arrives asynchronously as ServerError.
    svc::exchange_publish::publish(&mut runtime, &checked_hash).unwrap();
    let error = match runtime
        .recv_event_timeout(WAIT)
        .expect("asynchronous broker rejection")
    {
        RuntimeEvent::ServerError {
            details: Some(details),
            ..
        } => details,
        other => panic!("wrong boundary for valid hash: {other:?}"),
    };
    assert!(error.contains("FacetViolation"), "{error}");
    executor.block_on(async {
        assert!(tokio::time::timeout(QUIET, observer.recv()).await.is_err());
    });
    assert!(seen.recv_timeout(QUIET).is_err());
    assert!(runtime.recv_event_timeout(QUIET).is_none());
    eprintln!("RUST-OMS-010 {VERSION} valid 32-byte hash: broker rejected before routing: {error}");
    svc::exchange_publish::publish(&mut runtime, &decode(&normal)).unwrap();
    assert_eq!(receive_wire(&mut observer), json!({MESSAGE: normal}));
    inspect(&seen.recv_timeout(WAIT).unwrap().1, &normal);
    assert!(runtime.try_recv_event().is_none());

    let mut cases = negatives();
    for member in [
        "FileName",
        "FileMetadataID",
        "FileDescription",
        "FileSource",
        "CreationSource",
        "UntrustedModification",
        "Timestamp",
        "SecurityInformation",
    ] {
        let mut body = normal.clone();
        body["MessageData"].as_object_mut().unwrap().remove(member);
        cases.push((
            format!("missing-{member}"),
            body,
            "Missing".to_owned(),
            false,
        ));
    }
    for (name, body, diagnostic, decoder) in cases {
        let wire = json!({MESSAGE: body});
        let result = executor.block_on(raw_publish(&mut publisher, TOPIC, &wire));
        if decoder {
            result.expect("pinned broker admits lexical length 32");
            assert_eq!(receive_wire(&mut observer), wire);
            match runtime
                .recv_event_timeout(WAIT)
                .expect("subscription rejection")
            {
                RuntimeEvent::SubscriptionDecodeError {
                    sid,
                    message_name,
                    error: MessageDecodeError::Codec(error),
                } => {
                    assert_eq!(sid, subscription.id());
                    assert_eq!(message_name, MESSAGE);
                    assert!(error.message().contains(&diagnostic), "{error}");
                    eprintln!(
                        "RUST-OMS-010 {VERSION} {name}: routed MSG; generated decoder rejected: {error}"
                    );
                }
                other => panic!("wrong error boundary: {other:?}"),
            }
        } else {
            let error = result.expect_err("broker rejects before routing");
            assert!(matches!(error, ClientError::Server { .. }), "{error}");
            assert!(error.to_string().contains(&diagnostic), "{name}: {error}");
            executor.block_on(async {
                assert!(tokio::time::timeout(QUIET, observer.recv()).await.is_err());
            });
            assert!(runtime.recv_event_timeout(QUIET).is_none());
            eprintln!("RUST-OMS-010 {VERSION} {name}: broker rejected: {error}");
        }
        assert!(
            seen.recv_timeout(QUIET).is_err(),
            "no invalid typed callback"
        );
        svc::exchange_publish::publish(&mut runtime, &decode(&normal)).unwrap();
        assert_eq!(receive_wire(&mut observer), json!({MESSAGE: normal}));
        inspect(&seen.recv_timeout(WAIT).unwrap().1, &normal);
        assert!(runtime.is_running());
        assert!(runtime.try_recv_event().is_none());
    }
    for (name, topic, wire, diagnostic) in [
        (
            "wrong-global",
            TOPIC,
            json!({"FileMetadataMT": normal}),
            "UnknownGlobalElement",
        ),
        (
            "unauthorized-topic",
            "forbidden",
            json!({MESSAGE: normal}),
            "not authorized",
        ),
    ] {
        let error = executor
            .block_on(raw_publish(&mut publisher, topic, &wire))
            .unwrap_err();
        assert!(matches!(error, ClientError::Server { .. }));
        assert!(error.to_string().contains(diagnostic), "{name}: {error}");
        executor.block_on(async {
            assert!(tokio::time::timeout(QUIET, observer.recv()).await.is_err());
        });
        assert!(seen.recv_timeout(QUIET).is_err());
        assert!(runtime.recv_event_timeout(QUIET).is_none());
        eprintln!("RUST-OMS-010 {VERSION} {name}: broker rejected: {error}");
        svc::exchange_publish::publish(&mut runtime, &decode(&second)).unwrap();
        assert_eq!(receive_wire(&mut observer), json!({MESSAGE: second}));
        inspect(&seen.recv_timeout(WAIT).unwrap().1, &second);
    }
    let (tx, witness) = mpsc::channel();
    let remaining = svc::exchange_e::subscribe(&mut runtime, move |value: &m::FileMetadataMT| {
        tx.send(value.clone()).unwrap();
    })
    .unwrap();
    subscription.unsubscribe().unwrap();
    executor
        .block_on(raw_publish(
            &mut publisher,
            TOPIC,
            &json!({MESSAGE: second}),
        ))
        .unwrap();
    assert_eq!(receive_wire(&mut observer), json!({MESSAGE: second}));
    inspect(&witness.recv_timeout(WAIT).unwrap(), &second);
    assert!(
        seen.recv_timeout(QUIET).is_err(),
        "original callback removed, connection still live"
    );
    remaining.unsubscribe().unwrap();
    // Reuse the established TLS teardown barrier to prove close actually joins
    // this exact live worker, not merely that it eventually exits.
    let (done_tx, done) = mpsc::channel();
    let closing = std::thread::spawn(move || {
        done_tx.send(runtime.close()).unwrap();
    });
    assert_eq!(exit.entered.recv_timeout(WAIT).unwrap(), worker);
    crate::common::worker_join::assert_still_blocked(&done, "FileMetadata worker TLS teardown");
    exit.release.unwrap().send(()).unwrap();
    assert_eq!(exit.finished.recv_timeout(WAIT).unwrap(), worker);
    done.recv_timeout(WAIT).unwrap().unwrap();
    closing.join().unwrap();
    executor.block_on(async {
        observer.unsubscribe("wire").await.unwrap();
    });
    drop(observer);
    drop(publisher);
    drop(executor);
    #[cfg(target_os = "linux")]
    {
        let deadline = Instant::now() + WAIT;
        while active_connections(&url) != 0 {
            assert!(
                Instant::now() < deadline,
                "connection leak before broker shutdown"
            );
            std::thread::sleep(QUIET);
        }
    }
    server.shutdown();
    assert!(
        TcpListener::bind(url.strip_prefix("ws://").unwrap()).is_ok(),
        "broker listener released"
    );
    eprintln!("RUST-OMS-010 {VERSION} SUBSCRIPTION REMOVAL AND CLEANUP: PASSED");
    eprintln!("RUST-OMS-010 {VERSION} PINNED SLEET: PASSED");
}
