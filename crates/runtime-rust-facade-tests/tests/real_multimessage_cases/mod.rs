use ams_gra_oms_runtime_rust::{
    MessageDecodeError, OmsJsonCodec, RuntimeConfig, RuntimeEvent, SleetRuntime,
};
use generated::{
    model as m,
    service_api::{function_log as log, function_rdma as rdma},
    service_codec::ServiceCodec,
};
use serde_json::{Value, json};
use sleet_client::{CalClient, ClientError, InitOptions};
use std::collections::BTreeSet;
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

const WAIT: Duration = Duration::from_secs(10);
const QUIET: Duration = Duration::from_millis(200);
// The shared helper uses TOPIC as the LA-CAL service ID, not as a route here.
const TOPIC: &str = "rust-oms-003-multimessage";
const LOG: &str = "rust-oms-003-log";
const RDMA: &str = "rust-oms-003-rdma";
const UUID: &str = "12345678-1234-1234-8234-123456789abc";

include!("../real_sleet_support/mod.rs");
include!("inspect.rs");

fn log_golden() -> Value {
    serde_json::from_str(LOG_GOLDEN).unwrap()
}
fn rdma_golden() -> Value {
    serde_json::from_str(RDMA_GOLDEN).unwrap()
}

fn inspect_log_body(value: &m::LogMT, body: &Value) {
    inspect_log(value);
    let bytes = match body["MessageData"].get("Data").and_then(Value::as_str) {
        None => None,
        Some("") => Some(&[][..]),
        Some("0080FF") => Some(&[0, 128, 255][..]),
        other => panic!("unexpected independent binary fixture: {other:?}"),
    };
    assert_eq!(data(value), bytes);
}

// Live OS evidence, not another transport or a proxy. Only this process's
// established client sockets to this test's broker port are counted.
fn connections(url: &str) -> usize {
    let port: u16 = url.rsplit(':').next().unwrap().parse().unwrap();
    let sockets: BTreeSet<_> = std::fs::read_dir("/proc/self/fd")
        .unwrap()
        .filter_map(|fd| std::fs::read_link(fd.ok()?.path()).ok())
        .filter_map(|path| {
            path.to_str()?
                .strip_prefix("socket:[")?
                .strip_suffix(']')
                .map(str::to_owned)
        })
        .collect();
    std::fs::read_to_string("/proc/net/tcp")
        .unwrap()
        .lines()
        .skip(1)
        .filter(|line| {
            let fields: Vec<_> = line.split_whitespace().collect();
            fields[2].ends_with(&format!(":{port:04X}"))
                && fields[3] == "01"
                && sockets.contains(fields[9])
        })
        .count()
}

fn workers() -> BTreeSet<PathBuf> {
    std::fs::read_dir("/proc/self/task")
        .unwrap()
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            std::fs::read_to_string(path.join("comm"))
                .ok()?
                .starts_with("ams-gra-oms-lac")
                .then_some(path)
        })
        .collect()
}

fn wire(executor: &tokio::runtime::Runtime, observer: &mut CalClient, sid: &str, document: &Value) {
    executor.block_on(async {
        let msg = tokio::time::timeout(WAIT, observer.recv())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(msg.sid, sid, "OWP witness subscription identity");
        assert_eq!(
            serde_json::from_str::<Value>(&msg.payload).unwrap(),
            *document
        );
    });
    eprintln!(
        "RUST-OMS-003 {VERSION} OWP MSG {sid}: {}",
        document.as_object().unwrap().keys().next().unwrap()
    );
}

fn quiet(logs: &mpsc::Receiver<m::LogMT>, rdmas: &mpsc::Receiver<m::RDMAInitializeSetupMT>) {
    assert!(
        matches!(
            logs.recv_timeout(QUIET),
            Err(mpsc::RecvTimeoutError::Timeout)
        ),
        "no stray Log callback; sender still live"
    );
    assert!(
        matches!(
            rdmas.recv_timeout(QUIET),
            Err(mpsc::RecvTimeoutError::Timeout)
        ),
        "no stray RDMA callback; sender still live"
    );
}

