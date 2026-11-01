use ams_gra_oms_runtime_rust::{
    MessageDecodeError, OmsJsonCodec, RuntimeConfig, RuntimeEvent, SleetRuntime,
};
use generated::{
    model as m,
    service_api::{function_log as log, function_rdma as rdma, function_survivability as risk},
    service_codec::ServiceCodec,
};
use serde_json::{Value, json};
use sleet_client::{CalClient, ClientError, InitOptions};
use std::collections::BTreeSet;
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::thread::ThreadId;
use std::time::{Duration, Instant};

const WAIT: Duration = Duration::from_secs(10);
const QUIET: Duration = Duration::from_millis(200);
const TOPIC: &str = "rust-oms-004-survivability";
const LOG: &str = "rust-oms-003-log";
const RDMA: &str = "rust-oms-003-rdma";
const RISK: &str = "rust-oms-004-survivability";
const UUID: &str = "12345678-1234-1234-8234-123456789abc";
const MESSAGES: [&str; 3] = ["Log", "RDMA_InitializeSetup", "SurvivabilityRiskLevel"];
const TOPICS: [&str; 3] = [LOG, RDMA, RISK];
const WITNESSES: [&str; 3] = ["wire-log", "wire-rdma", "wire-risk"];

include!("../real_sleet_support/mod.rs");
include!("../real_sleet_support/connections.rs");
// Existing independently authored Log/RDMA model inspection and endpoint corpus.
include!("../real_multimessage_cases/inspect.rs");
include!("inspect.rs");

fn log_golden() -> Value {
    serde_json::from_str(LOG_GOLDEN).unwrap()
}
fn rdma_golden() -> Value {
    serde_json::from_str(RDMA_GOLDEN).unwrap()
}
fn golden(family: usize) -> Value {
    match family {
        0 => log_golden(),
        1 => rdma_golden(),
        2 => risk_golden(),
        _ => unreachable!(),
    }
}

fn wire(executor: &tokio::runtime::Runtime, observer: &mut CalClient, sid: &str, document: &Value) {
    let msg = executor.block_on(async {
        tokio::time::timeout(WAIT, observer.recv())
            .await
            .unwrap()
            .unwrap()
    });
    assert_eq!(msg.sid, sid, "independent OWP witness identity");
    assert_eq!(
        serde_json::from_str::<Value>(&msg.payload).unwrap(),
        *document
    );
    eprintln!("RUST-OMS-004 {VERSION} OWP MSG {sid}: {document}");
}

fn silent<T>(receiver: &mpsc::Receiver<T>, active: bool) {
    match receiver.recv_timeout(QUIET) {
        Err(mpsc::RecvTimeoutError::Timeout) => {}
        Err(mpsc::RecvTimeoutError::Disconnected) if !active => {}
        _ => panic!("unexpected typed callback or lost active subscription"),
    }
}

struct Live {
    runtime: SleetRuntime<ServiceCodec>,
    executor: tokio::runtime::Runtime,
    publisher: CalClient,
    observer: CalClient,
    logs: mpsc::Receiver<(ThreadId, m::LogMT)>,
    rdmas: mpsc::Receiver<(ThreadId, m::RDMAInitializeSetupMT)>,
    risks: mpsc::Receiver<(ThreadId, m::SurvivabilityRiskLevelMT)>,
    worker: Option<ThreadId>,
    counts: [usize; 3],
    active: [bool; 3],
}

impl Live {
    fn quiet(&self) {
        silent(&self.logs, self.active[0]);
        silent(&self.rdmas, self.active[1]);
        silent(&self.risks, self.active[2]);
    }

    fn worker(&mut self, id: ThreadId) {
        assert_eq!(
            *self.worker.get_or_insert(id),
            id,
            "all handlers use the original worker"
        );
    }

