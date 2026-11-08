//! Exercise the real session owner and cleanup with a deterministic final decision.
use super::*;
use futures_util::{SinkExt, StreamExt};
use std::collections::BTreeSet;
use std::process::Command;
use tokio_tungstenite::tungstenite::{Message, handshake::server};

const WAIT: Duration = Duration::from_secs(10);
const CHILD: &str = "session::deadline_tests::activation_child";

fn workers() -> BTreeSet<String> {
    std::fs::read_dir("/proc/self/task")
        .unwrap()
        .map(Result::unwrap)
        .filter(|entry| {
            std::fs::read_to_string(entry.path().join("comm"))
                .unwrap_or_default()
                .starts_with("ams-gra-oms-lac")
        })
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect()
}

#[test]
fn post_subscription_deadline_boundaries() {
    for count in [4, 5] {
        for mode in ["at", "after", "before", "cleanup-error"] {
            let output = Command::new(std::env::current_exe().unwrap())
                .args(["--exact", CHILD, "--nocapture"])
                // Test-harness subprocess only; the production executable has no hook.
                .env("RUST_OMS_008_TEST_BOUNDARY", mode)
                .env("RUST_OMS_011_TEST_SUBSCRIPTIONS", count.to_string())
                .output()
                .unwrap();
            let text = format!(
                "{}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            println!("boundary={mode}\n{text}");
            assert_eq!(output.status.success(), mode == "before", "{text}");
            assert!(
                text.contains("CONNECTED session=2"),
                "INIT succeeded: {text}"
            );
            assert!(text.contains("SUBSCRIBED session=2"), "{text}");
            assert_eq!(text.matches("RECONNECT_ATTEMPT").count(), 1, "{text}");
            assert_eq!(
                text.matches("RECONNECT_COMPLETE").count(),
                usize::from(mode == "before"),
                "{text}"
            );
            assert!(
                !text.contains("RECEIVED "),
                "queued old value was presented: {text}"
            );
            assert!(!text.contains("PUBLISHED "), "publication replay: {text}");
            assert!(
                text.contains(&format!(
                    "PEER connections=2 INIT=2 SUB={} PUB=1",
                    count * 2
                )),
                "{text}"
            );
            assert!(
                text.contains("replacement worker gone; typed queue dropped"),
                "{text}"
            );
            if mode != "before" {
                assert!(text.contains("ERROR: ConnectionClosed:"), "{text}");
                assert!(
                    text.contains("execution deadline expired during subscription setup"),
                    "{text}"
                );
            }
            if mode == "cleanup-error" {
                assert!(
                    text.contains("unsubscribe Log:"),
                    "cleanup error lost: {text}"
                );
            } else {
                assert!(
                    text.contains("CLOSED runtime; worker joined; session=2"),
                    "{text}"
                );
                assert_eq!(
                    text.matches("handler removed; session=2").count(),
                    count,
                    "{text}"
                );
            }
        }
    }
}

#[test]
fn activation_child() {
    let Ok(mode) = std::env::var("RUST_OMS_008_TEST_BOUNDARY") else {
        return;
    };
    let count: usize = std::env::var("RUST_OMS_011_TEST_SUBSCRIPTIONS")
        .unwrap()
        .parse()
        .unwrap();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("ws://{}", listener.local_addr().unwrap());
    listener.set_nonblocking(true).unwrap();
    let (ready_tx, ready) = mpsc::channel();
    let (done_tx, done) = mpsc::channel();
    let (close_tx, close_rx) = mpsc::channel();
    let cleanup_error = mode == "cleanup-error";
    let peer = std::thread::spawn(move || {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async move {
                tokio::time::timeout(WAIT * 3, async move {
                    let listener = tokio::net::TcpListener::from_std(listener).unwrap();
                    let mut frames = Vec::new();
                    for epoch in 1..=2 {
                        let (stream, _) = listener.accept().await.unwrap();
                        #[allow(clippy::result_large_err)]
                        let callback = |_: &server::Request, mut response: server::Response| {
                            response.headers_mut().insert("Sec-WebSocket-Protocol", "owp".parse().unwrap());
                            Ok(response)
                        };
                        let mut ws = tokio_tungstenite::accept_hdr_async(stream, callback).await.unwrap();
                        let mut subs = 0;
                        let mut unsubs = 0;
                        while let Some(Ok(frame)) = ws.next().await {
                            let Message::Text(text) = frame else { break };
                            frames.push(text.to_string());
                            if text.starts_with("INIT ") {
                                ws.send(Message::Text(r#"INFO {"version":"1.0","server_id":"mock","uuids":{"system":"s","service":"v"},"system_label":"mock"}"#.into())).await.unwrap();
                            } else if text.starts_with("SUB ") {
                                subs += 1;
                                if epoch == 2 && subs == count {
                                    ready_tx.send(()).unwrap();
                                    if cleanup_error {
                                        close_rx.recv_timeout(WAIT).unwrap();
                                        break;
                                    }
                                }
                            } else if text.starts_with("PUB ") && epoch == 1 {
                                assert_eq!(subs, count);
                                break; // Original ConnectionClosed, after initial-only publication.
                            } else if text.starts_with("UNSUB ") {
                                unsubs += 1;
                            }
                        }
                        if epoch == 2 && !cleanup_error { assert_eq!(unsubs, count); }
                    }
                    // No third connection is even queued once the owner returns.
                    done.recv_timeout(WAIT).unwrap();
                    assert!(matches!(listener.into_std().unwrap().accept(), Err(error) if error.kind() == ErrorKind::WouldBlock));
                    assert_eq!(frames.iter().filter(|frame| frame.starts_with("INIT ")).count(), 2);
                    assert_eq!(frames.iter().filter(|frame| frame.starts_with("SUB ")).count(), count * 2);
                    assert_eq!(frames.iter().filter(|frame| frame.starts_with("PUB ")).count(), 1);
                    println!("PEER connections=2 INIT=2 SUB={} PUB=1", count * 2);
                }).await.unwrap();
            });
    });
    let mut options = cli::Options::parse(
        [
            "--url",
            &url,
            "--service-id",
            "test",
            "--reconnect-attempts",
            "2",
            "--reconnect-delay-ms",
            "50",
        ]
        .map(str::to_owned),
    )
    .unwrap()
    .unwrap();
    options.enable_file_metadata = count == 5;
    let mut session = Session::connect(&options, 1, WAIT).unwrap();
    let old_workers = workers();
    assert_eq!(old_workers.len(), 1);
    // Leave an actual typed value pending in the old owner's queue. It must
    // be dropped, not drained or relabelled as a replacement receipt.
    let fixture = format!(
        "{}/../../tests/fixtures/service-generate/task093-real-log{}.json",
        env!("CARGO_MANIFEST_DIR"),
        env!("AMS_GRA_UCI_RELEASE_BUILD").replace('.', "")
    );
    let value: m::LogMT = load(Path::new(&fixture)).unwrap();
    let (old_queue, receiver) = mpsc::channel();
    old_queue
        .send((1, Received::Log(Box::new(value.clone()))))
        .unwrap();
    session.receiver = receiver;
    log::exchange_publish::publish(&mut session.runtime, &value).unwrap();
    // Wait for the loss event without consuming the queued typed value.
    let stopped_by = Instant::now() + WAIT;
    while session.runtime.is_running() {
        assert!(Instant::now() < stopped_by);
        std::thread::yield_now();
    }
    let deadline = Instant::now() + WAIT;
    let mut replacement_worker = BTreeSet::new();
    let mut queue_witness = None;
    let result =
        receive_sessions_with_activation_time(session, &options, deadline, |replacement| {
            ready.recv_timeout(WAIT).unwrap();
            replacement_worker = workers();
            assert_eq!(replacement_worker.len(), 1);
            assert!(
                old_workers.is_disjoint(&replacement_worker),
                "old worker not joined before INIT"
            );
            assert!(
                old_queue
                    .send((1, Received::Log(Box::new(value.clone()))))
                    .is_err(),
                "old queue not dropped"
            );
            assert_eq!(replacement.epoch, 2);
            assert_eq!(replacement.subscriptions.len(), count);
            let (sender, receiver) = mpsc::channel();
            replacement.receiver = receiver;
            queue_witness = Some(sender);
            assert!(replacement.runtime.is_running() || cleanup_error);
            if cleanup_error {
                close_tx.send(()).unwrap();
                assert!(matches!(
                    replacement.runtime.recv_event_timeout(WAIT),
                    Some(RuntimeEvent::ConnectionClosed { .. })
                ));
            }
            match mode.as_str() {
                "before" => deadline - Duration::from_nanos(1),
                "after" => deadline + Duration::from_nanos(1),
                _ => deadline,
            }
        });
    // The owner has returned from JoinHandle::join. Linux can briefly retain
    // the exited task in procfs; bound the OS observation instead of mistaking
    // that lag for a running worker. No production delay or retry is introduced.
    let observed_by = Instant::now() + WAIT;
    while !workers().is_disjoint(&replacement_worker) {
        assert!(
            Instant::now() < observed_by,
            "replacement worker not joined"
        );
        std::thread::yield_now();
    }
    assert!(
        queue_witness
            .unwrap()
            .send((2, Received::Log(Box::new(value))))
            .is_err()
    );
    println!("replacement worker gone; typed queue dropped");
    done_tx.send(()).unwrap();
    peer.join().unwrap();
    match result {
        Ok(()) => assert_eq!(mode, "before"),
        Err(error) => {
            eprintln!("ERROR: {error}");
            std::process::exit(1);
        }
    }
}
