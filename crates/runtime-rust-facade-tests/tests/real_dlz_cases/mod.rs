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
const TOPIC: &str = "task098-dlz";
const MESSAGE: &str = "DLZ";

include!("../real_sleet_support/mod.rs");

fn golden() -> Value {
    serde_json::from_str(GOLDEN).expect("unchanged Task101 independently authored fixture")
}

fn corpus() -> Value {
    let text = match VERSION {
        "002.5.0" => include_str!("../../../../tests/fixtures/rust-oms-006/corpus25.json"),
        "002.6.0" => include_str!("../../../../tests/fixtures/rust-oms-006/corpus26.json"),
        _ => panic!("unqualified release"),
    };
    serde_json::from_str(text).unwrap()
}

fn decode(body: &Value) -> m::DLZMT {
    ServiceCodec
        .decode_payload(body)
        .expect("generated DLZ decoder")
}

fn positives() -> Vec<(String, Value, Value)> {
    let mut cases: Vec<_> = corpus()["positives"]
        .as_array()
        .unwrap()
        .iter()
        .map(|case| {
            (
                case[0].as_str().unwrap().to_owned(),
                case[1].clone(),
                case[2].clone(),
            )
        })
        .collect();
    // Exercise actual Float32 fields separately from the Float64 carriers.
    let mut body = golden();
    body["MessageData"]["DLZ_Data"]["SteeringUnitVector"]["LOS_UnitVector"] = json!({
        "X": 0.5000000596046448,
        "Y": -0.0,
        "Z": 1.401298464324817e-45
    });
    // The generated encoder uses shortest Float32 spellings, not f64 expansion.
    let mut expected = body.clone();
    expected["MessageData"]["DLZ_Data"]["SteeringUnitVector"]["LOS_UnitVector"] =
        json!({"X": 0.50000006, "Y": -0.0, "Z": 1e-45});
    cases.push(("float32".to_owned(), body, expected));
    cases
}

fn inspect(value: &m::DLZMT, name: &str, expected: &Value) {
    assert_eq!(value.messageheader.schemaversion.as_str(), VERSION);
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
                assert_eq!(*id, m::OwnerProducerEnum::USA);
                assert_eq!(expected["GovernmentIdentifier"], "USA");
            }
            m::OwnerProducerChoiceType::NATOSpecialWord(word) => {
                assert_eq!(
                    word.as_str(),
                    expected["NATO_SpecialWord"].as_str().unwrap()
                );
            }
        }
    }
    let data = &value.messagedata.dlz_data;
    // IEEE-754 expectations are fixed hexadecimal encodings, not JSON equality
    // or a second use of the codec/parser being qualified.
    assert_eq!(
        data.rangeprobabilityofintercept.get().to_bits(),
        0x3ff0_0000_0000_0001
    );
    assert_eq!(
        data.loftangle.unwrap().get().to_bits(),
        0x8000_0000_0000_0000
    );
    assert_eq!(
        data.rangeminimum.get().to_bits(),
        if name == "boundary-3" {
            1
        } else {
            0x3ff0_0000_0000_0000
        }
    );
    assert_eq!(
        data.rangemaxguidance.get().to_bits(),
        if name == "boundary-4" {
            0x7fef_ffff_ffff_ffff
        } else {
            0x4059_0000_0000_0000
        }
    );
    assert_eq!(
        data.targetclosurerate.northspeed.get().to_bits(),
        0xc029_0000_0000_0000
    );
    assert_eq!(
        data.targetclosurerate.eastspeed.get().to_bits(),
        0x3fd0_0000_0000_0000
    );
    let vector = &data.steeringunitvector.as_ref().unwrap().los_unitvector;
    let bits = [
        vector.x.get().to_bits(),
        vector.y.get().to_bits(),
        vector.z.get().to_bits(),
    ];
    assert_eq!(
        bits,
        if name == "float32" {
            [0x3f00_0001, 0x8000_0000, 1]
        } else {
            [0x3f00_0000, 0xbf00_0000, 0x3f80_0000]
        }
    );
    let expected_data = &expected["MessageData"]["DLZ_Data"];
    for (field, actual) in [
        ("TimeOfIntercept", Some(&data.timeofintercept)),
        ("TimeToRelease", Some(&data.timetorelease)),
        ("TimeOfActivation", data.timeofactivation.as_ref()),
        ("TimeOfMPRF", data.timeofmprf.as_ref()),
        ("TimeOfAutonomous", data.timeofautonomous.as_ref()),
    ] {
        assert_eq!(
            actual.map(m::TimeType::as_str),
            expected_data.get(field).and_then(Value::as_str),
            "{name}: {field}"
        );
    }
}

