//! The mock peer's single-connection serve loop (test-only).

use super::{INFO_FRAME, InitReply, Observed, Outgoing, WAIT};
use futures_util::{SinkExt, StreamExt};
use std::sync::mpsc as std_mpsc;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::handshake::server::{ErrorResponse, Request, Response};
use tokio_tungstenite::tungstenite::http;

pub async fn serve(
    init: InitReply,
    address: std_mpsc::Sender<std::net::SocketAddr>,
    protocol: std_mpsc::Sender<Option<String>>,
    observed: std_mpsc::Sender<Observed>,
    mut outgoing: mpsc::UnboundedReceiver<Outgoing>,
    watch_connections: bool,
) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    address
        .send(listener.local_addr().expect("address"))
        .expect("report address");
    let Ok(Ok((stream, _))) = tokio::time::timeout(WAIT, listener.accept()).await else {
        return;
    };
    // The handshake callback's `Err` type is fixed by tungstenite (as in
    // sleet-client's own test peer, which allows the same lint).
    #[allow(clippy::result_large_err)]
    let callback = |request: &Request, mut response: Response| {
        let offered = request
            .headers()
            .get("Sec-WebSocket-Protocol")
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);
        let accepted = offered.as_deref() == Some("owp");
        let _ = protocol.send(offered);
        if !accepted {
            let rejection: ErrorResponse = http::Response::builder()
                .status(http::StatusCode::BAD_REQUEST)
                .body(Some("owp subprotocol required".to_owned()))
                .expect("400 response");
            return Err(rejection);
        }
        response.headers_mut().insert(
            "Sec-WebSocket-Protocol",
            http::HeaderValue::from_static("owp"),
        );
        Ok(response)
    };
    let Ok(mut ws) = tokio_tungstenite::accept_hdr_async(stream, callback).await else {
        return;
    };
    loop {
        tokio::select! {
            incoming = ws.next() => match incoming {
                Some(Ok(Message::Text(text))) => {
                    let text = text.to_string();
                    let is_init = text.starts_with("INIT ");
                    let _ = observed.send(Observed::Text(text));
                    if is_init {
                        let reply = match init {
                            InitReply::Info => INFO_FRAME,
                            InitReply::Frame(frame) => frame,
                        };
                        if ws.send(Message::Text(reply.into())).await.is_err() {
                            break;
                        }
                    }
                }
                Some(Ok(Message::Close(_)) | Err(_)) | None => {
                    let _ = observed.send(Observed::Closed);
                    break;
                }
                Some(Ok(_)) => {}
            },
            command = outgoing.recv() => match command {
                Some(Outgoing::Text(frame)) => {
                    if ws.send(Message::Text(frame.into())).await.is_err() {
                        break;
                    }
                }
                Some(Outgoing::Close) | None => {
                    let _ = ws.close(None).await;
                    break;
                }
            },
        }
    }
    if !watch_connections {
        return;
    }
    let observation = if tokio::time::timeout(super::QUIET, listener.accept())
        .await
        .is_err()
    {
        Observed::NoConnection
    } else {
        Observed::ExtraConnection
    };
    let _ = observed.send(observation);
}