#[test]
fn single_connection_interleaving_isolation_and_unsubscribe() {
    if std::env::var_os("AMS_GRA_SLEET_BIN").is_none() {
        assert!(
            std::env::var_os("AMS_GRA_REQUIRE_REAL_UCI_MULTIMESSAGE").is_none(),
            "qualification requires pinned Sleet"
        );
        eprintln!("SKIPPED: RUST-OMS-003 live broker not configured");
        return;
    }
    fn compatible<C: OmsJsonCodec<m::LogMT> + OmsJsonCodec<m::RDMAInitializeSetupMT>>() {}
    compatible::<ServiceCodec>();
    assert_version_surface();
    assert_eq!(generated::service_api::SERVICE_NAME, TOPIC);
    assert_eq!(
        (
            log::exchange_receive::MESSAGE_NAME,
            log::exchange_publish::MESSAGE_NAME
        ),
        ("Log", "Log")
    );
    assert_eq!(
        (
            rdma::exchange_receive::MESSAGE_NAME,
            rdma::exchange_publish::MESSAGE_NAME
        ),
        ("RDMA_InitializeSetup", "RDMA_InitializeSetup")
    );
    assert_eq!(
        (log::exchange_receive::TOPIC, log::exchange_publish::TOPIC),
        (LOG, LOG)
    );
    assert_eq!(
        (rdma::exchange_receive::TOPIC, rdma::exchange_publish::TOPIC),
        (RDMA, RDMA)
    );
    let (mut server, url) = start_sleet_with_topics(&[LOG, RDMA]);
    let before = workers();
    let mut runtime: SleetRuntime<ServiceCodec> = connect(&url);
    let owned: Vec<_> = workers().difference(&before).cloned().collect();
    assert_eq!(owned.len(), 1, "one new worker");
    assert_eq!(
        connections(&url),
        1,
        "one active runtime connection after INIT"
    );
    let (tx, logs) = mpsc::channel();
    let log_sub = log::exchange_receive::subscribe(&mut runtime, move |v: &m::LogMT| {
        tx.send(v.clone()).unwrap();
    })
    .unwrap();
    let (tx, rdmas) = mpsc::channel();
    let rdma_sub =
        rdma::exchange_receive::subscribe(&mut runtime, move |v: &m::RDMAInitializeSetupMT| {
            tx.send(v.clone()).unwrap();
        })
        .unwrap();
    let log_id = log_sub.id().to_owned();
    let rdma_id = rdma_sub.id().to_owned();
    assert_ne!(log_id, rdma_id);
    assert_eq!(
        connections(&url),
        1,
        "both generated subscriptions share connection"
    );
    eprintln!(
        "RUST-OMS-003 {VERSION} runtime SUB Log/{LOG}={log_id}, RDMA_InitializeSetup/{RDMA}={rdma_id}; connections=1 workers=1"
    );
    let executor = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let mut publisher = executor.block_on(raw_connect(&url));
    let mut observer = executor.block_on(raw_connect(&url));
    executor.block_on(async {
        for (sid, message, topic) in [
            ("wire-log", "Log", LOG),
            ("wire-rdma", "RDMA_InitializeSetup", RDMA),
            ("cross-log", "Log", RDMA),
            ("cross-rdma", "RDMA_InitializeSetup", LOG),
        ] {
            tokio::time::timeout(WAIT, observer.subscribe(sid, message, topic, None))
                .await
                .unwrap()
                .unwrap();
        }
    });
    assert_eq!(
        connections(&url),
        3,
        "runtime plus external publisher and witness"
    );
    let mut deliveries = 0;
    let rdma_cases = positives();
    // Alternate message families and generated/external publishers. Every
    // expected document originates in Task092/093 fixtures, never encoding.
    for (index, (name, body)) in rdma_cases.iter().enumerate() {
        if index < 3 {
            let mut log_body = log_golden();
            match index {
                0 => {
                    log_body["MessageData"]
                        .as_object_mut()
                        .unwrap()
                        .remove("Data");
                }
                1 => log_body["MessageData"]["Data"] = json!(""),
                _ => {}
            }
            for external in [false, true] {
                let document = json!({"Log": log_body});
                if external {
                    executor
                        .block_on(raw_publish(&mut publisher, LOG, &document))
                        .unwrap();
                } else {
                    let value: m::LogMT = ServiceCodec.decode_payload(&log_body).unwrap();
                    inspect_log_body(&value, &log_body);
                    log::exchange_publish::publish(&mut runtime, &value).unwrap();
                }
                wire(&executor, &mut observer, "wire-log", &document);
                inspect_log_body(&logs.recv_timeout(WAIT).unwrap(), &log_body);
                deliveries += 1;
                quiet(&logs, &rdmas);
            }
        }
        for external in [true, false] {
            let mut body = body.clone();
            if name == &"empty" {
                body["MessageData"]
                    .as_object_mut()
                    .unwrap()
                    .remove("Endpoint");
            }
            let document = json!({"RDMA_InitializeSetup": body});
            if external {
                executor
                    .block_on(raw_publish(&mut publisher, RDMA, &document))
                    .unwrap();
            } else {
                let value: m::RDMAInitializeSetupMT = ServiceCodec.decode_payload(&body).unwrap();
                inspect_rdma(&value, &body);
                rdma::exchange_publish::publish(&mut runtime, &value).unwrap();
            }
            wire(&executor, &mut observer, "wire-rdma", &document);
            inspect_rdma(&rdmas.recv_timeout(WAIT).unwrap(), &body);
            deliveries += 1;
            quiet(&logs, &rdmas);
        }
    }
    assert_eq!(deliveries, 18);
    // Valid messages on the OTHER topic are witnessed, not misrouted.
    for (topic, sid, document) in [
        (RDMA, "cross-log", json!({"Log": log_golden()})),
        (
            LOG,
            "cross-rdma",
            json!({"RDMA_InitializeSetup": rdma_golden()}),
        ),
    ] {
        executor
            .block_on(raw_publish(&mut publisher, topic, &document))
            .unwrap();
        wire(&executor, &mut observer, sid, &document);
        quiet(&logs, &rdmas);
    }
    for bad_log in [true, false] {
        let (topic, message, sid, witness, mut body, diagnostic) = if bad_log {
            (LOG, "Log", &log_id, "wire-log", log_golden(), "Data")
        } else {
            (
                RDMA,
                "RDMA_InitializeSetup",
                &rdma_id,
                "wire-rdma",
                rdma_golden(),
                "Timestamp",
            )
        };
        if bad_log {
            body["MessageData"]["Data"] = json!("0G");
        } else {
            body["MessageHeader"]["Timestamp"] = json!("not-a-dateZ");
        }
        let document = json!({message: body});
        executor
            .block_on(raw_publish(&mut publisher, topic, &document))
            .expect("Sleet admits this decoder-negative PUB");
        wire(&executor, &mut observer, witness, &document);
        match runtime
            .recv_event_timeout(WAIT)
            .expect("observable decoder rejection")
        {
            RuntimeEvent::SubscriptionDecodeError {
                sid: actual,
                message_name,
                error: MessageDecodeError::Codec(error),
            } => {
                assert_eq!(&actual, sid);
                assert_eq!(message_name, message);
                assert!(error.to_string().contains(diagnostic), "{error}");
                eprintln!("RUST-OMS-003 {VERSION} decode rejection {message}/{actual}: {error}");
            }
            other => panic!("wrong decoder failure identity/boundary: {other:?}"),
        }
        quiet(&logs, &rdmas);
        // Unaffected family FIRST, then failed family, using the original SIDs.
        for send_log in [!bad_log, bad_log] {
            if send_log {
                let body = log_golden();
                let value: m::LogMT = ServiceCodec.decode_payload(&body).unwrap();
                log::exchange_publish::publish(&mut runtime, &value).unwrap();
                wire(&executor, &mut observer, "wire-log", &json!({"Log": body}));
                inspect_log_body(&logs.recv_timeout(WAIT).unwrap(), &body);
            } else {
                let body = rdma_golden();
                let value: m::RDMAInitializeSetupMT = ServiceCodec.decode_payload(&body).unwrap();
                rdma::exchange_publish::publish(&mut runtime, &value).unwrap();
                wire(
                    &executor,
                    &mut observer,
                    "wire-rdma",
                    &json!({"RDMA_InitializeSetup": body}),
                );
                inspect_rdma(&rdmas.recv_timeout(WAIT).unwrap(), &body);
            }
            deliveries += 1;
            quiet(&logs, &rdmas);
        }
        assert!(runtime.is_running());
        assert!(runtime.try_recv_event().is_none());
        eprintln!(
            "RUST-OMS-003 {VERSION} {message}/{sid} cross-message isolation and original-subscription recovery: PASSED"
        );
    }
    // Broker rejection is separately identified; it is NOT decoder evidence.
    let denied = executor
        .block_on(raw_publish(
            &mut publisher,
            "unauthorized-topic",
            &json!({"Log":log_golden()}),
        ))
        .unwrap_err();
    assert!(matches!(denied, ClientError::Server { .. }));
    assert!(runtime.recv_event_timeout(QUIET).is_none());
    log_sub.unsubscribe().unwrap();
    let value: m::LogMT = ServiceCodec.decode_payload(&log_golden()).unwrap();
    log::exchange_publish::publish(&mut runtime, &value).unwrap();
    wire(
        &executor,
        &mut observer,
        "wire-log",
        &json!({"Log":log_golden()}),
    );
    assert!(
        logs.recv_timeout(QUIET).is_err(),
        "removed Log handler is silent"
    );
    let value: m::RDMAInitializeSetupMT = ServiceCodec.decode_payload(&rdma_golden()).unwrap();
    rdma::exchange_publish::publish(&mut runtime, &value).unwrap();
    wire(
        &executor,
        &mut observer,
        "wire-rdma",
        &json!({"RDMA_InitializeSetup":rdma_golden()}),
    );
    inspect_rdma(&rdmas.recv_timeout(WAIT).unwrap(), &rdma_golden());
    deliveries += 1;
    rdma_sub.unsubscribe().unwrap();
    assert!(runtime.try_recv_event().is_none());
    assert_eq!(
        connections(&url),
        3,
        "UNSUB did not close shared connection"
    );
    runtime.close().expect("one shutdown joins shared worker");
    assert!(!owned[0].exists(), "worker task reaped");
    assert_eq!(
        connections(&url),
        2,
        "runtime socket closed; witnesses remain live"
    );
    drop(observer);
    drop(publisher);
    drop(executor);
    assert_eq!(connections(&url), 0, "all test clients cleaned up");
    server.shutdown();
    assert_eq!(deliveries, 23);
    eprintln!(
        "RUST-OMS-003 {VERSION} typed deliveries={deliveries}; unsubscribe witness and socket/worker cleanup: PASSED"
    );
    eprintln!("RUST-OMS-003 {VERSION} SINGLE CONNECTION AND ISOLATION: PASSED");
}
