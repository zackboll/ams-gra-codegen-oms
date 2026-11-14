use ams_gra_oms_runtime_rust::{
    MessageDecodeError, OmsJsonCodec, RuntimeConfig, RuntimeEvent, SleetRuntime,
    unwrap_global_element,
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
const QUIET: Duration = Duration::from_millis(150);
const TOPIC: &str = "orders.metadata";
const MESSAGE: &str = "OrdersMetadata";
const QUALIFIER: &str =
    "/MessageData/OrdersAssociation/0/Orders/ATO/0/Identifier/QualifierSerialNumber";

include!("../real_sleet_support/mod.rs");

fn document(text: &str) -> Value {
    serde_json::from_str(text).expect("independently authored JSON, not a generated value")
}

fn decode(body: &Value) -> m::OrdersMetadataMT {
    ServiceCodec
        .decode_payload(body)
        .expect("authentic OrdersMetadata")
}

fn inspect(value: &m::OrdersMetadataMT, expected: &Value) {
    assert_eq!(value.messageheader.schemaversion.as_str(), VERSION);
    assert_eq!(
        value.securityinformation.classification,
        m::ClassificationEnum::U
    );
    assert_eq!(
        value.messagedata.ordersmetadataid.uuid.as_str(),
        expected["MessageData"]["OrdersMetadataID"]["UUID"]
    );
    let association = &value.messagedata.ordersassociation.as_slice()[0];
    let identifier = association.orders.ato.as_slice()[0]
        .identifier
        .as_ref()
        .unwrap();
    assert_eq!(
        identifier.qualifierserialnumber.map(|v| v.get()),
        expected.pointer(QUALIFIER).and_then(Value::as_i64)
    );
    assert_eq!(
        ServiceCodec.encode_payload(value).unwrap(),
        *expected,
        "encoding compared with independently authored JSON, not just a round trip"
    );
}

fn negatives() -> Vec<(String, Value, String, bool)> {
    document(NEGATIVE)
        .as_array()
        .unwrap()
        .iter()
        .map(|case| {
            let mut body = document(NORMAL);
            let path = case["path"].as_str().unwrap();
            if case["omit"] == true {
                let (parent, member) = path.rsplit_once('/').unwrap();
                assert!(
                    body.pointer_mut(parent)
                        .unwrap()
                        .as_object_mut()
                        .unwrap()
                        .remove(member)
                        .is_some()
                );
            } else {
                *body.pointer_mut(path).expect("authored path exists") = case["value"].clone();
            }
            (
                case["name"].as_str().unwrap().to_owned(),
                body,
                case["diagnostic"].as_str().unwrap().to_owned(),
                case["boundary"] == "decoder",
            )
        })
        .collect()
}

fn omitted_documents() -> Vec<Value> {
    let mut no_qualifier = document(NORMAL);
    no_qualifier
        .pointer_mut("/MessageData/OrdersAssociation/0/Orders/ATO/0/Identifier")
        .unwrap()
        .as_object_mut()
        .unwrap()
        .remove("QualifierSerialNumber");
    let mut no_identifier = document(NORMAL);
    no_identifier
        .pointer_mut("/MessageData/OrdersAssociation/0/Orders/ATO/0")
        .unwrap()
        .as_object_mut()
        .unwrap()
        .remove("Identifier");
    let mut no_ato = document(NORMAL);
    no_ato
        .pointer_mut("/MessageData/OrdersAssociation/0/Orders")
        .unwrap()
        .as_object_mut()
        .unwrap()
        .remove("ATO");
    vec![no_qualifier, no_identifier, no_ato]
}

#[test]
fn checked_model_and_codec() {
    for text in [NORMAL, SECOND] {
        let body = document(text);
        inspect(&decode(&body), &body);
    }
    for n in [1, 999] {
        let checked = m::USMTFSerialNumberOfQualifierType::new(n).unwrap();
        assert_eq!(checked.get(), n);
        let mut value = decode(&document(NORMAL));
        // Build the nested path from production checked values and sequences.
        let mut associations = value.messagedata.ordersassociation.as_slice().to_vec();
        let mut atos = associations[0].orders.ato.as_slice().to_vec();
        atos[0].identifier.as_mut().unwrap().qualifierserialnumber = Some(checked);
        associations[0].orders.ato = m::UnboundedVec::new(atos).unwrap();
        value.messagedata.ordersassociation = m::UnboundedVec::new(associations).unwrap();
        let mut expected = document(NORMAL);
        *expected.pointer_mut(QUALIFIER).unwrap() = json!(n);
        inspect(&value, &expected);
    }
    for n in [0, 1000, -1] {
        assert!(m::USMTFSerialNumberOfQualifierType::new(n).is_none());
    }
    for (name, body, _, _) in negatives() {
        let result: Result<m::OrdersMetadataMT, _> = ServiceCodec.decode_payload(&body);
        let error = result.expect_err(&name);
        eprintln!("RUST-OMS-012 {VERSION} {name}: local codec rejected: {error}");
        if name.starts_with("qualifier-") {
            assert!(error.message().contains("QualifierSerialNumber"));
        }
    }
    for body in omitted_documents() {
        let value = decode(&body);
        assert_eq!(ServiceCodec.encode_payload(&value).unwrap(), body);
    }
    for wire in [
        json!({"OrdersMetadataMT": document(NORMAL)}),
        json!({}),
        json!({MESSAGE: document(NORMAL), "Log": {}}),
        json!([]),
    ] {
        assert!(unwrap_global_element(MESSAGE, &wire.to_string()).is_err());
    }
    assert_eq!(
        unwrap_global_element(MESSAGE, &json!({MESSAGE: document(NORMAL)}).to_string()).unwrap(),
        document(NORMAL)
    );
    assert_eq!(svc::exchange_e::MESSAGE_NAME, MESSAGE);
    assert_eq!(svc::exchange_publish::MESSAGE_NAME, MESSAGE);
    assert_eq!(svc::exchange_e::TOPIC, TOPIC);
    assert_eq!(svc::exchange_publish::TOPIC, TOPIC);
    assert_eq!(svc::exchange_e::SUBSCRIPTION_GROUP, None);
    // SchemaVersion is a lexical type, not a release-specific singleton.
    let mut other_version = document(NORMAL);
    other_version["MessageHeader"]["SchemaVersion"] = json!(if VERSION == "002.5.0" {
        "002.6.0"
    } else {
        "002.5.0"
    });
    let value = decode(&other_version);
    assert_eq!(ServiceCodec.encode_payload(&value).unwrap(), other_version);
    eprintln!("RUST-OMS-012 {VERSION} MODEL CODEC: PASSED");
}

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
fn pinned_sleet_interoperability() {
    if std::env::var_os("AMS_GRA_SLEET_BIN").is_none() {
        assert!(
            std::env::var_os("AMS_GRA_REQUIRE_REAL_UCI_ORDERS_METADATA").is_none(),
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
        svc::exchange_e::subscribe(&mut runtime, move |value: &m::OrdersMetadataMT| {
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
        .expect_err("broker rejects incorrect INIT version");
    assert!(matches!(error, ClientError::Server { .. }), "{error}");
    assert!(error.to_string().contains("does not match"), "{error}");
    eprintln!("RUST-OMS-012 {VERSION} incorrect INIT schema: broker rejected connection: {error}");
    executor
        .block_on(observer.subscribe("wire", MESSAGE, TOPIC, None))
        .unwrap();
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
    // A successful PUB write alone proves nothing. Require independent MSG
    // delivery and a checked generated typed callback for the authored body.
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
    eprintln!("RUST-OMS-012 {VERSION} TYPED PUBLISH AND INDEPENDENT RECEIVE: PASSED");
    for body in omitted_documents() {
        executor
            .block_on(raw_publish(&mut publisher, TOPIC, &json!({MESSAGE: body})))
            .unwrap();
        assert_eq!(receive_wire(&mut observer), json!({MESSAGE: body}));
        assert_eq!(
            ServiceCodec
                .encode_payload(&seen.recv_timeout(WAIT).unwrap().1)
                .unwrap(),
            body
        );
    }
    for (name, body, diagnostic, decoder) in negatives() {
        let wire = json!({MESSAGE: body});
        let result = executor.block_on(raw_publish(&mut publisher, TOPIC, &wire));
        if decoder {
            result.expect("broker admission is not generated decoder acceptance");
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
                    assert!(error.message().contains(&diagnostic), "{name}: {error}");
                    eprintln!(
                        "RUST-OMS-012 {VERSION} {name}: routed MSG; generated decoder rejected: {error}"
                    );
                }
                other => panic!("wrong rejection boundary: {other:?}"),
            }
        } else {
            let error = result.expect_err("broker rejects before routing");
            assert!(
                matches!(error, ClientError::Server { .. }),
                "{name}: {error}"
            );
            assert!(error.to_string().contains(&diagnostic), "{name}: {error}");
            executor.block_on(async {
                assert!(tokio::time::timeout(QUIET, observer.recv()).await.is_err());
            });
            assert!(runtime.recv_event_timeout(QUIET).is_none());
            eprintln!("RUST-OMS-012 {VERSION} {name}: broker rejected: {error}");
        }
        assert!(
            seen.recv_timeout(QUIET).is_err(),
            "invalid input never becomes a typed callback"
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
            json!({"OrdersMetadataMT": normal}),
            "UnknownGlobalElement",
        ),
        (
            "unauthorized-topic",
            "forbidden",
            json!({MESSAGE: normal}),
            "not authorized",
        ),
    ] {
        assert!(
            unwrap_global_element(MESSAGE, &wire.to_string()).is_err()
                || name == "unauthorized-topic"
        );
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
        eprintln!("RUST-OMS-012 {VERSION} {name}: broker rejected: {error}");
    }
    let (tx, witness) = mpsc::channel();
    let remaining = svc::exchange_e::subscribe(&mut runtime, move |value: &m::OrdersMetadataMT| {
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
        "original handler removed while runtime remains live"
    );
    remaining.unsubscribe().unwrap();
    let (done_tx, done) = mpsc::channel();
    let closing = std::thread::spawn(move || {
        done_tx.send(runtime.close()).unwrap();
    });
    assert_eq!(exit.entered.recv_timeout(WAIT).unwrap(), worker);
    crate::common::worker_join::assert_still_blocked(&done, "OrdersMetadata worker TLS teardown");
    exit.release.unwrap().send(()).unwrap();
    assert_eq!(exit.finished.recv_timeout(WAIT).unwrap(), worker);
    done.recv_timeout(WAIT).unwrap().unwrap();
    closing.join().unwrap();
    executor.block_on(observer.unsubscribe("wire")).unwrap();
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
    eprintln!("RUST-OMS-012 {VERSION} SUBSCRIPTION REMOVAL AND CLEANUP: PASSED");
    eprintln!("RUST-OMS-012 {VERSION} PINNED SLEET: PASSED");
}
