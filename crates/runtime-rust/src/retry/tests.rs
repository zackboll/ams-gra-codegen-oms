use super::*;
use crate::{MAX_INITIAL_CONNECT_ATTEMPTS, RuntimeConfig};
use std::cell::{Cell, RefCell};
use std::future::ready;
use tokio_tungstenite::tungstenite::Error as WsError;

fn refused() -> RuntimeError {
    ClientError::Transport(WsError::Io(IoError::from(ErrorKind::ConnectionRefused))).into()
}

fn config(attempts: u8, initial: Duration, cap: Duration) -> RuntimeConfig {
    RuntimeConfig::new("ws://127.0.0.1:9", "svc", "1")
        .with_initial_connect_retry(attempts, initial, cap)
}

#[test]
fn task070_policy_validation() {
    let ms = Duration::from_millis(1);
    for invalid in [
        config(0, ms, ms),
        config(MAX_INITIAL_CONNECT_ATTEMPTS + 1, ms, ms),
        config(2, Duration::ZERO, ms),
        config(2, ms, Duration::ZERO),
        config(2, Duration::MAX, Duration::MAX),
    ] {
        assert!(matches!(
            invalid.validate(),
            Err(RuntimeError::InvalidConfig(_))
        ));
        assert!(matches!(
            crate::SleetRuntime::connect(invalid, ()),
            Err(RuntimeError::InvalidConfig(_))
        ));
    }
    config(1, Duration::ZERO, Duration::ZERO)
        .validate()
        .unwrap();
    config(8, ms, ms).validate().unwrap();
}

#[test]
fn task070_backoff_progression_and_overflow() {
    let policy =
        *config(8, Duration::from_millis(3), Duration::from_millis(10)).initial_connect_retry();
    assert_eq!(
        (1..=4)
            .map(|n| policy.delay_after(n).unwrap().as_millis())
            .collect::<Vec<_>>(),
        [3, 6, 10, 10]
    );
    assert_eq!(policy.delay_after(8), None);
    let extreme = config(8, Duration::MAX / 2 + Duration::from_secs(1), Duration::MAX);
    assert_eq!(
        extreme.initial_connect_retry().delay_after(2),
        Some(Duration::MAX)
    );
    assert_eq!(
        extreme.initial_connect_retry().delay_after(7),
        Some(Duration::MAX)
    );
}

#[tokio::test(flavor = "current_thread")]
async fn task070_attempt_budget_and_no_final_delay() {
    for cfg in [
        RuntimeConfig::new("ws://h:1", "svc", "1"),
        config(1, Duration::ZERO, Duration::ZERO),
        config(3, Duration::from_millis(2), Duration::from_millis(9)),
    ] {
        let attempts = Cell::new(0);
        let waits = RefCell::new(Vec::new());
        let error = initial_connect::<(), _, _, _, _>(
            cfg.initial_connect_retry(),
            || {
                attempts.set(attempts.get() + 1);
                ready(Err(refused()))
            },
            |wait| {
                waits.borrow_mut().push(wait);
                ready(())
            },
        )
        .await
        .unwrap_err();
        assert!(is_refused(&error));
        assert!(
            error
                .source()
                .unwrap()
                .downcast_ref::<ClientError>()
                .is_some()
        );
        assert_eq!(attempts.get(), cfg.initial_connect_retry().max_attempts());
        assert_eq!(waits.borrow().len(), usize::from(attempts.get() - 1));
        if attempts.get() == 3 {
            assert_eq!(
                *waits.borrow(),
                [Duration::from_millis(2), Duration::from_millis(4)]
            );
        }
    }
}

#[tokio::test(flavor = "current_thread")]
async fn task070_success_stops_attempts() {
    let cfg = config(3, Duration::from_millis(1), Duration::from_millis(2));
    for success_at in [1, 2, 3] {
        let attempts = Cell::new(0);
        let waits = Cell::new(0);
        let result = initial_connect(
            cfg.initial_connect_retry(),
            || {
                attempts.set(attempts.get() + 1);
                ready(if attempts.get() == success_at {
                    Ok(42)
                } else {
                    Err(refused())
                })
            },
            |_| {
                waits.set(waits.get() + 1);
                ready(())
            },
        )
        .await
        .unwrap();
        assert_eq!(result, 42);
        assert_eq!(attempts.get(), success_at);
        assert_eq!(waits.get(), success_at - 1);
    }
}