fn typed(name: &str, body: &Value) -> m::DLZMT {
    let mut value = decode(body);
    // Independently construct the interesting binary carriers for publication;
    // a symmetric encoder/decoder mistake cannot supply these expectations.
    let data = &mut value.messagedata.dlz_data;
    data.rangeprobabilityofintercept =
        m::DistanceType::new(f64::from_bits(0x3ff0_0000_0000_0001)).unwrap();
    data.loftangle = Some(m::AngleType::new(f64::from_bits(0x8000_0000_0000_0000)).unwrap());
    if name == "boundary-3" {
        data.rangeminimum = m::DistanceType::new(f64::from_bits(1)).unwrap();
    }
    if name == "boundary-4" {
        data.rangemaxguidance =
            m::DistanceType::new(f64::from_bits(0x7fef_ffff_ffff_ffff)).unwrap();
    }
    if name == "float32" {
        let vector = &mut data.steeringunitvector.as_mut().unwrap().los_unitvector;
        vector.x = m::UnitBallFloatType::new(f32::from_bits(0x3f00_0001)).unwrap();
        vector.y = m::UnitBallFloatType::new(f32::from_bits(0x8000_0000)).unwrap();
        vector.z = m::UnitBallFloatType::new(f32::from_bits(1)).unwrap();
    }
    value
}

fn negatives() -> Vec<(String, Value, &'static str, bool)> {
    let mut cases = Vec::new();
    for (name, field, replacement, diagnostic, decoder) in [
        (
            "offset-time",
            "TimeOfIntercept",
            json!("12:34:56+00:00"),
            "FacetViolation",
            false,
        ),
        (
            "number-time",
            "TimeOfIntercept",
            json!(123456),
            "TypeMismatch",
            false,
        ),
        (
            "null-time",
            "TimeOfIntercept",
            Value::Null,
            "TypeMismatch",
            false,
        ),
        (
            "quoted-float64",
            "RangeMinimum",
            json!("1.0"),
            "TypeMismatch",
            false,
        ),
        (
            "constrained-numeric",
            "RangeMinimum",
            json!(-1.0),
            "FacetViolation",
            false,
        ),
        (
            "admitted-time",
            "TimeOfIntercept",
            json!("not-a-timeZ"),
            "TimeOfIntercept",
            true,
        ),
    ] {
        let mut body = golden();
        body["MessageData"]["DLZ_Data"][field] = replacement;
        cases.push((name.to_owned(), body, diagnostic, decoder));
    }
    let mut missing = golden();
    missing["MessageData"]["DLZ_Data"]
        .as_object_mut()
        .unwrap()
        .remove("TimeOfIntercept");
    cases.push((
        "missing-time".to_owned(),
        missing,
        "MissingRequiredMember",
        false,
    ));
    let mut choice = golden();
    choice["SecurityInformation"]["OwnerProducer"] =
        json!([{"GovernmentIdentifier":"USA", "NATO_SpecialWord":"NATO:COSMIC"}]);
    cases.push((
        "choice-two-arms".to_owned(),
        choice,
        "ChoiceViolation",
        false,
    ));
    cases
}

