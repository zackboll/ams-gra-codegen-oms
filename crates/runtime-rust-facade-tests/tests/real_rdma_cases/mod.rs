use ams_gra_oms_runtime_rust::{
    MessageDecodeError, OmsJsonCodec, RuntimeConfig, RuntimeEvent, SleetRuntime,
};
use generated::{model as m, service_api::function_rdma as svc, service_codec::ServiceCodec};
use serde_json::{Value, json};
use sleet_client::{CalClient, ClientError, InitOptions};
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

const WAIT: Duration = Duration::from_secs(10);
const QUIET: Duration = Duration::from_millis(200);
const TOPIC: &str = "task103-rdma";
const MESSAGE: &str = "RDMA_InitializeSetup";
const UUID: &str = "12345678-1234-1234-8234-123456789abc";

include!("../real_sleet_support/mod.rs");

fn golden() -> Value {
    serde_json::from_str(GOLDEN).expect("unchanged independent Task092 golden")
}

fn decode(body: &Value) -> m::RDMAInitializeSetupMT {
    ServiceCodec
        .decode_payload(body)
        .expect("generated RDMA decoder")
}

/// Independently authored JSON, never obtained from a generated encoder.
fn positives() -> Vec<(&'static str, Value)> {
    let mut cases = vec![("golden", golden())];
    for (name, endpoints) in [
        (
            "ipv4",
            json!([{"IPv4_Endpoint":{"IPv4_Address":"198.51.100.42"}}]),
        ),
        (
            "ipv6",
            json!([{"IPv6_Endpoint":{"IPv6_Address":"2001:db8::42"}}]),
        ),
        (
            "reversed",
            json!([
                {"IPv6_Endpoint":{"IPv6_Address":"2001:db8::1"}},
                {"IPv4_Endpoint":{"IPv4_Address":"192.0.2.10"}}
            ]),
        ),
        (
            "alternating",
            json!([
                {"IPv4_Endpoint":{"IPv4_Address":"198.51.100.42"}},
                {"IPv6_Endpoint":{"IPv6_Address":"2001:db8::42"}},
                {"IPv4_Endpoint":{"IPv4_Address":"192.0.2.10"}},
                {"IPv6_Endpoint":{"IPv6_Address":"2001:db8::1"}}
            ]),
        ),
        ("empty", json!([])),
    ] {
        let mut body = golden();
        body["MessageData"]["Endpoint"] = endpoints;
        if name == "alternating" {
            body["SecurityInformation"]["OwnerProducer"] = json!([
                {"GovernmentIdentifier":"USA"}, {"NATO_SpecialWord":"NATO:EXAMPLE"},
                {"GovernmentIdentifier":"CAN"}, {"NATO_SpecialWord":"NATO:EXAMPLE"}
            ]);
        }
        cases.push((name, body));
    }
    cases
}

fn canonical(body: &Value) -> Value {
    let mut body = body.clone();
    if body["MessageData"]["Endpoint"] == json!([]) {
        body["MessageData"]
            .as_object_mut()
            .unwrap()
            .remove("Endpoint");
    }
    body
}

/// Inspect real model fields and Choice variants, not just codec round trips.
fn inspect(value: &m::RDMAInitializeSetupMT, expected: &Value) {
    assert_eq!(
        value.securityinformation.classification,
        m::ClassificationEnum::U
    );
    let owners = value.securityinformation.ownerproducer.as_slice();
    let expected_owners = expected["SecurityInformation"]["OwnerProducer"]
        .as_array()
        .unwrap();
    assert_eq!(owners.len(), expected_owners.len());
    for (owner, expected) in owners.iter().zip(expected_owners) {
        match owner {
            m::OwnerProducerChoiceType::GovernmentIdentifier(id) => {
                let id = match id {
                    m::OwnerProducerEnum::USA => "USA",
                    m::OwnerProducerEnum::CAN => "CAN",
                    other => panic!("unexpected government {other:?}"),
                };
                assert_eq!(expected, &json!({"GovernmentIdentifier":id}));
            }
            m::OwnerProducerChoiceType::NATOSpecialWord(word) => {
                assert_eq!(word.as_str(), "NATO:EXAMPLE");
                assert_eq!(expected, &json!({"NATO_SpecialWord":"NATO:EXAMPLE"}));
            }
        }
    }
    assert_eq!(value.messageheader.systemid.uuid.as_str(), UUID);
    assert_eq!(
        value.messageheader.timestamp.as_str(),
        "2026-01-02T03:04:05Z"
    );
    assert_eq!(value.messageheader.schemaversion.as_str(), VERSION);
    assert_eq!(value.messageheader.mode, m::MessageModeEnum::LIVE);
    let data = &value.messagedata;
    assert_eq!(data.rdma_initializesetupid.uuid.as_str(), UUID);
    assert_eq!(data.rdma_provider, m::RDMAProviderEnum::TCP);
    let endpoints = data.endpoint.as_slice();
    let expected_endpoints = expected["MessageData"]
        .get("Endpoint")
        .map(|v| v.as_array().unwrap().as_slice())
        .unwrap_or(&[]);
    assert_eq!(endpoints.len(), expected_endpoints.len());
    for (endpoint, expected) in endpoints.iter().zip(expected_endpoints) {
        match endpoint {
            m::NetworkEndpointType::IPv4Endpoint(ip) => {
                assert_eq!(
                    Some(ip.ipv4_address.as_str()),
                    expected["IPv4_Endpoint"]["IPv4_Address"].as_str()
                );
                assert!(ip.prefix.is_none() && ip.description.is_none());
            }
            m::NetworkEndpointType::IPv6Endpoint(ip) => {
                assert_eq!(
                    Some(ip.ipv6_address.as_str()),
                    expected["IPv6_Endpoint"]["IPv6_Address"].as_str()
                );
                assert!(ip.prefix.is_none() && ip.description.is_none());
            }
        }
    }
}