#[tokio::test(flavor = "current_thread")]
async fn task070_typed_error_classification() {
    assert!(is_refused(&refused()));
    let cfg = config(3, Duration::from_millis(1), Duration::from_millis(2));
    let invalid =
        sleet_client::CalClient::connect_with_options(cfg.url(), "bad service", cfg.init_options())
            .await
            .unwrap_err();
    // Obtain protocol and server errors using the pinned parser via a real
    // client rather than fabricating error text as a transport predicate.
    let mut errors = vec![
        RuntimeError::from(invalid),
        RuntimeError::ConnectTimeout,
        RuntimeError::WorkerSpawn(IoError::from(ErrorKind::ConnectionRefused)),
        RuntimeError::WorkerPanicked,
        ClientError::UnexpectedServerOp("unsupported".to_owned()).into(),
        ClientError::Transport(WsError::Http(Box::new(
            tokio_tungstenite::tungstenite::http::Response::builder()
                .status(403)
                .body(None)
                .unwrap(),
        )))
        .into(),
        ClientError::Transport(WsError::Url(
            tokio_tungstenite::tungstenite::error::UrlError::TlsFeatureNotEnabled,
        ))
        .into(),
    ];
    for kind in [
        ErrorKind::NotFound,
        ErrorKind::TimedOut,
        ErrorKind::ConnectionReset,
        ErrorKind::ConnectionAborted,
        ErrorKind::BrokenPipe,
        ErrorKind::AddrNotAvailable,
        ErrorKind::Other,
    ] {
        errors.push(
            ClientError::Transport(WsError::Io(IoError::new(kind, "connection refused"))).into(),
        );
    }
    for (index, frame) in [
        "-ERR Illegal-State rejected",
        "invalid frame",
        "MSG sub-1 {}",
    ]
    .into_iter()
    .enumerate()
    {
        let error = client_frame_error(frame).await;
        assert!(
            matches!(&error, RuntimeError::Client(client) if match index {
                0 => matches!(client.as_ref(), ClientError::Server { .. }),
                1 => matches!(client.as_ref(), ClientError::ProtocolParse(_)),
                2 => matches!(client.as_ref(), ClientError::UnexpectedServerOp(_)),
                _ => false,
            })
        );
        errors.push(error);
    }
    for error in errors {
        assert!(!is_refused(&error), "{error:?}");
        let mut error = Some(error);
        let attempts = Cell::new(0);
        let _ = initial_connect::<(), _, _, _, _>(
            cfg.initial_connect_retry(),
            || {
                attempts.set(attempts.get() + 1);
                ready(Err(error.take().expect("must not retry")))
            },
            |_| async { panic!("must not delay") },
        )
        .await
        .unwrap_err();
        assert_eq!(attempts.get(), 1);
    }
}

async fn client_frame_error(frame: &'static str) -> RuntimeError {
    use futures_util::{SinkExt, StreamExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("ws://{}", listener.local_addr().unwrap());
    let peer = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut ws = accept_owp(stream).await;
        assert!(
            ws.next()
                .await
                .unwrap()
                .unwrap()
                .into_text()
                .unwrap()
                .starts_with("INIT ")
        );
        ws.send(tokio_tungstenite::tungstenite::Message::Text(frame.into()))
            .await
            .unwrap();
    });
    let result = tokio::time::timeout(
        Duration::from_secs(5),
        sleet_client::CalClient::connect_with_options(
            &url,
            "svc",
            RuntimeConfig::new(&url, "svc", "1").init_options(),
        ),
    )
    .await
    .unwrap()
    .unwrap_err();
    peer.await.unwrap();
    result.into()
}

#[tokio::test(flavor = "current_thread")]
async fn task070_real_refused_then_info() {
    use futures_util::{SinkExt, StreamExt};
    let reserved = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = reserved.local_addr().unwrap();
    drop(reserved);
    let url = format!("ws://{addr}");
    let cfg = RuntimeConfig::new(&url, "svc", "1").with_initial_connect_retry(
        2,
        Duration::from_millis(1),
        Duration::from_millis(1),
    );
    let peer = RefCell::new(None);
    let attempts = Cell::new(0);
    let client = tokio::time::timeout(Duration::from_secs(5), initial_connect(cfg.initial_connect_retry(), || {
        attempts.set(attempts.get() + 1);
        let cfg = &cfg;
        async move { sleet_client::CalClient::connect_with_options(cfg.url(), cfg.service_id(), cfg.init_options()).await.map_err(RuntimeError::from) }
    }, |_| {
        // Invoked only after the actual typed refusal. Bind before proceeding,
        // so no sleep-and-hope scheduling assumption enters the test.
        let listener = std::net::TcpListener::bind(addr).unwrap();
        listener.set_nonblocking(true).unwrap();
        let listener = tokio::net::TcpListener::from_std(listener).unwrap();
        *peer.borrow_mut() = Some(tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut ws = accept_owp(stream).await;
            let init = ws.next().await.unwrap().unwrap().into_text().unwrap();
            assert!(init.starts_with("INIT "));
            ws.send(tokio_tungstenite::tungstenite::Message::Text(r#"INFO {"version":"1.0","server_id":"mock","uuids":{"system":"s","service":"v"},"system_label":"mock"}"#.into())).await.unwrap();
            // Client is dropped immediately after the successful initial phase.
            let _ = ws.next().await;
            assert!(tokio::time::timeout(Duration::from_millis(30), listener.accept()).await.is_err());
        }));
        ready(())
    })).await.unwrap().unwrap();
    assert_eq!(attempts.get(), 2);
    drop(client);
    let peer = peer.into_inner().unwrap();
    tokio::time::timeout(Duration::from_secs(5), peer)
        .await
        .unwrap()
        .unwrap();
}

async fn accept_owp(
    stream: tokio::net::TcpStream,
) -> tokio_tungstenite::WebSocketStream<tokio::net::TcpStream> {
    #[allow(clippy::result_large_err)]
    let callback =
        |_: &tokio_tungstenite::tungstenite::handshake::server::Request,
         mut response: tokio_tungstenite::tungstenite::handshake::server::Response| {
            response
                .headers_mut()
                .insert("Sec-WebSocket-Protocol", "owp".parse().unwrap());
            Ok(response)
        };
    tokio_tungstenite::accept_hdr_async(stream, callback)
        .await
        .unwrap()
}
