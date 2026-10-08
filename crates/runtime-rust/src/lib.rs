//! Task 049: the reference Rust LA-CAL runtime adapter.
//!
//! [`SleetRuntime`] executes the typed operations of a generated Rust
//! service API (Task 048) through a real LA-CAL connection:
//!
//! ```text
//! generated service_api  --PublishAdapter / SubscribeAdapter-->  SleetRuntime
//!   --OmsJsonCodec<P> (payload body)-->  global-element envelope (this crate)
//!   --sleet-client @ e38f61d8 (OWP over WebSocket)-->  Sleet / any CAL server
//! ```
//!
//! # Layering
//!
//! * **Runtime (this crate)**: the message QName → LA-CAL name
//!   ([`oms_global_element_name`]), the one-member OMS JSON envelope,
//!   subscription IDs, `PUB`/`SUB`/`UNSUB`, `MSG` dispatch by SID, the
//!   connection, and runtime events.
//! * **Codec ([`OmsJsonCodec`])**: payload fields and values only.
//! * **`sleet-client`**: WebSocket upgrade (`Sec-WebSocket-Protocol: owp`),
//!   `INIT`/`INFO`, OWP framing, and authoritative OWP lexical validation of
//!   topics, groups, service and subscription IDs. Not copied or forked.
//!
//! # Worker
//!
//! One background OS thread per runtime owns a current-thread Tokio runtime
//! and the only `CalClient`. Adapter calls are synchronous: each sends one
//! command over a bounded channel ([`COMMAND_CAPACITY`]) and blocks for its
//! reply. The worker selects fairly between commands and incoming `MSG` /
//! `-ERR` frames, so receive processing keeps progressing while application
//! operations are active, and a busy socket cannot starve commands either.
//! Subscription handlers run **on the worker thread**; a handler must not
//! call back into the runtime (it gets [`RuntimeError::CalledFromAsyncContext`]).
//!
//! # Non-verbose OWP
//!
//! Connections are opened with `"verbose": false`. An adapter call returning
//! `Ok` means the frame was **written to the connection**, not that the
//! server accepted it. A server `-ERR` arrives later as
//! [`RuntimeEvent::ServerError`] (drain with [`SleetRuntime::try_recv_event`]
//! and friends); it is never discarded. This keeps the single worker free to
//! receive `MSG` frames without an operation/acknowledgement pairing that OWP
//! `-ERR` frames cannot express (they carry no operation or subscription ID).

mod codec;
mod config;
mod envelope;
mod error;
mod retry;
mod worker;

pub use codec::{CodecError, OmsJsonCodec};
pub use config::{
    ConnectRetryPolicy, DEFAULT_CONNECT_TIMEOUT, DEFAULT_OWP_VERSION, MAX_INITIAL_CONNECT_ATTEMPTS,
    RuntimeConfig,
};
pub use envelope::{
    MessageDecodeError, OAM_NAMESPACE, oms_global_element_name, unwrap_global_element,
    wrap_global_element,
};
pub use error::{RuntimeError, RuntimeEvent};
pub use worker::COMMAND_CAPACITY;

use ams_gra_oms_runtime_api::{PublishAdapter, SubscribeAdapter};
use sleet_client::CalClient;
use std::sync::{Arc, mpsc as std_mpsc};
use std::thread::{self, JoinHandle};
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};
use worker::{Command, Dispatch, Reply, Worker};

/// The worker thread's name, visible in debuggers and thread listings.
pub const WORKER_THREAD_NAME: &str = "ams-gra-oms-lacal";

/// One LA-CAL connection serving generated Rust service APIs.
///
/// `C` is the payload codec provider; it must implement [`OmsJsonCodec<P>`]
/// for every payload type `P` the application publishes or subscribes.
pub struct SleetRuntime<C> {
    codec: Arc<C>,
    commands: mpsc::Sender<Command>,
    events: std_mpsc::Receiver<RuntimeEvent>,
    shutdown: Option<oneshot::Sender<()>>,
    thread: Option<JoinHandle<()>>,
}