/// Typed construction independently of decoding the endpoint/security corpus.
fn typed(body: &Value) -> m::RDMAInitializeSetupMT {
    let mut value = decode(&golden());
    let endpoints = body["MessageData"]["Endpoint"]
        .as_array()
        .unwrap()
        .iter()
        .map(|ip| {
            if let Some(ip) = ip.get("IPv4_Endpoint") {
                m::NetworkEndpointType::IPv4Endpoint(m::IPv4EndpointType {
                    ipv4_address: m::IPv4AddressType::new(ip["IPv4_Address"].as_str().unwrap())
                        .unwrap(),
                    prefix: None,
                    description: None,
                })
            } else {
                m::NetworkEndpointType::IPv6Endpoint(m::IPv6EndpointType {
                    ipv6_address: m::IPv6AddressType::new(
                        ip["IPv6_Endpoint"]["IPv6_Address"].as_str().unwrap(),
                    )
                    .unwrap(),
                    prefix: None,
                    description: None,
                })
            }
        })
        .collect();
    value.messagedata.endpoint = m::UnboundedVec::new(endpoints).unwrap();
    if body["SecurityInformation"]["OwnerProducer"]
        .as_array()
        .unwrap()
        .len()
        == 4
    {
        value.securityinformation.ownerproducer = m::UnboundedVec::new(vec![
            m::OwnerProducerChoiceType::GovernmentIdentifier(m::OwnerProducerEnum::USA),
            m::OwnerProducerChoiceType::NATOSpecialWord(
                m::NATOSpecialWordsType::new("NATO:EXAMPLE").unwrap(),
            ),
            m::OwnerProducerChoiceType::GovernmentIdentifier(m::OwnerProducerEnum::CAN),
            m::OwnerProducerChoiceType::NATOSpecialWord(
                m::NATOSpecialWordsType::new("NATO:EXAMPLE").unwrap(),
            ),
        ])
        .unwrap();
    }
    value
}

/// Payload failures with the exact Task103 broker/decoder boundary.
fn negatives() -> Vec<(&'static str, Value, &'static str, bool)> {
    let mut cases = Vec::new();
    for (name, pointer, replacement, diagnostic, decoder) in [
        (
            "UUID variant",
            "/MessageHeader/SystemID/UUID",
            json!("12345678-1234-1234-1234-123456789abc"),
            "UUID",
            false,
        ),
        (
            "schema version",
            "/MessageHeader/SchemaVersion",
            json!("002.5"),
            "SchemaVersion",
            false,
        ),
        (
            "IPv4",
            "/MessageData/Endpoint/0/IPv4_Endpoint/IPv4_Address",
            json!("999.0.0.1"),
            "IPv4_Address",
            false,
        ),
        (
            "IPv6",
            "/MessageData/Endpoint/1/IPv6_Endpoint/IPv6_Address",
            json!("not-an-ipv6"),
            "IPv6_Address",
            false,
        ),
        (
            "both Choice arms",
            "/MessageData/Endpoint/0",
            json!({
                "IPv4_Endpoint":{"IPv4_Address":"192.0.2.10"},
                "IPv6_Endpoint":{"IPv6_Address":"2001:db8::1"}
            }),
            "ChoiceViolation",
            false,
        ),
        (
            "unknown Choice member",
            "/MessageData/Endpoint/0",
            json!({"NotAnEndpoint":{}}),
            "NotAnEndpoint",
            false,
        ),
        (
            "endpoint number",
            "/MessageData/Endpoint",
            json!(42),
            "OccurrenceMismatch",
            false,
        ),
        (
            "timestamp",
            "/MessageHeader/Timestamp",
            json!("not-a-dateZ"),
            "Timestamp",
            true,
        ),
    ] {
        let mut body = golden();
        *body.pointer_mut(pointer).unwrap() = replacement;
        cases.push((name, body, diagnostic, decoder));
    }
    let mut missing = golden();
    missing["MessageData"]
        .as_object_mut()
        .unwrap()
        .remove("RDMA_InitializeSetupID");
    cases.push(("missing setup ID", missing, "RDMA_InitializeSetupID", false));
    cases
}

