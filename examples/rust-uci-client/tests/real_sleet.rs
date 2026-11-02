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
const WAIT: Duration = Duration::from_secs(10);
const QUIET: Duration = Duration::from_millis(200);

// Reuse existing isolated broker configuration, external CalClient and Linux
// socket/worker witnesses. This imports source helpers, never the facade crate.
#[allow(dead_code)]
mod support {
    use super::*;
    include!("../../../crates/runtime-rust-facade-tests/tests/real_sleet_support/mod.rs");
    include!("../../../crates/runtime-rust-facade-tests/tests/real_sleet_support/connections.rs");

    pub fn accept() {
        let (mut server, url) = start_sleet_with_topics(&[LOG, RDMA]);
        // Startup probe also verifies the configured schema/identity handshake.
        connect(&url).close().unwrap();
        let executor = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let mut observer = executor.block_on(raw_connect(&url));
        let mut publisher = executor.block_on(raw_connect(&url));
        executor.block_on(async {
            observer
                .subscribe("wire-log", "Log", LOG, None)
                .await
                .unwrap();
            observer
                .subscribe("wire-rdma", "RDMA_InitializeSetup", RDMA, None)
                .await
                .unwrap();
        });
        let fixture =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/service-generate");
        let suffix = env!("AMS_GRA_UCI_RELEASE_BUILD").replace('.', "");
        let log_file = fixture.join(format!("task093-real-log{suffix}.json"));
        let rdma_file = fixture.join(format!("task092-real-rdma{suffix}.json"));
        let log: Value = serde_json::from_slice(&std::fs::read(&log_file).unwrap()).unwrap();
        let rdma: Value = serde_json::from_slice(&std::fs::read(&rdma_file).unwrap()).unwrap();
        let baseline_workers = workers();
        let mut app = Application::start(
            &url,
            &[
                "--publish-log",
                log_file.to_str().unwrap(),
                "--publish-rdma",
                rdma_file.to_str().unwrap(),
            ],
        );
        app.until("SUBSCRIBED");
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
            "one production runtime worker serves both subscriptions"
        );
        for (sid, name, document) in [
            ("wire-log", "Log", &log),
            ("wire-rdma", "RDMA_InitializeSetup", &rdma),
        ] {
            let message = executor.block_on(async {
                tokio::time::timeout(WAIT, observer.recv())
                    .await
                    .unwrap()
                    .unwrap()
            });
            assert_eq!(message.sid, sid);
            assert_eq!(
                serde_json::from_str::<Value>(&message.payload).unwrap(),
                json!({name: document})
            );
            eprintln!(
                "RUST-OMS-005 {VERSION} independent OWP MSG {sid}: {}",
                message.payload
            );
        }
        app.until("RECEIVED RDMA_InitializeSetup");
        assert_eq!(
            connections(&url),
            2,
            "witness process owns only its two independent sockets"
        );
        // Label/UUID changes distinguish external publication from self echo.
        let mut external_log = log.clone();
        external_log["MessageData"]["Label"] = json!("External005");
        let mut external_rdma = rdma.clone();
        external_rdma["MessageData"]["RDMA_InitializeSetupID"]["UUID"] =
            json!("22345678-1234-1234-8234-123456789abc");
        for (topic, name, body) in [
            (LOG, "Log", &external_log),
            (RDMA, "RDMA_InitializeSetup", &external_rdma),
        ] {
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
        }
        app.until("label=\"External005\"");
        app.until("setup=22345678-1234-1234-8234-123456789abc");
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
        eprintln!("RUST-OMS-005 {VERSION} malformed external input rejected: {error}");
        app.finish(true);
        assert_eq!(app.output.matches("RECEIVED Log ").count(), 2);
        assert_eq!(
            app.output.matches("RECEIVED RDMA_InitializeSetup ").count(),
            2
        );
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
        // Permission rejection must surface asynchronously as ServerError,
        // never as an acknowledged successful subscription/publication.
        server.shutdown();
        drop(server);
        let (mut denied_server, denied_url) = start_sleet_with_topics(&[LOG]);
        connect(&denied_url).close().unwrap();
        let mut denied = Application::start(&denied_url, &[]);
        denied.finish(false);
        assert!(denied.output.contains("ServerError"), "{}", denied.output);
        assert!(denied.output.contains("CLOSED runtime; worker joined"));
        denied_server.shutdown();
        drop(denied_server);
        // Pinned Sleet admits malformed hexBinary; the generated decoder does
        // not. Independently witness that frame, then require the application's
        // SubscriptionDecodeError with no corresponding typed delivery.
        let (mut server, decoder_url) = start_sleet_with_topics(&[LOG, RDMA]);
        connect(&decoder_url).close().unwrap();
        let mut decoder_publisher = executor.block_on(raw_connect(&decoder_url));
        executor
            .block_on(decoder_publisher.subscribe("bad-wire", "Log", LOG, None))
            .unwrap();
        let mut decoder = Application::start(&decoder_url, &[]);
        decoder.until("SUBSCRIBED");
        let mut bad_binary = log.clone();
        bad_binary["MessageData"]["Data"] = json!("0G");
        let document = json!({"Log": bad_binary});
        executor
            .block_on(raw_publish(&mut decoder_publisher, LOG, &document))
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
        assert!(decoder.output.contains("no typed delivery"));
        assert!(!decoder.output.contains("RECEIVED"));
        drop(decoder_publisher);
        server.shutdown();
        drop(server);
        // Established connection closure also exits nonzero and joins worker.
        let (mut server, closure_url) = start_sleet_with_topics(&[LOG, RDMA]);
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
        drop(observer);
        drop(publisher);
        drop(executor);
        assert_eq!(connections(&url), 0);
        assert_eq!(workers(), baseline_workers);
        eprintln!(
            "RUST-OMS-005 {VERSION} standalone typed/external/OWP/error/unsubscribe/process cleanup: PASSED"
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
    fn start(url: &str, extra: &[&str]) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_rust-uci-client"))
            .args([
                "--url",
                url,
                "--service-id",
                TOPIC,
                "--duration-seconds",
                "4",
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
}