impl<C: Send + Sync + 'static> SleetRuntime<C> {
    /// Connect, perform the WebSocket upgrade and `INIT`/`INFO` handshake,
    /// and start the worker. Returns only after the handshake succeeded.
    ///
    /// # Errors
    ///
    /// [`RuntimeError::InvalidConfig`], [`RuntimeError::Client`] (for
    /// example an invalid service ID, a refused connection, or a server
    /// `-ERR` to `INIT`), [`RuntimeError::ConnectTimeout`],
    /// [`RuntimeError::WorkerSpawn`], [`RuntimeError::WorkerPanicked`], or
    /// [`RuntimeError::CalledFromAsyncContext`].
    pub fn connect(config: RuntimeConfig, codec: C) -> Result<Self, RuntimeError> {
        config.validate()?;
        refuse_async_context()?;
        let (commands, command_rx) = mpsc::channel(COMMAND_CAPACITY);
        let (shutdown, shutdown_rx) = oneshot::channel();
        let (event_tx, events) = std_mpsc::channel();
        let (ready_tx, ready_rx) = std_mpsc::sync_channel(1);
        let thread = thread::Builder::new()
            .name(WORKER_THREAD_NAME.to_owned())
            .spawn(move || serve(&config, command_rx, shutdown_rx, event_tx, &ready_tx))
            .map_err(RuntimeError::WorkerSpawn)?;
        match ready_rx.recv() {
            Ok(Ok(())) => Ok(Self {
                codec: Arc::new(codec),
                commands,
                events,
                shutdown: Some(shutdown),
                thread: Some(thread),
            }),
            Ok(Err(error)) => {
                let _ = thread.join();
                Err(error)
            }
            // The worker ended without reporting: it panicked.
            Err(_) => {
                let _ = thread.join();
                Err(RuntimeError::WorkerPanicked)
            }
        }
    }
}

impl<C> SleetRuntime<C> {
    /// Stop the worker, drop the connection, and join the worker thread.
    /// No background thread remains afterwards. Subscriptions still held
    /// are not individually unsubscribed; the connection simply ends.
    ///
    /// # Errors
    ///
    /// [`RuntimeError::WorkerPanicked`] if the worker thread panicked.
    pub fn close(mut self) -> Result<(), RuntimeError> {
        self.stop()
    }

    /// The next queued runtime event, without waiting.
    pub fn try_recv_event(&self) -> Option<RuntimeEvent> {
        self.events.try_recv().ok()
    }

    /// The next runtime event, waiting at most `timeout`.
    pub fn recv_event_timeout(&self, timeout: Duration) -> Option<RuntimeEvent> {
        self.events.recv_timeout(timeout).ok()
    }

    /// The next runtime event, waiting until one arrives or the worker has
    /// stopped and every queued event was drained (`None`).
    pub fn recv_event(&self) -> Option<RuntimeEvent> {
        self.events.recv().ok()
    }

    /// Whether the worker thread is still running.
    #[must_use]
    pub fn is_running(&self) -> bool {
        self.thread
            .as_ref()
            .is_some_and(|thread| !thread.is_finished())
    }

    fn stop(&mut self) -> Result<(), RuntimeError> {
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
        let Some(thread) = self.thread.take() else {
            return Ok(());
        };
        // A runtime moved into (and dropped by) its own handler cannot join
        // the thread it is running on; the worker exits after this dispatch.
        if thread.thread().id() == thread::current().id() {
            return Ok(());
        }
        thread.join().map_err(|_| RuntimeError::WorkerPanicked)
    }
}

