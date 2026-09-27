//! Task 049: worker lifecycle -- shutdown, drop, connection loss, handler
//! panics, and re-entrant calls -- through GENERATED endpoint functions.

mod common;

use ams_gra_oms_runtime_rust::{
    RuntimeError, RuntimeEvent, SubscriptionHandle, WORKER_THREAD_NAME,
};
use ams_gra_oms_runtime_rust_facade_tests::codec::shared;
use ams_gra_oms_runtime_rust_facade_tests::runtime_test::model::SharedPayload;
use ams_gra_oms_runtime_rust_facade_tests::runtime_test::service_api::function_track as track;
use common::{MockPeer, WAIT, connect};
use std::sync::mpsc;
use std::time::{Duration, Instant};

fn wait_until(mut done: impl FnMut() -> bool) -> bool {
    let deadline = Instant::now() + WAIT;
    while !done() {
        if Instant::now() > deadline {
            return false;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    true
}

/// Live runtime worker threads in this process, by name (Linux only;
/// the kernel truncates thread names to 15 bytes).
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

/// connect, subscribe, publish, then close (or drop): the connection
/// closes and the worker is joined before close/drop returns.
///
/// This is the only test in this binary that holds a runtime while it
/// counts worker threads, and every other test here closes its runtime
/// before it returns, so zero live workers is deterministic at the end.
#[test]
fn task049_close_and_drop_leave_no_worker_thread() {
    for explicit in [true, false] {
        let peer = MockPeer::start();
        let mut runtime = connect(&peer);
        assert!(runtime.is_running());
        let _handle = track::exchange_input_a::subscribe(&mut runtime, |_: &SharedPayload| {})
            .expect("subscribe");
        peer.expect("SUB sub-1 {urn:test}MessageA input-topic");
        track::exchange_output_a::publish(&mut runtime, &shared(1)).expect("publish");
        peer.expect(r#"PUB output-topic {"{urn:test}MessageA":{"Count":1}}"#);
        let started = Instant::now();
        if explicit {
            runtime.close().expect("close");
        } else {
            drop(runtime);
        }
        assert!(started.elapsed() < WAIT, "shutdown must be prompt");
        peer.expect_closed();
    }
    assert!(
        cfg!(not(target_os = "linux")) || wait_until(|| live_workers() == 0),
        "a runtime worker thread outlived its runtime"
    );
}

/// The connection ending is an event; the worker then exits by itself and
/// every operation fails with WorkerStopped.
#[test]
fn task049_connection_loss_is_an_event_and_stops_the_worker() {
    let peer = MockPeer::start();
    let mut runtime = connect(&peer);
    peer.close();
    assert!(matches!(
        runtime.recv_event_timeout(WAIT),
        Some(RuntimeEvent::ConnectionClosed { .. })
    ));
    assert!(wait_until(|| !runtime.is_running()));
    assert!(matches!(
        track::exchange_output_a::publish(&mut runtime, &shared(1)),
        Err(RuntimeError::WorkerStopped)
    ));
    runtime.close().expect("close after loss");
}

/// A panicking handler is contained and reported; dispatch continues.
#[test]
fn task049_handler_panic_is_contained() {
    let peer = MockPeer::start();
    let mut runtime = connect(&peer);
    let (seen_tx, seen) = mpsc::channel();
    let _handle = track::exchange_input_a::subscribe(&mut runtime, move |m: &SharedPayload| {
        assert!(m.count.get() != 13, "deliberate test panic");
        seen_tx.send(m.count.get()).expect("test alive");
    })
    .expect("subscribe");
    peer.expect("SUB sub-1 {urn:test}MessageA input-topic");
    peer.send(r#"MSG sub-1 {"{urn:test}MessageA":{"Count":13}}"#);
    assert!(matches!(
        runtime.recv_event_timeout(WAIT),
        Some(RuntimeEvent::HandlerPanicked { sid }) if sid == "sub-1"
    ));
    peer.send(r#"MSG sub-1 {"{urn:test}MessageA":{"Count":14}}"#);
    assert_eq!(seen.recv_timeout(WAIT), Ok(14));
    runtime.close().expect("close");
}

/// A handler calling back into the runtime is refused, not deadlocked.
#[test]
fn task049_reentrant_handler_call_is_refused() {
    let peer = MockPeer::start();
    let mut runtime = connect(&peer);
    let (probe_tx, probe) = mpsc::channel();
    let (handle_tx, handle_rx) = mpsc::channel::<SubscriptionHandle>();
    let first = track::exchange_input_a::subscribe(&mut runtime, move |_: &SharedPayload| {
        if let Ok(handle) = handle_rx.try_recv() {
            probe_tx.send(handle.unsubscribe()).expect("test alive");
        }
    })
    .expect("subscribe");
    peer.expect("SUB sub-1 {urn:test}MessageA input-topic");
    let second = track::exchange_input_a::subscribe(&mut runtime, |_: &SharedPayload| {})
        .expect("subscribe");
    peer.expect("SUB sub-2 {urn:test}MessageA input-topic");
    handle_tx.send(second).expect("handoff");
    peer.send(r#"MSG sub-1 {"{urn:test}MessageA":{"Count":1}}"#);
    assert!(matches!(
        probe.recv_timeout(WAIT),
        Ok(Err(RuntimeError::CalledFromAsyncContext))
    ));
    first.unsubscribe().expect("worker still serving");
    peer.expect("UNSUB sub-1");
    runtime.close().expect("close");
}
