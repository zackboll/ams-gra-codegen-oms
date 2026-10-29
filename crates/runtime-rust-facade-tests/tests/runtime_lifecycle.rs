//! Task 049: worker lifecycle -- shutdown, drop, connection loss, handler
//! panics, and re-entrant calls -- through GENERATED endpoint functions.

mod common;

use ams_gra_oms_runtime_rust::{
    RuntimeError, RuntimeEvent, SubscriptionHandle, WORKER_THREAD_NAME,
};
use ams_gra_oms_runtime_rust_facade_tests::codec::shared;
use ams_gra_oms_runtime_rust_facade_tests::runtime_test::model::SharedPayload;
use ams_gra_oms_runtime_rust_facade_tests::runtime_test::service_api::function_track as track;
use common::worker_join::{WorkerExit, assert_still_blocked};
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

/// connect, subscribe, publish, then close (or drop): the connection
/// closes and the worker is joined before close/drop returns.
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
}

fn shutdown_joins_actual_worker(explicit: bool) {
    let peer = MockPeer::start();
    let mut runtime = connect(&peer);
    let (mut worker_exit, exit) = WorkerExit::new(true);
    let (entered_tx, entered) = mpsc::channel();
    let (release_tx, release) = mpsc::channel();
    let (callback_done_tx, callback_done) = mpsc::channel();
    let handle = track::exchange_input_a::subscribe(&mut runtime, move |_: &SharedPayload| {
        let id = worker_exit.arm();
        assert_eq!(std::thread::current().name(), Some(WORKER_THREAD_NAME));
        entered_tx.send(id).expect("test alive");
        release.recv_timeout(WAIT).expect("release blocked handler");
        callback_done_tx.send(id).expect("test alive");
    })
    .expect("subscribe");
    peer.expect("SUB sub-1 {urn:test}MessageA input-topic");
    peer.send(r#"MSG sub-1 {"{urn:test}MessageA":{"Count":1}}"#);
    let worker = entered
        .recv_timeout(WAIT)
        .expect("handler entered actual worker");
    assert_ne!(worker, std::thread::current().id());
    let (started_tx, started) = mpsc::channel();
    let (done_tx, done) = mpsc::channel();
    let shutdown = std::thread::spawn(move || {
        started_tx.send(()).expect("test alive");
        let result = if explicit {
            runtime.close()
        } else {
            drop(runtime);
            Ok(())
        };
        done_tx.send(result).expect("test alive");
    });
    started
        .recv_timeout(WAIT)
        .expect("shutdown application thread started");
    assert_still_blocked(&done, "handler");
    release_tx.send(()).expect("release callback");
    assert_eq!(callback_done.recv_timeout(WAIT), Ok(worker));
    assert_eq!(exit.entered.recv_timeout(WAIT), Ok(worker));
    // Even after the callback and connection-owning worker body finish,
    // shutdown must wait for this exact thread's TLS teardown. A detached
    // worker which merely stops later cannot pass this second barrier.
    assert_still_blocked(&done, "thread-local teardown");
    exit.release
        .as_ref()
        .unwrap()
        .send(())
        .expect("release TLS teardown");
    done.recv_timeout(WAIT)
        .expect("shutdown completed")
        .expect("successful shutdown");
    assert_eq!(exit.finished.try_recv(), Ok(worker));
    shutdown.join().expect("shutdown application thread");
    peer.expect_closed();
    assert!(
        matches!(entered.try_recv(), Err(mpsc::TryRecvError::Disconnected)),
        "no further callback can execute"
    );
    assert!(matches!(
        handle.unsubscribe(),
        Err(RuntimeError::WorkerStopped)
    ));
    eprintln!("WORKER_JOIN_COMPLETE explicit={explicit} worker={worker:?}");
}

#[test]
fn rust_ci_001_close_joins_blocked_worker() {
    shutdown_joins_actual_worker(true);
}

#[test]
fn rust_ci_001_drop_joins_blocked_worker() {
    shutdown_joins_actual_worker(false);
}

/// Handler panics are deliberately contained by Worker::deliver (tested
/// below). A panic while destroying the handler is outside that catch and
/// really unwinds the worker: explicit close must report its failed join.
#[test]
fn rust_ci_001_close_reports_unexpected_worker_panic() {
    struct PanicOnWorkerDrop;
    impl Drop for PanicOnWorkerDrop {
        fn drop(&mut self) {
            assert_ne!(
                std::thread::current().name(),
                Some(WORKER_THREAD_NAME),
                "deliberate unexpected worker teardown panic"
            );
        }
    }
    let peer = MockPeer::start();
    let mut runtime = connect(&peer);
    let panic_on_drop = PanicOnWorkerDrop;
    let (mut worker_exit, exit) = WorkerExit::new(false);
    let (seen_tx, seen) = mpsc::channel();
    let _handle = track::exchange_input_a::subscribe(&mut runtime, move |_: &SharedPayload| {
        let _keep_capture = &panic_on_drop;
        seen_tx.send(worker_exit.arm()).expect("test alive");
    })
    .expect("subscribe");
    peer.expect("SUB sub-1 {urn:test}MessageA input-topic");
    peer.send(r#"MSG sub-1 {"{urn:test}MessageA":{"Count":1}}"#);
    let worker = seen.recv_timeout(WAIT).expect("handler installed witness");
    let (closed_tx, closed) = mpsc::channel();
    let closer = std::thread::spawn(move || {
        closed_tx.send(runtime.close()).expect("test alive");
    });
    assert!(matches!(
        closed.recv_timeout(WAIT),
        Ok(Err(RuntimeError::WorkerPanicked))
    ));
    assert_eq!(exit.finished.try_recv(), Ok(worker));
    closer.join().expect("close application thread");
    peer.expect_closed();
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
