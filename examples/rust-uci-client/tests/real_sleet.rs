//! Focused executable acceptance, not another codec qualification corpus.
//! Run explicitly with AMS_GRA_SLEET_BIN from run-real-sleet-test.sh.

use ams_gra_oms_runtime_rust::{RuntimeConfig, SleetRuntime};
use serde_json::{Value, json};
use sleet_client::{CalClient, ClientError, InitOptions};
use std::collections::BTreeSet;
use std::io::{BufRead, BufReader};
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

include!(concat!(env!("OUT_DIR"), "/generated.rs"));
use generated::service_codec::ServiceCodec;

const ROOT: &str = env!("AMS_GRA_UCI_ROOT_BUILD");
const VERSION: &str = env!("AMS_GRA_SCHEMA_VERSION_BUILD");
const TOPIC: &str = "rust-oms-003-multimessage";
const LOG: &str = "rust-oms-003-log";
const RDMA: &str = "rust-oms-003-rdma";
const RISK: &str = "rust-oms-004-survivability";
const DLZ: &str = "task098-dlz";
const TOPICS: [&str; 4] = [LOG, RDMA, RISK, DLZ];
const MESSAGES: [&str; 4] = [
    "Log",
    "RDMA_InitializeSetup",
    "SurvivabilityRiskLevel",
    "DLZ",
];
const FLAGS: [&str; 4] = [
    "--publish-log",
    "--publish-rdma",
    "--publish-survivability",
    "--publish-dlz",
];
const WAIT: Duration = Duration::from_secs(10);
const QUIET: Duration = Duration::from_millis(200);

// Reuse existing isolated broker configuration, external CalClient and Linux
// socket/worker witnesses. This imports source helpers, never the facade crate.
#[allow(dead_code)]
mod support {
    use super::*;
    include!("../../../crates/runtime-rust-facade-tests/tests/real_sleet_support/mod.rs");
    include!("../../../crates/runtime-rust-facade-tests/tests/real_sleet_support/connections.rs");
    include!("support/reconnect.rs");

