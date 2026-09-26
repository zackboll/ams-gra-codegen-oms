//! PR #50 corrective: receive progress under sustained command load,
//! end to end through the GENERATED facade.
//!
//! Several application threads keep publishing through the generated
//! endpoint function. While that load is still running, a `MSG` must reach
//! the typed handler and a late `-ERR` must become a runtime event; then the
//! load stops, the subscription is unsubscribed, the runtime closes, and no
//! worker thread remains. Every wait is bounded.
//!
//! Scope, stated honestly: the public adapter calls take
//! `&mut SleetRuntime` and block for their reply, so application threads
//! must serialize on a lock and at most one publish is queued at a time.
//! That keeps the worker busy but cannot hold its queue *continuously*
//! ready, so this test also passed under the original `biased` select. The
//! discriminating regression, which keeps the bounded queue at capacity and
//! failed at reviewed head 168782d, is the crate-private
//! `worker::fairness_tests` in `ams-gra-oms-runtime-rust`.

mod common;

use ams_gra_oms_runtime_rust::{RuntimeEvent, SleetRuntime, WORKER_THREAD_NAME};
use ams_gra_oms_runtime_rust_facade_tests::codec::{TestCodec, shared};
use ams_gra_oms_runtime_rust_facade_tests::runtime_test::model::SharedPayload;
use ams_gra_oms_runtime_rust_facade_tests::runtime_test::service_api::function_track as track;
use common::{MockPeer, Observed, WAIT, connect};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

/// Concurrent publisher threads sharing the runtime.
const PUBLISHERS: usize = 4;

/// Publishes the load must complete between probes, proving it is still
/// running (a liveness floor, not a rate assertion).
const PROGRESS: u64 = 200;

type Shared = Arc<Mutex<SleetRuntime<TestCodec>>>;

struct Load {
    stop: Arc<AtomicBool>,
    published: Arc<AtomicU64>,
    threads: Vec<JoinHandle<()>>,
}
impl Load {
    /// `SleetRuntime` is `!Sync` and each adapter call takes `&mut self`,
    /// so concurrent application threads necessarily share it through a
    /// lock; the load is therefore back-to-back commands from several
    /// threads, each blocking for its own reply.
    fn start(runtime: &Shared) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let published = Arc::new(AtomicU64::new(0));
        let threads = (0..PUBLISHERS)
            .map(|index| {
                let runtime = Arc::clone(runtime);
                let stop = Arc::clone(&stop);
                let published = Arc::clone(&published);
                thread::Builder::new()
                    .name(format!("publisher-{index}"))
                    .spawn(move || {
                        while !stop.load(Ordering::Relaxed) {
                            let mut guard = runtime.lock().expect("runtime lock");
                            track::exchange_output_a::publish(&mut *guard, &shared(1))
                                .expect("publish under load");
                            drop(guard);
                            published.fetch_add(1, Ordering::Relaxed);
                        }
                    })
                    .expect("spawn publisher")
            })
            .collect();
        Self {
            stop,
            published,
            threads,
        }
    }

    fn published(&self) -> u64 {
        self.published.load(Ordering::Relaxed)
    }

    /// Every publisher is still looping.
    fn running(&self) -> bool {
        !self.stop.load(Ordering::Relaxed) && self.threads.iter().all(|t| !t.is_finished())
    }

    /// Wait (bounded) until the load has completed `count` more publishes.
    fn advance_by(&self, count: u64) {
        let target = self.published() + count;
        let deadline = Instant::now() + WAIT;
        while self.published() < target {
            assert!(Instant::now() < deadline, "publisher load made no progress");
            thread::sleep(Duration::from_millis(1));
        }
    }

    fn stop_and_join(self) -> u64 {
        self.stop.store(true, Ordering::Relaxed);
        for thread in self.threads {
            thread.join().expect("publisher thread");
        }
        self.published.load(Ordering::Relaxed)
    }
}

/// Wait (bounded) for a runtime event, taking the lock only briefly so the
/// publishers keep running meanwhile.
fn next_event(runtime: &Shared) -> Option<RuntimeEvent> {
    let deadline = Instant::now() + WAIT;
    while Instant::now() < deadline {
        if let Some(event) = runtime.lock().expect("runtime lock").try_recv_event() {
            return Some(event);
        }
        thread::sleep(Duration::from_millis(1));
    }
    None
}

/// Live runtime worker threads in this process (Linux; names truncated to
/// 15 bytes by the kernel).
fn live_workers() -> usize {
    let Ok(tasks) = std::fs::read_dir("/proc/self/task") else {
        return 0;
    };
    tasks
        .filter_map(Result::ok)
        .filter(|task| {
            std::fs::read_to_string(task.path().join("comm")).is_ok_and(|name| {
                let name = name.trim_end();
                !name.is_empty() && WORKER_THREAD_NAME.starts_with(name)
            })
        })
        .count()
}

const LOAD_FRAME: &str = r#"PUB output-topic {"{urn:test}MessageA":{"Count":1}}"#;

#[test]
fn task049_receive_progresses_under_sustained_publish_load() {
    let peer = MockPeer::start();
    let mut runtime = connect(&peer);
    let (seen_tx, seen) = mpsc::channel();
    let subscription =
        track::exchange_input_a::subscribe(&mut runtime, move |m: &SharedPayload| {
            let _ = seen_tx.send(m.count.get());
        })
        .expect("subscribe");
    peer.expect("SUB sub-1 {urn:test}MessageA input-topic");

    let runtime: Shared = Arc::new(Mutex::new(runtime));
    let load = Load::start(&runtime);
    load.advance_by(PROGRESS);

    // MSG while the command side is busy: the typed handler must run
    // before the publishers are stopped.
    peer.send(r#"MSG sub-1 {"{urn:test}MessageA":{"Count":42}}"#);
    assert_eq!(
        seen.recv_timeout(WAIT),
        Ok(42),
        "MSG was not dispatched while publishers were active"
    );
    assert!(load.running(), "load must still be active at dispatch");
    load.advance_by(PROGRESS);

    // Late -ERR while the command side is still busy.
    peer.send("-ERR Illegal-State synthetic late error");
    match next_event(&runtime) {
        Some(RuntimeEvent::ServerError { error, details }) => {
            assert_eq!(error, "Illegal-State");
            assert_eq!(details.as_deref(), Some("synthetic late error"));
        }
        other => panic!("expected ServerError under load, got {other:?}"),
    }
    assert!(load.running(), "load must still be active at the event");
    load.advance_by(PROGRESS);

    // Only now stop the command load, then tear down in order.
    let total = load.stop_and_join();
    assert!(total >= 3 * PROGRESS);
    subscription.unsubscribe().expect("unsubscribe");
    let runtime = Arc::try_unwrap(runtime)
        .unwrap_or_else(|_| panic!("publishers still hold the runtime"))
        .into_inner()
        .expect("runtime lock");
    runtime.close().expect("close");

    // Every load frame is the exact generated PUB; then UNSUB, then close.
    let (skipped, next) = peer.skip_while(|frame| frame == LOAD_FRAME);
    assert_eq!(skipped, total, "every completed publish was written once");
    assert_eq!(next, Observed::Text("UNSUB sub-1".to_owned()));
    peer.expect_closed();
    assert!(
        cfg!(not(target_os = "linux")) || live_workers() == 0,
        "a runtime worker thread outlived its runtime"
    );
}