/// Dropping a runtime without [`SleetRuntime::close`] performs the same
/// shutdown, best-effort: the worker is signalled and joined, but a worker
/// panic cannot be reported.
impl<C> Drop for SleetRuntime<C> {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

/// Generated `publish` → encode payload → wrap in the global element →
/// `PUB <topic> <OMS JSON>` via `sleet_client::CalClient::publish`.
///
/// The global element key comes ONLY from the generated message namespace
/// and local name; never from the payload type, the topic, or a Rust name.
impl<C, P> PublishAdapter<P> for SleetRuntime<C>
where
    C: OmsJsonCodec<P>,
{
    type Output = Result<(), RuntimeError>;

    fn publish(
        &mut self,
        message_namespace: &'static str,
        message_name: &'static str,
        topic: &'static str,
        value: &P,
    ) -> Self::Output {
        let payload = self
            .codec
            .encode_payload(value)
            .map_err(RuntimeError::Codec)?;
        let message = wrap_global_element(
            oms_global_element_name(message_namespace, message_name),
            payload,
        );
        request(&self.commands, |reply| Command::Publish {
            topic,
            message,
            reply,
        })
    }
}

/// Generated `subscribe` → runtime-owned SID →
/// `SUB <sid> <LA-CAL message name> <topic> [group]` → a dispatch entry
/// keyed by SID → an explicit [`SubscriptionHandle`].
///
/// The handler is stored beyond the call and runs on the worker thread,
/// hence `Send + 'static`; that bound is this runtime's policy and is not
/// part of the generated façade.
impl<C, P, H> SubscribeAdapter<P, H> for SleetRuntime<C>
where
    C: OmsJsonCodec<P>,
    P: 'static,
    H: FnMut(&P) + Send + 'static,
{
    type Output = Result<SubscriptionHandle, RuntimeError>;

    fn subscribe(
        &mut self,
        message_namespace: &'static str,
        message_name: &'static str,
        topic: &'static str,
        subscription_group: Option<&'static str>,
        mut handler: H,
    ) -> Self::Output {
        let element = oms_global_element_name(message_namespace, message_name);
        let codec = Arc::clone(&self.codec);
        let expected = element.clone();
        let dispatch: Dispatch = Box::new(move |raw: &str| {
            let body = unwrap_global_element(&expected, raw)?;
            let payload = codec
                .decode_payload(&body)
                .map_err(MessageDecodeError::Codec)?;
            handler(&payload);
            Ok(())
        });
        let sid = request(&self.commands, |reply| Command::Subscribe {
            message_name: element,
            topic,
            group: subscription_group,
            dispatch,
            reply,
        })?;
        Ok(SubscriptionHandle {
            sid,
            commands: self.commands.clone(),
        })
    }
}

/// One active subscription. Consumed by [`SubscriptionHandle::unsubscribe`],
/// so a handle can be unsubscribed at most once.
///
/// Dropping a handle does **not** unsubscribe: the subscription stays active
/// until [`SubscriptionHandle::unsubscribe`] or the runtime closes.
#[derive(Debug)]
#[must_use = "dropping a SubscriptionHandle leaves the subscription active"]
pub struct SubscriptionHandle {
    sid: String,
    commands: mpsc::Sender<Command>,
}

impl SubscriptionHandle {
    /// The runtime-generated OWP subscription ID.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.sid
    }

    /// Remove the dispatch entry, then send `UNSUB <sid>`. After this
    /// returns no `MSG` for this ID reaches the old handler.
    ///
    /// # Errors
    ///
    /// [`RuntimeError::WorkerStopped`] once the runtime closed,
    /// [`RuntimeError::SubscriptionNotActive`] if the worker no longer
    /// holds the ID, or a [`RuntimeError::Client`] send failure.
    pub fn unsubscribe(self) -> Result<(), RuntimeError> {
        let sid = self.sid;
        request(&self.commands, |reply| Command::Unsubscribe { sid, reply })
    }
}

/// Send one command and block for its reply.
fn request<T>(
    commands: &mpsc::Sender<Command>,
    command: impl FnOnce(Reply<T>) -> Command,
) -> Result<T, RuntimeError> {
    refuse_async_context()?;
    let (reply, response) = oneshot::channel();
    commands
        .blocking_send(command(reply))
        .map_err(|_| RuntimeError::WorkerStopped)?;
    response
        .blocking_recv()
        .map_err(|_| RuntimeError::WorkerStopped)?
}

/// Blocking inside a Tokio runtime (including a handler on the worker)
/// would panic or deadlock; report it as an error instead.
fn refuse_async_context() -> Result<(), RuntimeError> {
    if tokio::runtime::Handle::try_current().is_ok() {
        Err(RuntimeError::CalledFromAsyncContext)
    } else {
        Ok(())
    }
}

/// The worker thread body: build the runtime, connect (reporting the
/// handshake result to `connect`), then serve.
fn serve(
    config: &RuntimeConfig,
    commands: mpsc::Receiver<Command>,
    shutdown: oneshot::Receiver<()>,
    events: std_mpsc::Sender<RuntimeEvent>,
    ready: &std_mpsc::SyncSender<Result<(), RuntimeError>>,
) {
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            let _ = ready.send(Err(RuntimeError::WorkerSpawn(error)));
            return;
        }
    };
    runtime.block_on(async move {
        let client = match retry::initial_connect(
            config.initial_connect_retry(),
            || async {
                tokio::time::timeout(
                    config.connect_timeout(),
                    CalClient::connect_with_options(
                        config.url(),
                        config.service_id(),
                        config.init_options(),
                    ),
                )
                .await
                .map_err(|_| RuntimeError::ConnectTimeout)?
                .map_err(RuntimeError::from)
            },
            tokio::time::sleep,
        )
        .await
        {
            Ok(client) => client,
            Err(error) => {
                let _ = ready.send(Err(error));
                return;
            }
        };
        if ready.send(Ok(())).is_err() {
            return;
        }
        Worker::new(client, commands, shutdown, events).run().await;
    });
}