#[test]
fn model_codec_and_bits() {
    assert_eq!(svc::exchange_e::MESSAGE_NAME, MESSAGE);
    assert_eq!(svc::exchange_publish::MESSAGE_NAME, MESSAGE);
    assert_eq!(svc::exchange_e::TOPIC, TOPIC);
    assert_eq!(svc::exchange_publish::TOPIC, TOPIC);
    assert_eq!(corpus()["positives"][0][1], golden());
    for (name, body, expected) in positives() {
        inspect(&decode(&body), &name, &expected);
        let encoded = ServiceCodec.encode_payload(&typed(&name, &body)).unwrap();
        inspect(&decode(&encoded), &name, &expected);
        assert_eq!(encoded, expected, "{name}");
    }
    for case in corpus()["negatives"].as_array().unwrap() {
        let error = ServiceCodec.decode_payload(&case[1]) as Result<m::DLZMT, _>;
        assert!(error.is_err(), "Task101 {}: {}", case[0], case[2]);
    }
    for (name, body, _, _) in negatives() {
        let error = ServiceCodec.decode_payload(&body) as Result<m::DLZMT, _>;
        assert!(error.is_err(), "{name}");
    }
    for text in [
        "not-a-timeZ",
        "12:34:56+00:00",
        "24:00:00.001Z",
        "12:34:61Z",
        "12: 34:56Z",
    ] {
        assert!(m::TimeType::new(text).is_none(), "{text}");
    }
    assert!(m::DistanceType::new(-1.0).is_none());
    assert!(m::UnitBallFloatType::new(1.0001).is_none());
    // The codec accepts parsed Values; retain the existing configured runtime
    // policy rather than inventing a DLZ-specific duplicate-member parser.
    assert_eq!(
        RuntimeConfig::new("ws://127.0.0.1:1", TOPIC, VERSION).duplicate_member_policy(),
        ams_gra_oms_runtime_rust::DuplicateMemberPolicy::LegacyLastWins
    );
    let needle = "\"TimeOfIntercept\": \"12:34:56.5000Z\"";
    assert!(GOLDEN.contains(needle));
    let invalid_last: Value = serde_json::from_str(
        &GOLDEN.replace(needle, &format!("{needle}, \"TimeOfIntercept\": 123456")),
    )
    .unwrap();
    assert!((ServiceCodec.decode_payload(&invalid_last) as Result<m::DLZMT, _>).is_err());
    let valid_last: Value = serde_json::from_str(
        &GOLDEN.replace(needle, &format!("\"TimeOfIntercept\": 123456, {needle}")),
    )
    .unwrap();
    inspect(&decode(&valid_last), "full", &golden());
    eprintln!("RUST-OMS-006 {VERSION} MODEL CODEC BITS AND TIME: PASSED");
}