#[test]
fn golden_choices_and_rejections() {
    assert_version_surface();
    assert_eq!(svc::exchange_receive::MESSAGE_NAME, MESSAGE);
    assert_eq!(svc::exchange_publish::MESSAGE_NAME, MESSAGE);
    assert_eq!(svc::exchange_receive::TOPIC, TOPIC);
    assert_eq!(svc::exchange_publish::TOPIC, TOPIC);
    for (name, body) in positives() {
        inspect(&decode(&body), &body);
        inspect(&typed(&body), &body);
        assert_eq!(
            ServiceCodec.encode_payload(&typed(&body)).unwrap(),
            canonical(&body),
            "{name}"
        );
        inspect(&decode(&canonical(&body)), &body);
    }
    for (name, body, _, _) in negatives() {
        let result: Result<m::RDMAInitializeSetupMT, _> = ServiceCodec.decode_payload(&body);
        let error = result.expect_err(name);
        let path = match name {
            "UUID variant" => "MessageHeader.SystemID.UUID",
            "schema version" => "MessageHeader.SchemaVersion",
            "timestamp" => "MessageHeader.Timestamp",
            "missing setup ID" => "missing required member \"RDMA_InitializeSetupID\"",
            _ => "MessageData.Endpoint",
        };
        assert!(error.message().contains(path), "{error}");
        eprintln!("RUST-OMS-002 {VERSION} direct {name}: {error}");
    }
    assert!(
        m::UniversallyUniqueIdentifierType::new("12345678-1234-1234-1234-123456789abc").is_none()
    );
    assert!(m::UCISchemaVersionStringType::new("002.5").is_none());
    assert!(m::IPv4AddressType::new("999.0.0.1").is_none());
    assert!(m::IPv6AddressType::new("not-an-ipv6").is_none());
    assert!(m::DateTimeType::new("not-a-dateZ").is_none());
    eprintln!("RUST-OMS-002 {VERSION} GOLDEN: PASSED");
}

