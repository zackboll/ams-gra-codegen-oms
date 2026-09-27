//! Runtime errors and asynchronous runtime events.
//!
//! Deliberately separate from generation-time `CodegenError` and
//! `ServiceApiError`: nothing here describes source generation.

use crate::{CodecError, MessageDecodeError};
use sleet_client::ClientError;
use std::fmt;

/// A synchronous runtime operation failed.
///
/// Success of an operation means the OWP frame was **written to the
/// connection** (the connection is non-verbose); a later server `-ERR` is
/// delivered as [`RuntimeEvent::ServerError`], never here.
#[derive(Debug)]
#[non_exhaustive]
pub enum RuntimeError {
    /// The [`crate::RuntimeConfig`] cannot describe a connection.
    InvalidConfig(String),
    /// The payload codec rejected an outgoing value.
    Codec(CodecError),
    /// `sleet-client` rejected the operation (boxed to keep `Result`s
    /// small). `ClientError::InvalidInput` is the authoritative OWP lexical
    /// rejection of a topic, group, service ID, or subscription ID;
    /// transport and handshake failures (including a server `-ERR` to
    /// `INIT`) arrive here too.
    Client(Box<ClientError>),
    /// The WebSocket upgrade plus `INIT`/`INFO` did not finish in time.
    ConnectTimeout,
    /// The worker thread could not be started.
    WorkerSpawn(std::io::Error),
    /// The worker has stopped (closed, or its connection ended).
    WorkerStopped,
    /// The worker thread panicked.
    WorkerPanicked,
    /// A blocking runtime call was made from inside an async (Tokio)
    /// context -- including from a subscription handler, which runs on the
    /// worker -- where blocking would stall or deadlock the executor.
    CalledFromAsyncContext,
    /// The subscription ID is not active on this connection.
    SubscriptionNotActive { sid: String },
    /// The connection-local subscription ID counter is exhausted; IDs are
    /// never reused.
    SubscriptionIdsExhausted,
}

impl From<ClientError> for RuntimeError {
    fn from(error: ClientError) -> Self {
        Self::Client(Box::new(error))
    }
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidConfig(detail) => write!(f, "invalid runtime configuration: {detail}"),
            Self::Codec(error) => write!(f, "payload codec error: {error}"),
            Self::Client(error) => write!(f, "LA-CAL client error: {error}"),
            Self::ConnectTimeout => f.write_str("LA-CAL connect/INIT timed out"),
            Self::WorkerSpawn(error) => write!(f, "cannot start runtime worker: {error}"),
            Self::WorkerStopped => f.write_str("runtime worker has stopped"),
            Self::WorkerPanicked => f.write_str("runtime worker panicked"),
            Self::CalledFromAsyncContext => f.write_str(
                "blocking runtime operation called from an async context or subscription handler",
            ),
            Self::SubscriptionNotActive { sid } => {
                write!(f, "subscription {sid} is not active")
            }
            Self::SubscriptionIdsExhausted => f.write_str("subscription IDs exhausted"),
        }
    }
}

impl std::error::Error for RuntimeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Codec(error) => Some(error),
            Self::Client(error) => Some(error.as_ref()),
            Self::WorkerSpawn(error) => Some(error),
            _ => None,
        }
    }
}

/// Something asynchronous happened on the connection.
///
/// Events are queued without bound and never dropped; drain them with
/// [`crate::SleetRuntime::try_recv_event`] /
/// [`crate::SleetRuntime::recv_event_timeout`].
#[derive(Debug)]
#[non_exhaustive]
pub enum RuntimeEvent {
    /// The server sent `-ERR`. On a non-verbose connection this is the only
    /// signal that an earlier operation failed; OWP does not say which one.
    ServerError {
        /// The OWP error code, e.g. `Illegal-State`.
        error: String,
        /// Optional server detail text.
        details: Option<String>,
    },
    /// A `MSG` for an active subscription was not delivered: its JSON is
    /// not exactly the subscribed global element, or the codec rejected it.
    /// The subscription stays active.
    SubscriptionDecodeError {
        sid: String,
        /// The expected global element name.
        message_name: String,
        error: MessageDecodeError,
    },
    /// A `MSG` named a subscription ID this connection does not hold (never
    /// issued, or already unsubscribed). No handler was invoked.
    UnknownSubscription { sid: String, payload: String },
    /// A codec or handler panicked while dispatching a `MSG`. The panic was
    /// contained on the worker; the subscription stays active.
    HandlerPanicked { sid: String },
    /// The server sent a frame `sleet-client` could not parse; the
    /// connection continues.
    ProtocolError { error: Box<ClientError> },
    /// The connection ended. The worker has stopped; every later operation
    /// fails with [`RuntimeError::WorkerStopped`].
    ConnectionClosed { error: Box<ClientError> },
}