#[test]
fn pinned_sleet_roundtrip_and_recovery() {
    if std::env::var_os("AMS_GRA_SLEET_BIN").is_none() {
        assert!(
            std::env::var_os("AMS_GRA_REQUIRE_REAL_UCI_DLZ").is_none(),
            "explicit qualification needs pinned Sleet"
        );
        eprintln!("SKIPPED: AMS_GRA_SLEET_BIN not configured");
        return;
    }
    let (mut server, url) = start_sleet();
    let mut runtime = connect(&url);
    let (tx, seen) = mpsc::channel();
    let subscription = svc::exchange_e::subscribe(&mut runtime, move |value: &m::DLZMT| {
        tx.send(value.clone()).unwrap();
    })
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
    for (name, body, expected) in positives() {
        let result = executor.block_on(raw_publish(&mut publisher, TOPIC, &json!({MESSAGE: body})));
        if name == "boundary-0" {
            // Pinned Sleet applies .+Z to the uncollapsed string. The checked
            // model correctly collapses XML whitespace before publication.
            let error = result.expect_err("broker rejects unnormalized surrounding whitespace");
            assert!(matches!(error, ClientError::Server { .. }));
            assert!(error.to_string().contains("FacetViolation"), "{error}");
            executor.block_on(async {
                assert!(tokio::time::timeout(QUIET, observer.recv()).await.is_err());
            });
            assert!(seen.recv_timeout(QUIET).is_err());
            assert!(runtime.recv_event_timeout(QUIET).is_none());
            eprintln!(
                "RUST-OMS-006 {VERSION} raw whitespace: broker rejected before routing: {error}"
            );
        } else {
            result.unwrap();
            let wire = receive_wire(&mut observer);
            assert_eq!(wire, json!({MESSAGE: body}));
            inspect(&decode(&wire[MESSAGE]), &name, &expected);
            inspect(&seen.recv_timeout(WAIT).unwrap(), &name, &expected);
        }
        svc::exchange_publish::publish(&mut runtime, &typed(&name, &body)).unwrap();
        let wire = receive_wire(&mut observer);
        inspect(&decode(&wire[MESSAGE]), &name, &expected);
        if name == "float32" {
            // Float32 wire numbers need not have identical f64 expansions
            // after the broker parses/reserializes them. Their actual generated
            // f32 carriers were checked by bits above; compare all other data.
            let mut rest = wire[MESSAGE].clone();
            rest["MessageData"]["DLZ_Data"]["SteeringUnitVector"]["LOS_UnitVector"] =
                expected["MessageData"]["DLZ_Data"]["SteeringUnitVector"]["LOS_UnitVector"].clone();
            assert_eq!(rest, expected);
        } else {
            assert_eq!(wire, json!({MESSAGE: expected}));
        }
        inspect(&seen.recv_timeout(WAIT).unwrap(), &name, &expected);
        assert!(runtime.try_recv_event().is_none());
        eprintln!("RUST-OMS-006 {VERSION} {name}: typed DLZ MSG, bits and lexical values PASSED");
    }
    let mut cases: Vec<_> = negatives()
        .into_iter()
        .map(|(name, body, diagnostic, decoder)| {
            (name, json!({MESSAGE: body}), diagnostic, decoder)
        })
        .collect();
    cases.push((
        "wrong-global".to_owned(),
        json!({"DLZ_MT": golden()}),
        "UnknownGlobalElement",
        false,
    ));
    for (name, document, diagnostic, decoder) in cases {
        let result = executor.block_on(raw_publish(&mut publisher, TOPIC, &document));
        if decoder {
            result.expect("pinned broker admits not-a-timeZ; model must still reject");
            assert_eq!(
                receive_wire(&mut observer),
                document,
                "actual invalid DLZ MSG"
            );
            match runtime
                .recv_event_timeout(WAIT)
                .expect("decode rejection event")
            {
                RuntimeEvent::SubscriptionDecodeError {
                    sid,
                    message_name,
                    error: MessageDecodeError::Codec(error),
                } => {
                    assert_eq!(sid, subscription.id(), "original active subscription");
                    assert_eq!(message_name, MESSAGE);
                    assert!(error.message().contains(diagnostic), "{error}");
                    eprintln!(
                        "RUST-OMS-006 {VERSION} {name}: observed invalid MSG; generated decoder rejected: {error}"
                    );
                }
                other => panic!("wrong error boundary: {other:?}"),
            }
        } else {
            let error = result.expect_err("pinned broker must reject");
            assert!(matches!(error, ClientError::Server { .. }), "{error}");
            assert!(error.to_string().contains(diagnostic), "{name}: {error}");
            executor.block_on(async {
                assert!(tokio::time::timeout(QUIET, observer.recv()).await.is_err());
            });
            assert!(runtime.recv_event_timeout(QUIET).is_none());
            eprintln!("RUST-OMS-006 {VERSION} {name}: broker rejected: {error}");
        }
        assert!(
            seen.recv_timeout(QUIET).is_err(),
            "zero invalid typed callbacks"
        );
        svc::exchange_publish::publish(&mut runtime, &typed("full", &golden())).unwrap();
        assert_eq!(receive_wire(&mut observer), json!({MESSAGE: golden()}));
        inspect(
            &seen
                .recv_timeout(WAIT)
                .expect("same active subscription recovers"),
            "full",
            &golden(),
        );
        assert!(runtime.try_recv_event().is_none());
    }
    let denied = executor
        .block_on(raw_publish(
            &mut publisher,
            "forbidden",
            &json!({MESSAGE: golden()}),
        ))
        .unwrap_err();
    assert!(matches!(denied, ClientError::Server { .. }));
    assert!(denied.to_string().contains("not authorized"), "{denied}");
    executor.block_on(async {
        assert!(tokio::time::timeout(QUIET, observer.recv()).await.is_err());
    });
    assert!(runtime.recv_event_timeout(QUIET).is_none());
    assert!(seen.recv_timeout(QUIET).is_err());
    svc::exchange_publish::publish(&mut runtime, &typed("full", &golden())).unwrap();
    assert_eq!(receive_wire(&mut observer), json!({MESSAGE: golden()}));
    inspect(&seen.recv_timeout(WAIT).unwrap(), "full", &golden());
    assert!(runtime.try_recv_event().is_none());
    // A live second generated subscriber distinguishes UNSUB from a dead connection.
    let (tx, witness) = mpsc::channel();
    let second = svc::exchange_e::subscribe(&mut runtime, move |value: &m::DLZMT| {
        tx.send(value.clone()).unwrap();
    })
    .unwrap();
    subscription.unsubscribe().unwrap();
    svc::exchange_publish::publish(&mut runtime, &typed("full", &golden())).unwrap();
    assert_eq!(receive_wire(&mut observer), json!({MESSAGE: golden()}));
    inspect(&witness.recv_timeout(WAIT).unwrap(), "full", &golden());
    assert!(
        seen.recv_timeout(QUIET).is_err(),
        "no callbacks after unsubscribe"
    );
    second.unsubscribe().unwrap();
    runtime.close().expect("clean worker shutdown");
    executor.block_on(async {
        tokio::time::timeout(WAIT, observer.unsubscribe("wire"))
            .await
            .unwrap()
            .unwrap();
    });
    drop(observer);
    drop(publisher);
    drop(executor);
    server.shutdown();
    eprintln!("RUST-OMS-006 {VERSION} SLEET AND RECOVERY: PASSED");
}
