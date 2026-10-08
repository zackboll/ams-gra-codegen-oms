//! Initial phase only. No command/subscription state exists here.

use crate::{ConnectRetryPolicy, RuntimeError};
use sleet_client::ClientError;
use std::error::Error;
use std::future::Future;
use std::io::{Error as IoError, ErrorKind};
use std::time::Duration;

/// The pinned client's Transport wraps tungstenite::Error. Its Io variant
/// exposes the actual io::Error as its direct source; HTTP/TLS/protocol variants
/// do not. ClientError itself does not expose a source, so inspect Transport.
pub(crate) fn is_refused(error: &RuntimeError) -> bool {
    matches!(error, RuntimeError::Client(client)
        if matches!(client.as_ref(), ClientError::Transport(transport)
            if transport.source().and_then(|source| source.downcast_ref::<IoError>())
                .is_some_and(|io| io.kind() == ErrorKind::ConnectionRefused)))
}

/// Private seam: production uses the pinned connector and Tokio sleep; tests
/// script both without exposing transport injection to runtime callers.
pub(crate) async fn initial_connect<T, C, F, D, S>(
    policy: &ConnectRetryPolicy,
    mut connect: C,
    mut delay: D,
) -> Result<T, RuntimeError>
where
    C: FnMut() -> F,
    F: Future<Output = Result<T, RuntimeError>>,
    D: FnMut(Duration) -> S,
    S: Future<Output = ()>,
{
    let mut attempt = 1;
    loop {
        match connect().await {
            Ok(client) => return Ok(client),
            Err(error) => {
                let wait = policy.delay_after(attempt);
                if !is_refused(&error) || wait.is_none() {
                    return Err(error);
                }
                if let Some(wait) = wait {
                    delay(wait).await;
                }
                attempt += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests;
