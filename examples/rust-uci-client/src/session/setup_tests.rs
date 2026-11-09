//! Deterministic loss before fifth registration uses the actual session cleanup.
use super::*;
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::{Message, handshake::server};

fn workers() -> std::collections::BTreeSet<std::path::PathBuf> {
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

#[test]
fn fifth_registration_failure_joins_partial_session() {
    const WAIT: Duration = Duration::from_secs(10);
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("ws://{}", listener.local_addr().unwrap());
    listener.set_nonblocking(true).unwrap();
    let (stop, stopped) = mpsc::channel();
    let peer = std::thread::spawn(move || {
        tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
            tokio::time::timeout(WAIT, async {
                let listener = tokio::net::TcpListener::from_std(listener).unwrap();
                let (stream, _) = listener.accept().await.unwrap();
                #[allow(clippy::result_large_err)]
                let callback = |_: &server::Request, mut response: server::Response| {
                    response.headers_mut().insert("Sec-WebSocket-Protocol", "owp".parse().unwrap());
                    Ok(response)
                };
                let mut ws = tokio_tungstenite::accept_hdr_async(stream, callback).await.unwrap();
                assert!(ws.next().await.unwrap().unwrap().into_text().unwrap().starts_with("INIT "));
                ws.send(Message::Text(r#"INFO {"version":"1.0","server_id":"mock","uuids":{"system":"s","service":"v"},"system_label":"mock"}"#.into())).await.unwrap();
                for topic in ["rust-oms-003-log", "rust-oms-003-rdma", "rust-oms-004-survivability", "task098-dlz"] {
                    let frame = ws.next().await.unwrap().unwrap().into_text().unwrap();
                    assert!(frame.starts_with("SUB ") && frame.ends_with(topic), "{frame}");
                }
                stopped.recv_timeout(WAIT).unwrap();
                // All four earlier SUB frames are established; force real loss
                // while the application barrier prevents the fifth command.
                drop(ws);
                assert!(matches!(listener.into_std().unwrap().accept(), Err(error) if error.kind() == ErrorKind::WouldBlock));
            }).await.unwrap();
        });
    });
    let options = cli::Options::parse(
        [
            "--url",
            &url,
            "--service-id",
            "test",
            "--enable-file-metadata",
        ]
        .map(str::to_owned),
    )
    .unwrap()
    .unwrap();
    let before = workers();
    let runtime = SleetRuntime::connect(config(&options, WAIT), ServiceCodec).unwrap();
    let owned: Vec<_> = workers().difference(&before).cloned().collect();
    assert_eq!(owned.len(), 1);
    let result = Session::subscribe_with_metadata_barrier(runtime, &options, 1, |session| {
        assert_eq!(session.subscriptions.len(), 4);
        stop.send(()).unwrap();
        assert!(matches!(
            session.runtime.recv_event_timeout(WAIT),
            Some(RuntimeEvent::ConnectionClosed { .. })
        ));
    });
    let error = result.err().expect("fifth registration must fail");
    assert!(error.contains("subscribe FileMetadata:"), "{error}");
    // close joins synchronously, even when all earlier UNSUB writes fail.
    assert!(error.contains("unsubscribe Log:"), "{error}");
    assert!(error.contains("unsubscribe DLZ:"), "{error}");
    peer.join().unwrap();
    let deadline = Instant::now() + WAIT;
    while owned.iter().any(|path| path.exists()) {
        assert!(
            Instant::now() < deadline,
            "partial-session worker survived cleanup"
        );
        std::thread::yield_now();
    }
}