    pub fn accept() {
        let (mut server, url) = start_sleet_with_topics(&TOPICS);
        // Startup probe also verifies the configured schema/identity handshake.
        connect(&url).close().unwrap();
        let executor = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let mut observer = executor.block_on(raw_connect(&url));
        let mut publisher = executor.block_on(raw_connect(&url));
        executor.block_on(async {
            for (index, (name, topic)) in MESSAGES.iter().zip(TOPICS).enumerate() {
                observer
                    .subscribe(&format!("wire-{index}"), name, topic, None)
                    .await
                    .unwrap();
            }
        });
        let fixture =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/service-generate");
        let suffix = env!("AMS_GRA_UCI_RELEASE_BUILD").replace('.', "");
        let log_file = fixture.join(format!("task093-real-log{suffix}.json"));
        let rdma_file = fixture.join(format!("task092-real-rdma{suffix}.json"));
        let risk_file = if suffix == "25" {
            fixture.join("task092-real-survivability25.json")
        } else {
            fixture.join("../task105/survivability26.json")
        };
        let dlz_file = fixture.join(format!("../task101/uci{suffix}-full-dlz.json"));
        let log: Value = serde_json::from_slice(&std::fs::read(&log_file).unwrap()).unwrap();
        let rdma: Value = serde_json::from_slice(&std::fs::read(&rdma_file).unwrap()).unwrap();
        let risk: Value = serde_json::from_slice(&std::fs::read(&risk_file).unwrap()).unwrap();
        let dlz: Value = serde_json::from_slice(&std::fs::read(&dlz_file).unwrap()).unwrap();
        let files = [&log_file, &rdma_file, &risk_file, &dlz_file];
        let args: Vec<_> = FLAGS
            .iter()
            .zip(files)
            .flat_map(|(flag, file)| [*flag, file.to_str().unwrap()])
            .collect();
        let mut app = Application::start(&url, &args);
        app.until("SUBSCRIBED");
        let subscription_ids = app.subscription_ids();
        assert_eq!(
            subscription_ids.len(),
            4,
            "four independently typed subscription identities"
        );
        assert_eq!(subscription_ids.iter().collect::<BTreeSet<_>>().len(), 4);
        assert_eq!(application_connections(app.child.id(), &url), 1);
        let task_dir = PathBuf::from(format!("/proc/{}/task", app.child.id()));
        let app_workers = std::fs::read_dir(task_dir)
            .unwrap()
            .filter(|entry| {
                std::fs::read_to_string(entry.as_ref().unwrap().path().join("comm"))
                    .unwrap()
                    .starts_with("ams-gra-oms-lac")
            })
            .count();
        assert_eq!(
            app_workers, 1,
            "one production runtime worker serves all four subscriptions"
        );
        eprintln!(
            "RUST-OMS-007 {VERSION} application workers=1 connections=1 subscription IDs={subscription_ids:?}"
        );
        for (index, (name, document)) in MESSAGES.iter().zip([&log, &rdma, &risk, &dlz]).enumerate()
        {
            let message = executor.block_on(async {
                tokio::time::timeout(WAIT, observer.recv())
                    .await
                    .unwrap()
                    .unwrap()
            });
            assert_eq!(message.sid, format!("wire-{index}"));
            assert_eq!(
                serde_json::from_str::<Value>(&message.payload).unwrap(),
                json!({(*name): document})
            );
            eprintln!(
                "RUST-OMS-007 {VERSION} independent OWP MSG {}: {}",
                message.sid, message.payload
            );
        }
        app.until("RECEIVED DLZ");
        assert_eq!(
            connections(&url),
            2,
            "witness process owns only its two independent sockets"
        );
        // Label/UUID changes distinguish external publication from self echo.
        let mut external_log = log.clone();
        external_log["MessageData"]["Label"] = json!("External007");
        let mut external_rdma = rdma.clone();
        external_rdma["MessageData"]["RDMA_InitializeSetupID"]["UUID"] =
            json!("22345678-1234-1234-8234-123456789abc");
        let corpus: Value = serde_json::from_slice(
            &std::fs::read(fixture.join(format!("../rust-oms-004/corpus{suffix}.json"))).unwrap(),
        )
        .unwrap();
        let mut external_risk = corpus["positives"][3][1].clone();
        external_risk["MessageData"]["SurvivabilityRiskLevelID"]["UUID"] =
            json!("32345678-1234-1234-8234-123456789abc");
        external_risk["MessageData"]["RiskSetting"][1]["RiskLevel"][0]["MaximumContinuousExposure"] =
            json!({"CompositeExposureTime":"PT1S", "CumulativeExposureTime":"PT2S"});
        let mut external_dlz = dlz.clone();
        external_dlz["MessageData"]["DLZ_ID"]["UUID"] =
            json!("42345678-1234-1234-8234-123456789abc");
        // The focused Float32 case from RUST-OMS-006, not its complete corpus.
        external_dlz["MessageData"]["DLZ_Data"]["SteeringUnitVector"]["LOS_UnitVector"] =
            json!({"X":0.5000000596046448,"Y":-0.0,"Z":1.401298464324817e-45});
        for (index, (topic, name, body)) in [
            (LOG, "Log", &external_log),
            (RDMA, "RDMA_InitializeSetup", &external_rdma),
            (RISK, "SurvivabilityRiskLevel", &external_risk),
            (DLZ, "DLZ", &external_dlz),
        ]
        .into_iter()
        .enumerate()
        {
            executor
                .block_on(raw_publish(&mut publisher, topic, &json!({name: body})))
                .unwrap();
            let message = executor.block_on(async {
                tokio::time::timeout(WAIT, observer.recv())
                    .await
                    .unwrap()
                    .unwrap()
            });
            assert_eq!(
                serde_json::from_str::<Value>(&message.payload).unwrap(),
                json!({name: body})
            );
            assert_eq!(message.sid, format!("wire-{index}"));
            eprintln!(
                "RUST-OMS-007 {VERSION} external {name} OWP MSG {}: {}",
                message.sid, message.payload
            );
        }
        app.until("label=\"External007\"");
        app.until("setup=22345678-1234-1234-8234-123456789abc");
        app.until("id=32345678-1234-1234-8234-123456789abc");
        app.until("id=42345678-1234-1234-8234-123456789abc");
        let mut malformed = log.clone();
        malformed["MessageData"]
            .as_object_mut()
            .unwrap()
            .remove("Severity");
        let error = executor
            .block_on(raw_publish(&mut publisher, LOG, &json!({"Log": malformed})))
            .unwrap_err();
        assert!(matches!(error, ClientError::Server { .. }));
        assert!(
            executor
                .block_on(async { tokio::time::timeout(QUIET, observer.recv()).await })
                .is_err(),
            "rejected message must not reach independent witness"
        );
        eprintln!(
            "RUST-OMS-007 {VERSION} independent publication rejected by broker (no MSG): {error}"
        );
        app.finish(true);
        assert_eq!(app.output.matches("RECEIVED Log ").count(), 2);
        assert_eq!(
            app.output.matches("RECEIVED RDMA_InitializeSetup ").count(),
            2
        );
        assert_eq!(
            app.output
                .matches("RECEIVED SurvivabilityRiskLevel ")
                .count(),
            2
        );
        assert_eq!(app.output.matches("RECEIVED DLZ ").count(), 2);
        let risk_line = app
            .output
            .lines()
            .find(|line| line.contains("id=32345678"))
            .unwrap();
        for field in [
            "settings=2 risk-levels=2",
            "riskprioritization: NONE",
            "riskprioritization: HIGH",
            "allowedrisk: NONE",
            "allowedrisk: ALL",
            "maximumcontinuousexposure: None",
            "compositeexposuretime: DurationType { lexical: \"PT1S\" }",
            "cumulativeexposuretime: DurationType { lexical: \"PT2S\" }",
        ] {
            assert!(
                risk_line.contains(field),
                "missing typed nested field {field}: {risk_line}"
            );
        }
        for line in app
            .output
            .lines()
            .filter(|line| line.starts_with("RECEIVED DLZ "))
        {
            for field in [
                "type=DLZSTAPLE",
                "time-of-intercept=12:34:56.5000Z",
                "range-poi-bits=3ff0000000000001",
                "loft-bits=Some(8000000000000000)",
            ] {
                assert!(
                    line.contains(field),
                    "missing typed DLZ field {field}: {line}"
                );
            }
        }
        assert!(
            app.output
                .contains("vector-bits=Some([3f000000, bf000000, 3f800000])")
        );
        assert!(
            app.output
                .contains("vector-bits=Some([3f000001, 80000000, 00000001])")
        );
        app.assert_cleanup();
        for marker in [
            "severity=INFO",
            "provider=TCP endpoints=2",
            "UNSUBSCRIBED Log",
            "UNSUBSCRIBED RDMA_InitializeSetup",
            "CLOSED runtime; worker joined",
            "COMPLETE bounded execution",
        ] {
            assert!(
                app.output.contains(marker),
                "missing {marker}: {}",
                app.output
            );
        }
        assert!(!app.output.contains("ERROR:"));
        // Real executable errors, not unit-only parser checks.
        let bad_file = server.directory.join("bad.json");
        for (contents, diagnostic) in [
            ("{", "invalid input JSON"),
            ("{}", "generated codec rejected"),
        ] {
            std::fs::write(&bad_file, contents).unwrap();
            let mut bad = Application::start(&url, &["--publish-log", bad_file.to_str().unwrap()]);
            bad.finish(false);
            assert!(bad.output.contains(diagnostic));
            assert!(!bad.output.contains("CONNECTED"));
        }
        // The fourth bad file must stop the first three publications too.
        // A listening TCP socket independently proves no connection attempt.
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let offline_url = format!("ws://{}", listener.local_addr().unwrap());
        let mut invalid_time = dlz.clone();
        invalid_time["MessageData"]["DLZ_Data"]["TimeOfIntercept"] = json!("not-a-timeZ");
        for (family, body, diagnostic) in [
            (2, json!({}), "generated codec rejected"),
            (3, invalid_time, "TimeOfIntercept"),
        ]
        .into_iter()
        .chain((0..4).map(|family| {
            let mut body = [&log, &rdma, &risk, &dlz][family].clone();
            body["MessageHeader"]["SchemaVersion"] = json!(if VERSION == "002.5.0" {
                "002.6.0"
            } else {
                "002.5.0"
            });
            (family, body, "does not match build-selected")
        })) {
            std::fs::write(&bad_file, body.to_string()).unwrap();
            let mut selected = files;
            selected[family] = &bad_file;
            let args: Vec<_> = FLAGS
                .iter()
                .zip(selected)
                .flat_map(|(flag, file)| [*flag, file.to_str().unwrap()])
                .collect();
            let mut bad = Application::start(&offline_url, &args);
            bad.finish(false);
            assert!(bad.output.contains(diagnostic), "{}", bad.output);
            assert!(!bad.output.contains("CONNECTED") && !bad.output.contains("PUBLISHED"));
            assert_eq!(
                listener.accept().unwrap_err().kind(),
                std::io::ErrorKind::WouldBlock
            );
        }
        std::fs::remove_file(&bad_file).unwrap();
        // Permission rejection must surface asynchronously as ServerError,
        // never as an acknowledged successful subscription/publication.
        server.shutdown();
        drop(server);
        let (mut denied_server, denied_url) = start_sleet_with_topics(&[LOG]);
        connect(&denied_url).close().unwrap();
        let mut denied = Application::start(&denied_url, &args);
        denied.finish(false);
        assert!(denied.output.contains("ServerError"), "{}", denied.output);
        assert!(denied.output.contains("CLOSED runtime; worker joined"));
        denied.assert_cleanup();
        denied_server.shutdown();
        drop(denied_server);
        // Pinned Sleet admits malformed hexBinary; the generated decoder does
        // not. Independently witness that frame, then require the application's
        // SubscriptionDecodeError with no corresponding typed delivery.
        for (topic, name, body, field, diagnostic) in [
            (LOG, "Log", &log, "Data", "Data"),
            (DLZ, "DLZ", &dlz, "TimeOfIntercept", "TimeOfIntercept"),
        ] {
            let (mut server, decoder_url) = start_sleet_with_topics(&TOPICS);
            connect(&decoder_url).close().unwrap();
            let mut decoder_publisher = executor.block_on(raw_connect(&decoder_url));
            executor
                .block_on(decoder_publisher.subscribe("bad-wire", name, topic, None))
                .unwrap();
            let family = if name == "Log" { 0 } else { 3 };
            let mut decoder = Application::start(
                &decoder_url,
                &[
                    FLAGS[family],
                    files[family].to_str().unwrap(),
                    "--reconnect-attempts",
                    "8",
                ],
            );
            // SUB is non-verbose: its local frame write is not an admission
            // barrier. A real self-echo proves this subscription is active.
            decoder.until(&format!("RECEIVED {name} "));
            executor.block_on(async {
                tokio::time::timeout(WAIT, decoder_publisher.recv())
                    .await
                    .unwrap()
                    .unwrap()
            });
            let ids = decoder.subscription_ids();
            let mut malformed = body.clone();
            if name == "Log" {
                malformed["MessageData"][field] = json!("0G");
            } else {
                malformed["MessageData"]["DLZ_Data"][field] = json!("not-a-timeZ");
            }
            let document = json!({name: malformed});
            executor
                .block_on(raw_publish(&mut decoder_publisher, topic, &document))
                .unwrap();
            let observed = executor.block_on(async {
                tokio::time::timeout(WAIT, decoder_publisher.recv())
                    .await
                    .unwrap()
                    .unwrap()
            });
            assert_eq!(
                serde_json::from_str::<Value>(&observed.payload).unwrap(),
                document
            );
            decoder.finish(false);
            assert!(decoder.output.contains("SubscriptionDecodeError"));
            let sid = &ids[family];
            assert!(
                decoder
                    .output
                    .contains(&format!("sid={sid} message={name}"))
            );
            assert!(decoder.output.contains(diagnostic));
            assert!(decoder.output.contains("no typed delivery"));
            assert!(!decoder.output.contains("RECONNECT_ATTEMPT"));
            assert_eq!(
                decoder.output.matches("RECEIVED ").count(),
                1,
                "only the valid barrier message has typed delivery"
            );
            decoder.assert_cleanup();
            eprintln!(
                "RUST-OMS-007 {VERSION} broker-admitted {name} witnessed; contextual decode error and four-subscription cleanup verified"
            );
            drop(decoder_publisher);
            server.shutdown();
            drop(server);
        }
        // Established connection closure also exits nonzero and joins worker.
        let (mut server, closure_url) = start_sleet_with_topics(&TOPICS);
        connect(&closure_url).close().unwrap();
        let mut closed = Application::start(&closure_url, &[]);
        closed.until("SUBSCRIBED");
        server.shutdown();
        closed.finish(false);
        assert!(
            closed.output.contains("ConnectionClosed"),
            "{}",
            closed.output
        );
        assert!(closed.output.contains("CLOSED runtime; worker joined"));
        assert!(!closed.output.contains("RECONNECT_ATTEMPT"));
        drop(observer);
        drop(publisher);
        drop(executor);
        assert_eq!(connections(&url), 0);
        // close() and the production lifecycle regressions prove worker joining;
        // do not race TLS teardown with immediate procfs disappearance checks.
        eprintln!(
            "RUST-OMS-007 {VERSION} standalone four-message typed/external/OWP/error/unsubscribe/process cleanup: PASSED"
        );
    }
}

