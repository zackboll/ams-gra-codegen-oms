//! PR #50 corrective: the worker must keep receiving while its command
//! queue is CONTINUOUSLY ready.
//!
//! The public API cannot hold the queue full on its own (each adapter call
//! takes `&mut SleetRuntime` and blocks for its reply), so this test drives
//! the crate-private [`Worker`] directly: several flooder threads keep the
//! bounded queue at capacity without waiting for replies, which is the
//! situation many concurrent callers create. Under the original `biased`
//! select (commands before `recv`) the `MSG` below was never dispatched and
//! this test failed at its bounded timeout (`Err(Timeout)`, reproduced at
//! reviewed head 168782d with the corrected test). A single flooder is not
//! enough: the queue briefly drains between its sends and `recv` gets in.

use super::{COMMAND_CAPACITY, Command, Subscription, Worker};
use crate::RuntimeEvent;
use futures_util::{SinkExt, StreamExt};
use sleet_client::{CalClient, InitOptions};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, mpsc as std_mpsc};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, oneshot};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::handshake::server::{ErrorResponse, Request, Response};
use tokio_tungstenite::tungstenite::http::HeaderValue;

const WAIT: Duration = Duration::from_secs(5);

/// Threads keeping the bounded queue at capacity.
const FLOODERS: usize = 4;

/// PUB frames the peer must see before it sends the probes, so the flood
/// is demonstrably established first.
const FLOOD_BEFORE_PROBE: u64 = 500;

const INFO_FRAME: &str = concat!(
    r#"INFO {"version":"1.0","server_id":"mock","#,
    r#""uuids":{"system":"s","service":"v"},"system_label":"mock"}"#
);
const MSG_FRAME: &str = r#"MSG sub-1 {"{urn:test}MessageA":{"Count":42}}"#;
const ERR_FRAME: &str = "-ERR Illegal-State synthetic late error";

/// Test-only OWP peer: answers `INIT`, counts `PUB` frames, and once the
/// flood is established sends one `MSG` and one `-ERR`.
fn start_peer(pubs: Arc<AtomicU64>) -> (String, JoinHandle<()>) {
    let (address_tx, address_rx) = std_mpsc::channel();
    let peer = thread::spawn(move || {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("peer runtime")
            .block_on(async move {
                let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
                    .await
                    .expect("bind");
                address_tx
                    .send(listener.local_addr().expect("address"))
                    .expect("report address");
                let (stream, _) = listener.accept().await.expect("accept");
                #[allow(clippy::result_large_err)]
                let callback =
                    |_: &Request, mut response: Response| -> Result<Response, ErrorResponse> {
                        response
                            .headers_mut()
                            .insert("Sec-WebSocket-Protocol", HeaderValue::from_static("owp"));
                        Ok(response)
                    };
                let mut ws = tokio_tungstenite::accept_hdr_async(stream, callback)
                    .await
                    .expect("upgrade");
                let mut probed = false;
                while let Some(Ok(frame)) = ws.next().await {
                    let Message::Text(text) = frame else { continue };
                    if text.starts_with("INIT ") {
                        ws.send(Message::Text(INFO_FRAME.into()))
                            .await
                            .expect("INFO");
                    } else if text.starts_with("PUB ") {
                        let seen = pubs.fetch_add(1, Ordering::Relaxed) + 1;
                        if !probed && seen >= FLOOD_BEFORE_PROBE {
                            probed = true;
                            ws.send(Message::Text(MSG_FRAME.into())).await.expect("MSG");
                            ws.send(Message::Text(ERR_FRAME.into()))
                                .await
                                .expect("-ERR");
                        }
                    }
                }
            });
    });
    let address = address_rx.recv_timeout(WAIT).expect("peer address");
    (format!("ws://{address}"), peer)
}

