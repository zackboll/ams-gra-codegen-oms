//! The connection-owning worker: one OS thread, one current-thread Tokio
//! runtime, one `sleet_client::CalClient`.
//!
//! Application threads never touch the client. They send [`Command`]s over
//! a bounded channel and wait on a per-command reply; the worker alone
//! performs `publish` / `subscribe` / `unsubscribe` and continuously
//! `recv`s `MSG` / `-ERR`, dispatching by subscription ID. Commands and
//! receives are selected fairly (see [`Worker::run`]), so neither a busy
//! command queue nor a busy socket can starve the other.

use crate::{MessageDecodeError, RuntimeError, RuntimeEvent};
use serde_json::Value;
use sleet_client::{CalClient, ClientError};
use std::collections::HashMap;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::mpsc as std_mpsc;
use tokio::sync::{mpsc, oneshot};

/// Capacity of the application -> worker command channel. Each blocking
/// operation waits for its own reply, so the queue only ever holds one
/// command per concurrently calling application thread; 64 bounds memory
/// while leaving ample headroom.
pub const COMMAND_CAPACITY: usize = 64;

/// A type-erased typed-dispatch entry: unwraps the global element, decodes
/// the payload, and calls the typed handler.
pub(crate) type Dispatch = Box<dyn FnMut(&str) -> Result<(), MessageDecodeError> + Send>;

pub(crate) type Reply<T> = oneshot::Sender<Result<T, RuntimeError>>;

pub(crate) enum Command {
    Publish {
        topic: &'static str,
        message: Value,
        reply: Reply<()>,
    },
    Subscribe {
        message_name: String,
        topic: &'static str,
        group: Option<&'static str>,
        dispatch: Dispatch,
        reply: Reply<String>,
    },
    Unsubscribe {
        sid: String,
        reply: Reply<()>,
    },
}

struct Subscription {
    message_name: String,
    dispatch: Dispatch,
}

/// Deterministic, connection-local, monotonic subscription IDs:
/// `sub-1`, `sub-2`, ... Every ID is a legal OWP identifier
/// (`^[A-Za-z0-9_\-.]+$`), is issued at most once per connection, and the
/// counter never wraps: exhaustion is an explicit error.
#[derive(Debug)]
pub(crate) struct SubscriptionIds {
    next: u64,
}

impl SubscriptionIds {
    pub(crate) const fn new() -> Self {
        Self { next: 1 }
    }

    #[cfg(test)]
    pub(crate) const fn starting_at(next: u64) -> Self {
        Self { next }
    }

    pub(crate) fn allocate(&mut self) -> Result<String, RuntimeError> {
        let id = self.next;
        self.next = id
            .checked_add(1)
            .ok_or(RuntimeError::SubscriptionIdsExhausted)?;
        Ok(format!("sub-{id}"))
    }
}

pub(crate) struct Worker {
    client: CalClient,
    commands: mpsc::Receiver<Command>,
    shutdown: oneshot::Receiver<()>,
    events: std_mpsc::Sender<RuntimeEvent>,
    subscriptions: HashMap<String, Subscription>,
    ids: SubscriptionIds,
}

impl Worker {
    pub(crate) fn new(
        client: CalClient,
        commands: mpsc::Receiver<Command>,
        shutdown: oneshot::Receiver<()>,
        events: std_mpsc::Sender<RuntimeEvent>,
    ) -> Self {
        Self {
            client,
            commands,
            shutdown,
            events,
            subscriptions: HashMap::new(),
            ids: SubscriptionIds::new(),
        }
    }

    /// Serve until shutdown is requested (or the runtime is dropped), every
    /// command sender is gone, or the connection ends. Dropping `self`
    /// afterwards drops the `CalClient`, closing the connection.
    ///
    /// # Fairness (PR #50 corrective)
    ///
    /// The select is deliberately **unbiased**: when several branches are
    /// ready, Tokio starts polling at a random branch, so none can starve
    /// another. A `biased` order would always prefer whichever branch is
    /// listed first: commands-first lets several application threads keep
    /// the queue ready forever so `recv` never runs (no `MSG`, no late
    /// `-ERR`, no remote close); `recv`-first would let a continuously
    /// readable socket starve application commands the same way. Shutdown
    /// needs no priority: its branch is polled on every iteration, and every
    /// command or dispatch finishes before the next one.
    ///
    /// Losing a race drops the pending `CalClient::recv` future. That was
    /// already true under the biased order and is safe: the pinned client
    /// only suspends in `WebSocketStream::next` (cancel-safe; a partially
    /// read frame stays buffered in tungstenite) or while answering a
    /// `Ping`, and a frame it has read is parsed and returned without
    /// another suspension point.
    pub(crate) async fn run(mut self) {
        loop {
            tokio::select! {
                _ = &mut self.shutdown => break,
                command = self.commands.recv() => match command {
                    Some(command) => self.command(command).await,
                    None => break,
                },
                received = self.client.recv() => match received {
                    Ok(message) => self.deliver(&message.sid, &message.payload),
                    Err(error) => {
                        if !self.receive_error(error) {
                            break;
                        }
                    }
                },
            }
        }
    }