struct Application {
    child: Child,
    lines: mpsc::Receiver<String>,
    reader: Option<std::thread::JoinHandle<()>>,
    output: String,
}

impl Application {
    fn subscription_ids(&self) -> Vec<String> {
        self.output
            .lines()
            .find(|line| line.starts_with("SUBSCRIBED "))
            .unwrap()
            .split_whitespace()
            .filter_map(|word| word.strip_prefix("sid=").map(str::to_owned))
            .collect()
    }

    fn assert_cleanup(&self) {
        for name in MESSAGES {
            assert!(
                self.output.contains(&format!(
                    "UNSUBSCRIBED {name}: frame written; handler removed"
                )),
                "{}",
                self.output
            );
        }
        assert!(
            self.output.contains("CLOSED runtime; worker joined"),
            "{}",
            self.output
        );
    }

    fn start(url: &str, extra: &[&str]) -> Self {
        Self::start_for(url, extra, "4")
    }

    fn start_for(url: &str, extra: &[&str], seconds: &str) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_rust-uci-client"))
            .args([
                "--url",
                url,
                "--service-id",
                TOPIC,
                "--duration-seconds",
                seconds,
            ])
            .args(extra)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let stdout = child.stdout.take().unwrap();
        let (sender, lines) = mpsc::channel();
        let reader = std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                if sender.send(line.unwrap()).is_err() {
                    break;
                }
            }
        });
        Self {
            child,
            lines,
            reader: Some(reader),
            output: String::new(),
        }
    }

    fn until(&mut self, marker: &str) {
        let deadline = Instant::now() + WAIT;
        loop {
            let line = self
                .lines
                .recv_timeout(deadline.saturating_duration_since(Instant::now()))
                .expect("application output deadline");
            self.output.push_str(&line);
            self.output.push('\n');
            if line.contains(marker) {
                break;
            }
        }
    }

    fn finish(&mut self, success: bool) {
        let deadline = Instant::now() + WAIT;
        let status = loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                break status;
            }
            assert!(Instant::now() < deadline, "application did not terminate");
            std::thread::sleep(Duration::from_millis(20));
        };
        self.reader.take().unwrap().join().unwrap();
        for line in self.lines.try_iter() {
            self.output.push_str(&line);
            self.output.push('\n');
        }
        use std::io::Read;
        self.child
            .stderr
            .take()
            .unwrap()
            .read_to_string(&mut self.output)
            .unwrap();
        eprintln!("{}", self.output);
        assert_eq!(status.success(), success, "{}", self.output);
        assert!(
            !PathBuf::from(format!("/proc/{}", self.child.id())).exists(),
            "application process and worker tasks reaped"
        );
    }
}

fn application_connections(pid: u32, url: &str) -> usize {
    let port: u16 = url.rsplit(':').next().unwrap().parse().unwrap();
    let sockets: BTreeSet<_> = std::fs::read_dir(format!("/proc/{pid}/fd"))
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

impl Drop for Application {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        if let Some(reader) = self.reader.take() {
            let _ = reader.join();
        }
    }
}

#[test]
#[ignore = "requires unmodified pinned Sleet; run via scripts/run-real-sleet-test.sh"]
fn standalone_real_sleet() {
    assert!(
        std::env::var_os("AMS_GRA_SLEET_BIN").is_some(),
        "pinned Sleet binary required"
    );
    support::accept();
    support::reconnection();
}