/// Keep the bounded command queue full: `blocking_send` parks only while
/// the queue is at capacity, so the worker's command branch is ready on
/// every loop iteration. Replies are dropped (the worker ignores that).
fn start_flooder(
    commands: mpsc::Sender<Command>,
    stop: Arc<AtomicBool>,
    queued: Arc<AtomicU64>,
) -> JoinHandle<()> {
    thread::spawn(move || {
        while !stop.load(Ordering::Relaxed) {
            let (reply, _) = oneshot::channel();
            let message = serde_json::json!({ "{urn:test}MessageB": { "Count": 7 } });
            let command = Command::Publish {
                topic: "output-topic",
                message,
                reply,
            };
            if commands.blocking_send(command).is_err() {
                return;
            }
            queued.fetch_add(1, Ordering::Relaxed);
        }
    })
}

#[test]
fn worker_receives_while_command_queue_is_continuously_ready() {
    let pubs = Arc::new(AtomicU64::new(0));
    let (url, peer) = start_peer(Arc::clone(&pubs));

    let (commands, command_rx) = mpsc::channel(COMMAND_CAPACITY);
    let (shutdown, shutdown_rx) = oneshot::channel();
    let (event_tx, events) = std_mpsc::channel();
    let (seen_tx, seen) = std_mpsc::channel::<String>();
    let worker = thread::spawn(move || {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("worker runtime")
            .block_on(async move {
                let options = InitOptions {
                    versions: vec!["1.0".to_owned()],
                    schema: "000.1.0".to_owned(),
                    verbose: false,
                    ..InitOptions::default()
                };
                let client = CalClient::connect_with_options(&url, "svc-1", options)
                    .await
                    .expect("connect");
                let mut worker = Worker::new(client, command_rx, shutdown_rx, event_tx);
                worker.subscriptions.insert(
                    "sub-1".to_owned(),
                    Subscription {
                        message_name: "{urn:test}MessageA".to_owned(),
                        dispatch: Box::new(move |raw: &str| {
                            let _ = seen_tx.send(raw.to_owned());
                            Ok(())
                        }),
                    },
                );
                worker.run().await;
            });
    });

    let stop = Arc::new(AtomicBool::new(false));
    let queued = Arc::new(AtomicU64::new(0));
    let flooders: Vec<_> = (0..FLOODERS)
        .map(|_| start_flooder(commands.clone(), Arc::clone(&stop), Arc::clone(&queued)))
        .collect();
    drop(commands);

    // Collect the observations while the flood is still running, then tear
    // down unconditionally, then assert, so a regression fails cleanly.
    let delivered = seen.recv_timeout(WAIT);
    let event = events.recv_timeout(WAIT);
    let flooding_throughout = flooders.iter().all(|flooder| !flooder.is_finished());
    let queued_at_probe = queued.load(Ordering::Relaxed);

    let started = Instant::now();
    stop.store(true, Ordering::Relaxed);
    let _ = shutdown.send(());
    for flooder in flooders {
        flooder.join().expect("flooder");
    }
    worker.join().expect("worker thread exits and is joined");
    let shutdown_took = started.elapsed();
    peer.join().expect("peer sees the connection end");

    assert_eq!(
        delivered.as_deref(),
        Ok(r#"{"{urn:test}MessageA":{"Count":42}}"#),
        "MSG was not dispatched while the command queue stayed ready"
    );
    match event {
        Ok(RuntimeEvent::ServerError { error, details }) => {
            assert_eq!(error, "Illegal-State");
            assert_eq!(details.as_deref(), Some("synthetic late error"));
        }
        other => panic!("late -ERR not surfaced under command load: {other:?}"),
    }
    assert!(flooding_throughout, "the flood must outlive both probes");
    assert!(pubs.load(Ordering::Relaxed) >= FLOOD_BEFORE_PROBE);
    assert!(queued_at_probe > FLOOD_BEFORE_PROBE);
    assert!(shutdown_took < WAIT, "shutdown under load must be prompt");
}