    async fn command(&mut self, command: Command) {
        match command {
            Command::Publish {
                topic,
                message,
                reply,
            } => {
                let result = self.client.publish(topic, &message).await;
                let _ = reply.send(result.map_err(RuntimeError::from));
            }
            Command::Subscribe {
                message_name,
                topic,
                group,
                dispatch,
                reply,
            } => {
                let result = self.subscribe(message_name, topic, group, dispatch).await;
                let _ = reply.send(result);
            }
            Command::Unsubscribe { sid, reply } => {
                // Remove dispatch state FIRST: from here on a MSG for this
                // SID can never reach the old handler, whatever the server
                // does with the UNSUB.
                let result = if self.subscriptions.remove(&sid).is_some() {
                    self.client
                        .unsubscribe(&sid)
                        .await
                        .map_err(RuntimeError::from)
                } else {
                    Err(RuntimeError::SubscriptionNotActive { sid })
                };
                let _ = reply.send(result);
            }
        }
    }

    async fn subscribe(
        &mut self,
        message_name: String,
        topic: &'static str,
        group: Option<&'static str>,
        dispatch: Dispatch,
    ) -> Result<String, RuntimeError> {
        let sid = self.ids.allocate()?;
        // sleet-client is authoritative for OWP lexical rules: an invalid
        // topic or group is rejected here before any frame is written.
        self.client
            .subscribe(&sid, &message_name, topic, group)
            .await?;
        self.subscriptions.insert(
            sid.clone(),
            Subscription {
                message_name,
                dispatch,
            },
        );
        Ok(sid)
    }

    /// Dispatch one `MSG` by its subscription ID only.
    fn deliver(&mut self, sid: &str, payload: &str) {
        let Some(subscription) = self.subscriptions.get_mut(sid) else {
            self.event(RuntimeEvent::UnknownSubscription {
                sid: sid.to_owned(),
                payload: payload.to_owned(),
            });
            return;
        };
        match catch_unwind(AssertUnwindSafe(|| (subscription.dispatch)(payload))) {
            Ok(Ok(())) => {}
            Ok(Err(error)) => {
                let message_name = subscription.message_name.clone();
                self.event(RuntimeEvent::SubscriptionDecodeError {
                    sid: sid.to_owned(),
                    message_name,
                    error,
                });
            }
            Err(_) => self.event(RuntimeEvent::HandlerPanicked {
                sid: sid.to_owned(),
            }),
        }
    }

    /// Surface a receive-path error. Returns whether to keep serving.
    fn receive_error(&self, error: ClientError) -> bool {
        match error {
            ClientError::Server { error, details } => {
                self.event(RuntimeEvent::ServerError {
                    error: error.to_string(),
                    details,
                });
                true
            }
            ClientError::ProtocolParse(_) => {
                self.event(RuntimeEvent::ProtocolError {
                    error: Box::new(error),
                });
                true
            }
            // The pinned client reports end of stream as exactly this text;
            // anything else unexpected (a binary frame, an unsupported
            // server operation) leaves the connection usable.
            ClientError::UnexpectedServerOp(ref text) if text != "connection closed" => {
                self.event(RuntimeEvent::ProtocolError {
                    error: Box::new(error),
                });
                true
            }
            other => {
                self.event(RuntimeEvent::ConnectionClosed {
                    error: Box::new(other),
                });
                false
            }
        }
    }

    /// Events are never dropped while the runtime exists; once the runtime
    /// is gone there is nobody left to observe them.
    fn event(&self, event: RuntimeEvent) {
        let _ = self.events.send(event);
    }
}

#[cfg(test)]
mod fairness_tests;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subscription_ids_are_monotonic_legal_owp_identifiers() {
        let mut ids = SubscriptionIds::new();
        let issued: Vec<_> = (0..3).map(|_| ids.allocate().expect("id")).collect();
        assert_eq!(issued, ["sub-1", "sub-2", "sub-3"]);
        for id in issued {
            assert!(
                id.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"_-.".contains(&b))
            );
        }
    }

    #[test]
    fn subscription_id_overflow_fails_and_never_wraps() {
        let mut ids = SubscriptionIds::starting_at(u64::MAX - 1);
        assert_eq!(
            ids.allocate().expect("penultimate"),
            format!("sub-{}", u64::MAX - 1)
        );
        for _ in 0..3 {
            assert!(matches!(
                ids.allocate(),
                Err(RuntimeError::SubscriptionIdsExhausted)
            ));
        }
    }
}