    fn deliver(&mut self, family: usize, input: &Value, expected: &Value, external: bool) {
        let body = if external { input } else { expected };
        let document = json!({MESSAGES[family]: body});
        if external {
            self.executor
                .block_on(raw_publish(&mut self.publisher, TOPICS[family], &document))
                .unwrap();
        } else {
            match family {
                0 => {
                    let value: m::LogMT = ServiceCodec.decode_payload(input).unwrap();
                    log::exchange_publish::publish(&mut self.runtime, &value).unwrap();
                }
                1 => {
                    let value: m::RDMAInitializeSetupMT =
                        ServiceCodec.decode_payload(input).unwrap();
                    rdma::exchange_publish::publish(&mut self.runtime, &value).unwrap();
                }
                2 => {
                    let value: m::SurvivabilityRiskLevelMT =
                        ServiceCodec.decode_payload(input).unwrap();
                    risk::exchange_publish::publish(&mut self.runtime, &value).unwrap();
                }
                _ => unreachable!(),
            }
        }
        wire(
            &self.executor,
            &mut self.observer,
            WITNESSES[family],
            &document,
        );
        let id = match family {
            0 => {
                let (id, value) = self.logs.recv_timeout(WAIT).unwrap();
                inspect_log(&value);
                let octets: Option<&[u8]> =
                    match input["MessageData"].get("Data").and_then(Value::as_str) {
                        None => None,
                        Some("") => Some(&[]),
                        Some("0080FF") => Some(&[0, 128, 255]),
                        other => panic!("independent binary fixture: {other:?}"),
                    };
                assert_eq!(data(&value), octets);
                assert_eq!(ServiceCodec.encode_payload(&value).unwrap(), *expected);
                id
            }
            1 => {
                let (id, value) = self.rdmas.recv_timeout(WAIT).unwrap();
                inspect_rdma(&value, expected);
                assert_eq!(ServiceCodec.encode_payload(&value).unwrap(), *expected);
                id
            }
            2 => {
                let (id, value) = self.risks.recv_timeout(WAIT).unwrap();
                inspect_risk(&value, expected);
                assert_eq!(ServiceCodec.encode_payload(&value).unwrap(), *expected);
                id
            }
            _ => unreachable!(),
        };
        self.worker(id);
        self.counts[family] += 1;
        self.quiet();
        assert!(self.runtime.try_recv_event().is_none());
    }
}

