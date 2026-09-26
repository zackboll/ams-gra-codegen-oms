//! A deterministic, TEST-ONLY OWP WebSocket peer.
//!
//! It accepts exactly one connection, REQUIRES `Sec-WebSocket-Protocol: owp`
//! (anything else gets HTTP 400, as OMSC-SPC-013 CERT LACAL-000004
//! requires), answers `INIT` as scripted, records every text frame the
//! client writes, and sends whatever frames the test asks for. It is not a
//! CAL server and nothing in production uses it.

#![allow(dead_code)]

use std::sync::mpsc as std_mpsc;
use std::thread;
use std::time::Duration;
use tokio::sync::mpsc;

mod serve;

/// Upper bound for every wait in these tests.
pub const WAIT: Duration = Duration::from_secs(5);

/// How long "nothing happens" is observed for.
pub const QUIET: Duration = Duration::from_millis(300);

/// Connect a runtime with the handwritten test codec and consume `INIT`.
pub fn connect(
    peer: &MockPeer,
) -> ams_gra_oms_runtime_rust::SleetRuntime<ams_gra_oms_runtime_rust_facade_tests::codec::TestCodec>
{
    let runtime = ams_gra_oms_runtime_rust::SleetRuntime::connect(
        ams_gra_oms_runtime_rust::RuntimeConfig::new(&peer.url, "svc-1", "000.1.0"),
        ams_gra_oms_runtime_rust_facade_tests::codec::TestCodec,
    )
    .expect("connect");
    assert!(peer.next_text().starts_with("INIT "));
    runtime
}

/// `sleet-client` is authoritative for OWP lexical rules; its
/// `ClientError::InvalidInput` is preserved as the error source.
pub fn is_invalid_input(error: &ams_gra_oms_runtime_rust::RuntimeError, needle: &str) -> bool {
    use std::error::Error;
    matches!(error, ams_gra_oms_runtime_rust::RuntimeError::Client(_))
        && error
            .source()
            .is_some_and(|source| source.to_string().starts_with("invalid input:"))
        && error.to_string().contains(needle)
}

pub const INFO_FRAME: &str = concat!(
    r#"INFO {"version":"1.0","server_id":"mock","#,
    r#""uuids":{"system":"s","service":"v"},"system_label":"mock"}"#
);

/// What the peer observed.
#[derive(Debug, PartialEq, Eq)]
pub enum Observed {
    Text(String),
    Closed,
}

pub enum Outgoing {
    Text(String),
    Close,
}

/// How to answer `INIT`.
#[derive(Clone, Copy)]
pub enum InitReply {
    Info,
    Frame(&'static str),
}

pub struct MockPeer {
    pub url: String,
    protocol: std_mpsc::Receiver<Option<String>>,
    observed: std_mpsc::Receiver<Observed>,
    outgoing: mpsc::UnboundedSender<Outgoing>,
}

impl MockPeer {
    pub fn start() -> Self {
        Self::start_with(InitReply::Info)
    }

    pub fn start_with(init: InitReply) -> Self {
        let (address_tx, address_rx) = std_mpsc::channel();
        let (protocol_tx, protocol) = std_mpsc::channel();
        let (observed_tx, observed) = std_mpsc::channel();
        let (outgoing, outgoing_rx) = mpsc::unbounded_channel();
        thread::Builder::new()
            .name("mock-owp-peer".to_owned())
            .spawn(move || {
                tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("mock runtime")
                    .block_on(serve::serve(
                        init,
                        address_tx,
                        protocol_tx,
                        observed_tx,
                        outgoing_rx,
                    ));
            })
            .expect("spawn mock peer");
        let address = address_rx.recv_timeout(WAIT).expect("mock peer address");
        Self {
            url: format!("ws://{address}"),
            protocol,
            observed,
            outgoing,
        }
    }

    /// The `Sec-WebSocket-Protocol` the client offered.
    pub fn offered_protocol(&self) -> Option<String> {
        self.protocol.recv_timeout(WAIT).expect("no handshake")
    }

    /// The next text frame the client wrote.
    pub fn next_text(&self) -> String {
        match self.observed.recv_timeout(WAIT) {
            Ok(Observed::Text(text)) => text,
            other => panic!("expected a text frame, got {other:?}"),
        }
    }

    /// The next text frame must be exactly `expected`.
    pub fn expect(&self, expected: &str) {
        assert_eq!(self.next_text(), expected);
    }

    /// The client must write nothing for `duration`.
    pub fn expect_silence(&self, duration: Duration) {
        if let Ok(observed) = self.observed.recv_timeout(duration) {
            panic!("expected no frame, got {observed:?}");
        }
    }

    /// The client must close the connection.
    pub fn expect_closed(&self) {
        assert_eq!(self.observed.recv_timeout(WAIT), Ok(Observed::Closed));
    }

    pub fn send(&self, frame: &str) {
        self.outgoing
            .send(Outgoing::Text(frame.to_owned()))
            .expect("mock peer running");
    }

    pub fn close(&self) {
        self.outgoing
            .send(Outgoing::Close)
            .expect("mock peer running");
    }
}