#[test]
fn pinned_sleet_roundtrip_and_recovery() {
    if std::env::var_os("AMS_GRA_SLEET_BIN").is_none() {
        assert!(
            std::env::var_os("AMS_GRA_REQUIRE_REAL_UCI_RDMA").is_none(),
            "explicit qualification needs pinned Sleet"
        );
        eprintln!("SKIPPED: AMS_GRA_SLEET_BIN not configured");
        return;
    }
    let (mut server, url) = start_sleet();
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
        tokio::time::timeout(WAIT, observer.subscribe("wire", MESSAGE, TOPIC, None))
            .await
            .unwrap()
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
    for (name, body) in positives() {
        // Independent external publication includes Endpoint: [], which must
        // be accepted on the wire even though generated encoding omits it.
        let document = json!({MESSAGE: body});
        executor
            .block_on(raw_publish(&mut publisher, TOPIC, &document))
            .unwrap();
        assert_eq!(receive_wire(&mut observer), document);
        let received = seen.recv_timeout(WAIT).unwrap();
        inspect(&received, &body);
        assert_eq!(
            ServiceCodec.encode_payload(&received).unwrap(),
            canonical(&body)
        );
        svc::exchange_publish::publish(&mut runtime, &typed(&body)).unwrap();
        assert_eq!(
            receive_wire(&mut observer),
            json!({MESSAGE: canonical(&body)})
        );
        let received = seen.recv_timeout(WAIT).unwrap();
        inspect(&received, &body);
        assert_eq!(
            ServiceCodec.encode_payload(&received).unwrap(),
            canonical(&body)
        );
        assert!(runtime.try_recv_event().is_none());
        eprintln!("RUST-OMS-002 {VERSION} {name}: external and generated typed MSG PASSED");
    }
    let mut cases: Vec<_> = negatives()
        .into_iter()
        .map(|(name, body, diagnostic, decoder)| {
            (name, json!({MESSAGE: body}), diagnostic, decoder)
        })
        .collect();
    cases.push((
        "wrong global element",
        json!({"RDMA_InitializeSetupMT":golden()}),
        "UnknownGlobalElement",
        false,
    ));
    for (name, document, diagnostic, decoder) in cases {
        let result = executor.block_on(raw_publish(&mut publisher, TOPIC, &document));
        if decoder {
            result.expect("Task103: pinned Sleet admits invalid timestamp");
            assert_eq!(receive_wire(&mut observer), document, "actual invalid MSG");
            match runtime
                .recv_event_timeout(WAIT)
                .expect("observable decode rejection")
            {
                RuntimeEvent::SubscriptionDecodeError {
                    sid,
                    message_name,
                    error: MessageDecodeError::Codec(error),
                } => {
                    assert!(!sid.is_empty());
                    assert_eq!(message_name, MESSAGE);
                    assert!(error.message().contains(diagnostic), "{error}");
                    eprintln!(
                        "RUST-OMS-002 {VERSION} {name}: Sleet MSG -> generated decoder rejected: {error}"
                    );
                }
                other => panic!("wrong rejection boundary: {other:?}"),
            }
        } else {
            let error = result.expect_err("pinned Sleet must reject invalid document");
            assert!(matches!(error, ClientError::Server { .. }), "{error}");
            assert!(error.to_string().contains(diagnostic), "{error}");
            eprintln!("RUST-OMS-002 {VERSION} {name}: Sleet rejected: {error}");
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
            "zero invalid typed callbacks: {name}"
        );
        // Recover after EVERY case without replacing the subscription/runtime.
        svc::exchange_publish::publish(&mut runtime, &typed(&golden())).unwrap();
        assert_eq!(receive_wire(&mut observer), json!({MESSAGE: golden()}));
        let recovered = seen
            .recv_timeout(WAIT)
            .expect("same active subscription recovers");
        inspect(&recovered, &golden());
        assert_eq!(ServiceCodec.encode_payload(&recovered).unwrap(), golden());
        assert!(runtime.try_recv_event().is_none());
        eprintln!("RUST-OMS-002 {VERSION} {name}: same-subscription recovery PASSED");
    }
    let denied = executor
        .block_on(raw_publish(
            &mut publisher,
            "unauthorized-topic",
            &json!({MESSAGE:golden()}),
        ))
        .unwrap_err();
    assert!(matches!(denied, ClientError::Server { .. }), "{denied}");
    assert!(denied.to_string().contains("not authorized"), "{denied}");
    executor.block_on(async {
        assert!(tokio::time::timeout(QUIET, observer.recv()).await.is_err());
    });
    assert!(runtime.recv_event_timeout(QUIET).is_none());
    assert!(seen.recv_timeout(QUIET).is_err());
    eprintln!("RUST-OMS-002 {VERSION} unauthorized topic: Sleet rejected: {denied}");
    svc::exchange_publish::publish(&mut runtime, &typed(&golden())).unwrap();
    assert_eq!(receive_wire(&mut observer), json!({MESSAGE:golden()}));
    inspect(&seen.recv_timeout(WAIT).unwrap(), &golden());
    // A second live GENERATED typed observer rules out a dead connection as
    // the cause of silence after removing the original subscription.
    let (tx, witness) = mpsc::channel();
    let observer_subscription = svc::exchange_receive::subscribe(
        &mut runtime,
        move |value: &svc::exchange_receive::Payload| {
            tx.send(value.clone()).unwrap();
        },
    )
    .unwrap();
    subscription.unsubscribe().expect("generated input UNSUB");
    svc::exchange_publish::publish(&mut runtime, &typed(&golden())).unwrap();
    assert_eq!(receive_wire(&mut observer), json!({MESSAGE:golden()}));
    inspect(
        &witness.recv_timeout(WAIT).expect("live typed witness"),
        &golden(),
    );
    assert!(seen.recv_timeout(QUIET).is_err(), "no callback after UNSUB");
    observer_subscription.unsubscribe().unwrap();
    runtime.close().expect("runtime worker clean shutdown");
    executor.block_on(async {
        tokio::time::timeout(WAIT, observer.unsubscribe("wire"))
            .await
            .unwrap()
            .unwrap();
    });
    server.shutdown();
    eprintln!("RUST-OMS-002 {VERSION} SLEET AND RECOVERY: PASSED");
}
