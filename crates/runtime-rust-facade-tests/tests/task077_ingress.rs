//! Raw mock ingress: counters, exact events, recovery, and independent connections.
mod codec_sample;
mod common;
use ams_gra_oms_runtime_api::SubscribeAdapter;
use ams_gra_oms_runtime_rust::{
    CodecError, DuplicateMemberPolicy, MessageDecodeError, OAM_NAMESPACE, OmsJsonCodec,
    RuntimeConfig, RuntimeEvent, SleetRuntime,
};
use common::{MockPeer, WAIT};
use serde_json::Value;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
    mpsc,
};

struct Counting(Arc<AtomicUsize>);
impl OmsJsonCodec<Value> for Counting {
    fn encode_payload(&self, value: &Value) -> Result<Value, CodecError> {
        Ok(value.clone())
    }
    fn decode_payload(&self, value: &Value) -> Result<Value, CodecError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        if value == &Value::Null {
            return Err(CodecError::new("null control"));
        }
        Ok(value.clone())
    }
}

#[test]
fn task077_dispatch_recovery_and_independent_policies() {
    let peers = [MockPeer::start(), MockPeer::start()];
    let counts = [Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0))];
    let handlers = [Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0))];
    let mut runtimes = Vec::new();
    let mut subscriptions = Vec::new();
    let mut receivers = Vec::new();
    for i in 0..2 {
        let mut config = RuntimeConfig::new(&peers[i].url, "svc-1", "000.1.0");
        if i == 1 {
            config = config.with_duplicate_member_policy(DuplicateMemberPolicy::Reject);
        }
        let mut runtime = SleetRuntime::connect(config, Counting(counts[i].clone())).unwrap();
        assert!(peers[i].next_text().starts_with("INIT "));
        let (tx, rx) = mpsc::channel();
        let counter = handlers[i].clone();
        subscriptions.push(
            runtime
                .subscribe(OAM_NAMESPACE, "M", "topic", None, move |v: &Value| {
                    counter.fetch_add(1, Ordering::SeqCst);
                    tx.send(v.clone()).unwrap();
                })
                .unwrap(),
        );
        peers[i].expect("SUB sub-1 M topic");
        receivers.push(rx);
        runtimes.push(runtime);
    }
    let (other_tx, other_rx) = mpsc::channel();
    let other = runtimes[1]
        .subscribe(OAM_NAMESPACE, "Other", "topic", None, move |v: &Value| {
            other_tx.send(v.clone()).unwrap();
        })
        .unwrap();
    peers[1].expect("SUB sub-2 Other topic");
    for raw in [
        r#"{"M":{"x":1,"x":2}}"#,
        r#"{"M":{"$type":"a","\u0024type":"b"}}"#,
        r#"{"M":{"unknown":[{"x":1,"x":1}]}}"#,
        r#"{"M":1,"M":2}"#,
    ] {
        let before = counts[1].load(Ordering::SeqCst);
        let handled = handlers[1].load(Ordering::SeqCst);
        for peer in &peers {
            peer.send(&format!("MSG sub-1 {raw}"));
        }
        assert_eq!(
            receivers[0].recv_timeout(WAIT).unwrap(),
            ams_gra_oms_runtime_rust::unwrap_global_element("M", raw).unwrap()
        );
        match runtimes[1].recv_event_timeout(WAIT) {
            Some(RuntimeEvent::SubscriptionDecodeError {
                sid,
                message_name,
                error: MessageDecodeError::DuplicateMember,
            }) => {
                assert_eq!(sid, "sub-1");
                assert_eq!(message_name, "M");
            }
            e => panic!("expected typed rejection: {e:?}"),
        }
        assert_eq!(counts[1].load(Ordering::SeqCst), before);
        assert_eq!(handlers[1].load(Ordering::SeqCst), handled);
        assert!(receivers[1].try_recv().is_err());
        peers[1].send(r#"MSG sub-2 {"Other":{"x":7}}"#);
        assert_eq!(other_rx.recv_timeout(WAIT).unwrap()["x"], 7);
        for peer in &peers {
            peer.send(r#"MSG sub-1 {"M":{"x":3}}"#);
        }
        for rx in &receivers {
            assert_eq!(rx.recv_timeout(WAIT).unwrap()["x"], 3);
        }
    }
    peers[1].send(r#"MSG sub-1 {"M":null}"#);
    assert!(matches!(
        runtimes[1].recv_event_timeout(WAIT),
        Some(RuntimeEvent::SubscriptionDecodeError {
            error: MessageDecodeError::Codec(_),
            ..
        })
    ));
    other.unsubscribe().unwrap();
    peers[1].expect("UNSUB sub-2");
    for (i, sub) in subscriptions.into_iter().enumerate() {
        sub.unsubscribe().unwrap();
        peers[i].expect("UNSUB sub-1");
    }
    for (i, runtime) in runtimes.into_iter().enumerate() {
        runtime.close().unwrap();
        peers[i].expect_closed();
    }
    println!("TASK077 DISPATCH RECOVERY: PASSED");
}

#[test]
fn task077_generated_codec_comparison() {
    use ams_gra_oms_runtime_rust_facade_tests::codec_oam::{
        model::CodecPayload, service_api::function_loop as svc, service_codec::ServiceCodec,
    };
    for policy in [
        DuplicateMemberPolicy::LegacyLastWins,
        DuplicateMemberPolicy::Reject,
    ] {
        let peer = MockPeer::start();
        let mut runtime = SleetRuntime::connect(
            RuntimeConfig::new(&peer.url, "svc-1", "000.1.0").with_duplicate_member_policy(policy),
            ServiceCodec,
        )
        .unwrap();
        assert!(peer.next_text().starts_with("INIT "));
        let (tx, rx) = mpsc::channel();
        let subscription =
            svc::exchange_input_notice::subscribe(&mut runtime, move |v: &CodecPayload| {
                tx.send(ServiceCodec.encode_payload(v).unwrap()).unwrap();
            })
            .unwrap();
        peer.expect("SUB sub-1 CodecNotice codec-topic");
        let body = serde_json::to_string(&codec_sample::sample_json()).unwrap();
        // Duplicate-free values, including parser number forms, produce identical codec results.
        for raw_body in [
            body.clone(),
            body.replacen("\"Level\":-10", "\"Level\":-1e1", 1),
        ] {
            let parsed: Value = serde_json::from_str(&raw_body).unwrap();
            // Float spelling in a bounded integer is rejected by the existing codec, not policy.
            let expected: Result<CodecPayload, CodecError> = ServiceCodec.decode_payload(&parsed);
            peer.send(&format!("MSG sub-1 {{\"CodecNotice\":{raw_body}}}"));
            match expected {
                Ok(value) => assert_eq!(
                    rx.recv_timeout(WAIT).unwrap(),
                    ServiceCodec.encode_payload(&value).unwrap()
                ),
                Err(error) => match runtime.recv_event_timeout(WAIT) {
                    Some(RuntimeEvent::SubscriptionDecodeError {
                        error: MessageDecodeError::Codec(actual),
                        ..
                    }) => assert_eq!(actual.message(), error.message()),
                    event => panic!("expected codec failure: {event:?}"),
                },
            }
        }
        for replacement in [
            "\"$type\":\"FutureShape\",\"$type\":\"BoxShape\"",
            "\"$type\":\"BoxShape\",\"$type\":\"FutureShape\"",
            "\"$type\":\"BoxShape\",\"\\u0024type\":\"BoxShape\"",
        ] {
            let raw_body = body.replacen("\"$type\":\"BoxShape\"", replacement, 1);
            assert_ne!(raw_body, body);
            peer.send(&format!("MSG sub-1 {{\"CodecNotice\":{raw_body}}}"));
            if policy == DuplicateMemberPolicy::Reject {
                assert!(matches!(
                    runtime.recv_event_timeout(WAIT),
                    Some(RuntimeEvent::SubscriptionDecodeError {
                        error: MessageDecodeError::DuplicateMember,
                        ..
                    })
                ));
            } else {
                let parsed: Value = serde_json::from_str(&raw_body).unwrap();
                match ServiceCodec.decode_payload(&parsed) {
                    Ok(v) => assert_eq!(
                        rx.recv_timeout(WAIT).unwrap(),
                        ServiceCodec.encode_payload(&v).unwrap()
                    ),
                    Err(e) => match runtime.recv_event_timeout(WAIT) {
                        Some(RuntimeEvent::SubscriptionDecodeError {
                            error: MessageDecodeError::Codec(actual),
                            ..
                        }) => assert_eq!(actual.message(), e.message()),
                        event => panic!("expected codec error: {event:?}"),
                    },
                }
            }
            peer.send(&format!("MSG sub-1 {{\"CodecNotice\":{body}}}"));
            assert_eq!(rx.recv_timeout(WAIT).unwrap(), codec_sample::sample_json());
        }
        subscription.unsubscribe().unwrap();
        peer.expect("UNSUB sub-1");
        runtime.close().unwrap();
        peer.expect_closed();
    }
    println!("TASK077 GENERATED CODEC COMPARISON: PASSED");
}