#[test]
fn single_connection_three_families() {
    if std::env::var_os("AMS_GRA_SLEET_BIN").is_none() {
        assert!(
            std::env::var_os("AMS_GRA_REQUIRE_REAL_UCI_SURVIVABILITY").is_none(),
            "qualification requires pinned Sleet"
        );
        eprintln!("SKIPPED: RUST-OMS-004 live broker not configured");
        return;
    }
    assert_eq!(generated::service_api::SERVICE_NAME, TOPIC);
    assert_eq!(
        (
            log::exchange_receive::MESSAGE_NAME,
            log::exchange_publish::MESSAGE_NAME
        ),
        (MESSAGES[0], MESSAGES[0])
    );
    assert_eq!(
        (
            rdma::exchange_receive::MESSAGE_NAME,
            rdma::exchange_publish::MESSAGE_NAME
        ),
        (MESSAGES[1], MESSAGES[1])
    );
    assert_eq!(
        (
            risk::exchange_receive::MESSAGE_NAME,
            risk::exchange_publish::MESSAGE_NAME
        ),
        (MESSAGES[2], MESSAGES[2])
    );
    assert_eq!(
        (log::exchange_receive::TOPIC, log::exchange_publish::TOPIC),
        (LOG, LOG)
    );
    assert_eq!(
        (rdma::exchange_receive::TOPIC, rdma::exchange_publish::TOPIC),
        (RDMA, RDMA)
    );
    assert_eq!(
        (risk::exchange_receive::TOPIC, risk::exchange_publish::TOPIC),
        (RISK, RISK)
    );
    let (mut server, url) = start_sleet_with_topics(&TOPICS);
    let before = workers();
    let mut runtime: SleetRuntime<ServiceCodec> = connect(&url);
    let owned: BTreeSet<_> = workers().difference(&before).cloned().collect();
    assert_eq!(owned.len(), 1, "one new worker");
    assert_eq!(connections(&url), 1, "one runtime connection after INIT");
    let original_socket = connection_inodes(&url);
    let (tx, logs) = mpsc::channel();
    let log_sub = log::exchange_receive::subscribe(&mut runtime, move |v: &m::LogMT| {
        tx.send((std::thread::current().id(), v.clone())).unwrap();
    })
    .unwrap();
    let (tx, rdmas) = mpsc::channel();
    let rdma_sub =
        rdma::exchange_receive::subscribe(&mut runtime, move |v: &m::RDMAInitializeSetupMT| {
            tx.send((std::thread::current().id(), v.clone())).unwrap();
        })
        .unwrap();
    let (tx, risks) = mpsc::channel();
    let risk_sub =
        risk::exchange_receive::subscribe(&mut runtime, move |v: &m::SurvivabilityRiskLevelMT| {
            tx.send((std::thread::current().id(), v.clone())).unwrap();
        })
        .unwrap();
    let ids = [
        log_sub.id().to_owned(),
        rdma_sub.id().to_owned(),
        risk_sub.id().to_owned(),
    ];
    assert_eq!(ids.iter().collect::<BTreeSet<_>>().len(), 3);
    assert_eq!(
        connections(&url),
        1,
        "three subscriptions share one connection"
    );
    eprintln!("RUST-OMS-004 {VERSION} original SIDs={ids:?}; connections=1 workers=1");
    let executor = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let publisher = executor.block_on(raw_connect(&url));
    let mut observer = executor.block_on(raw_connect(&url));
    executor.block_on(async {
        for family in 0..3 {
            tokio::time::timeout(
                WAIT,
                observer.subscribe(WITNESSES[family], MESSAGES[family], TOPICS[family], None),
            )
            .await
            .unwrap()
            .unwrap();
            for other in 0..3 {
                if family != other {
                    tokio::time::timeout(
                        WAIT,
                        observer.subscribe(
                            &format!("cross-{family}-{other}"),
                            MESSAGES[family],
                            TOPICS[other],
                            None,
                        ),
                    )
                    .await
                    .unwrap()
                    .unwrap();
                }
            }
        }
    });
    assert_eq!(connections(&url), 3, "runtime plus two external clients");
    let mut live = Live {
        runtime,
        executor,
        publisher,
        observer,
        logs,
        rdmas,
        risks,
        worker: None,
        counts: [0; 3],
        active: [true; 3],
    };
    let risk_cases = corpus();
    let rdma_cases = positives();
    for (index, case) in risk_cases["positives"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
    {
        for external in [false, true] {
            if index < 3 {
                let mut body = log_golden();
                match index {
                    0 => {
                        body["MessageData"].as_object_mut().unwrap().remove("Data");
                    }
                    1 => body["MessageData"]["Data"] = json!(""),
                    _ => body["MessageData"]["Data"] = json!("0080FF"),
                }
                live.deliver(0, &body, &body, external);
            }
            let mut body = rdma_cases[index].1.clone();
            if rdma_cases[index].0 == "empty" {
                body["MessageData"]
                    .as_object_mut()
                    .unwrap()
                    .remove("Endpoint");
            }
            live.deliver(1, &body, &body, external);
            live.deliver(2, &case[1], &case[2], external);
        }
    }
    // Additional real optional sequence/exposure variants beyond the unchanged corpus.
    for exposure in [false, true] {
        let mut body = risk_golden();
        body["MessageData"]["AppliesToSystemID"] = json!([
            {"UUID": UUID}, {"UUID": "87654321-4321-4321-8321-cba987654321"}
        ]);
        if exposure {
            body["MessageData"]["RiskSetting"][0]["RiskLevel"][0]["MaximumContinuousExposure"] = json!({
                "CompositeExposureTime": "PT01.2300S", "CumulativeExposureTime": "P1DT2H"
            });
        }
        for external in [true, false] {
            live.deliver(2, &body, &body, external);
        }
    }
    // Cover the interior cardinalities too: three settings, with 2/3/4 levels.
    let mut body = risk_golden();
    body["MessageData"]["RiskSetting"] = json!([
        {"RiskPrioritization":"LOW", "RiskLevel":[{"AllowedRisk":"TRACK"}, {"AllowedRisk":"LAUNCH"}]},
        {"RiskPrioritization":"HIGH", "RiskLevel":[{"AllowedRisk":"KILL"}, {"AllowedRisk":"INTERCEPT"}, {"AllowedRisk":"TRACK"}]},
        {"RiskPrioritization":"MEDIUM", "RiskLevel":[{"AllowedRisk":"ACQUISITION"}, {"AllowedRisk":"TRACK"}, {"AllowedRisk":"LAUNCH"}, {"AllowedRisk":"INTERCEPT"}]}
    ]);
    for external in [true, false] {
        live.deliver(2, &body, &body, external);
    }
    assert_eq!(live.counts, [6, 12, 18]);
    // Every valid wrong-topic pairing is independently witnessed and never typed.
    for family in 0..3 {
        for (other, topic) in TOPICS.iter().enumerate() {
            if family == other {
                continue;
            }
            let document = json!({MESSAGES[family]: golden(family)});
            live.executor
                .block_on(raw_publish(&mut live.publisher, topic, &document))
                .unwrap();
            wire(
                &live.executor,
                &mut live.observer,
                &format!("cross-{family}-{other}"),
                &document,
            );
            live.quiet();
        }
    }
    for failed in 0..3 {
        let mut body = golden(failed);
        let diagnostic = if failed == 0 {
            body["MessageData"]["Data"] = json!("0G");
            "Data"
        } else {
            body["MessageHeader"]["Timestamp"] = json!("not-a-dateZ");
            "Timestamp"
        };
        let document = json!({MESSAGES[failed]: body});
        live.executor
            .block_on(raw_publish(&mut live.publisher, TOPICS[failed], &document))
            .expect("broker-admitted decoder negative");
        wire(
            &live.executor,
            &mut live.observer,
            WITNESSES[failed],
            &document,
        );
        match live
            .runtime
            .recv_event_timeout(WAIT)
            .expect("decoder event")
        {
            RuntimeEvent::SubscriptionDecodeError {
                sid,
                message_name,
                error: MessageDecodeError::Codec(error),
            } => {
                assert_eq!(sid, ids[failed]);
                assert_eq!(message_name, MESSAGES[failed]);
                assert!(error.to_string().contains(diagnostic), "{error}");
                assert!(
                    error
                        .to_string()
                        .contains(if failed == 0 { "hexBinary" } else { "DateTime" }),
                    "{error}"
                );
                eprintln!("RUST-OMS-004 {VERSION} decoder-negative {message_name}/{sid}: {error}");
            }
            other => panic!("wrong failure boundary: {other:?}"),
        }
        live.quiet();
        // BOTH unaffected families first, then recovery of the failed family.
        for family in (0..3)
            .filter(|family| *family != failed)
            .chain(std::iter::once(failed))
        {
            let body = golden(family);
            live.deliver(family, &body, &body, false);
        }
        assert!(live.runtime.is_running());
        assert_eq!(connections(&url), 3);
        assert!(
            original_socket.is_subset(&connection_inodes(&url)),
            "original runtime socket never replaced"
        );
        assert_eq!(
            workers()
                .difference(&before)
                .cloned()
                .collect::<BTreeSet<_>>(),
            owned
        );
        eprintln!(
            "RUST-OMS-004 {VERSION} {} original-SID recovery, both unaffected families: PASSED",
            MESSAGES[failed]
        );
    }
    // Broker-rejected risk corpus is NOT represented as Rust decoder execution.
    for case in risk_cases["invalid"].as_array().unwrap() {
        let error = live
            .executor
            .block_on(raw_publish(
                &mut live.publisher,
                RISK,
                &json!({"SurvivabilityRiskLevel": case[1]}),
            ))
            .unwrap_err();
        assert!(matches!(error, ClientError::Server { .. }));
        assert!(
            format!("{error:?}").contains(case[2].as_str().unwrap()),
            "{}: {error:?}",
            case[0]
        );
        assert!(live.runtime.recv_event_timeout(QUIET).is_none());
        live.quiet();
        eprintln!(
            "RUST-OMS-004 {VERSION} broker-rejected {}: {error}",
            case[0]
        );
    }
    assert!(
        live.executor
            .block_on(async { tokio::time::timeout(QUIET, live.observer.recv()).await })
            .is_err(),
        "no rejected payload reached witness"
    );
    log_sub.unsubscribe().unwrap();
    live.active[0] = false;
    let value: m::LogMT = ServiceCodec.decode_payload(&log_golden()).unwrap();
    log::exchange_publish::publish(&mut live.runtime, &value).unwrap();
    wire(
        &live.executor,
        &mut live.observer,
        WITNESSES[0],
        &json!({"Log": log_golden()}),
    );
    live.quiet();
    for family in [1, 2] {
        let body = golden(family);
        live.deliver(family, &body, &body, false);
    }
    eprintln!(
        "RUST-OMS-004 {VERSION} Log removed; independent Log witness and RDMA/Survivability survivors: PASSED"
    );
    rdma_sub.unsubscribe().unwrap();
    live.active[1] = false;
    let body = risk_golden();
    live.deliver(2, &body, &body, true);
    eprintln!("RUST-OMS-004 {VERSION} RDMA removed; original Survivability survivor: PASSED");
    risk_sub.unsubscribe().unwrap();
    live.active[2] = false;
    assert_eq!(connections(&url), 3, "unsubscribe does not close runtime");
    assert!(original_socket.is_subset(&connection_inodes(&url)));
    live.runtime
        .close()
        .expect("close joins actual runtime worker");
    assert!(!live.runtime.is_running());
    assert_eq!(
        connections(&url),
        2,
        "runtime socket closed, external clients remain"
    );
    assert!(original_socket.is_disjoint(&connection_inodes(&url)));
    assert_eq!(live.counts, [9, 16, 23]);
    eprintln!(
        "RUST-OMS-004 {VERSION} typed counts Log={} RDMA={} Survivability={}; original SIDs={ids:?}",
        live.counts[0], live.counts[1], live.counts[2]
    );
    drop(live);
    assert_eq!(connections(&url), 0);
    server.shutdown();
    // Actual join is additionally proved by the existing TLS-barrier lifecycle
    // regression, not a racy immediate /proc/self/task disappearance assertion.
    eprintln!("RUST-OMS-004 {VERSION} SINGLE CONNECTION AND ISOLATION: PASSED");
}
