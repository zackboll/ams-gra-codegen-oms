//! Executable test-only procfs controls; no schemas, downloads or runtime mocks.
#[path = "../examples/rust-uci-client/tests/support/task_workers.rs"]
mod task_workers;

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, mpsc};
use task_workers::{application_workers, workers_at};

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "rust-task-workers-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn task(&self, tid: u32, comm: &str, record: &str) -> PathBuf {
        let path = self.0.join(tid.to_string());
        std::fs::create_dir(&path).unwrap();
        std::fs::write(path.join("comm"), format!("{comm}\n")).unwrap();
        std::fs::write(path.join("stat"), record).unwrap();
        path
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}
fn stat(tid: u32, comm: &str, state: &str, flags: &str) -> String {
    let mut fields = vec!["0"; 50];
    fields[0] = state;
    fields[6] = flags;
    format!("{tid} ({comm}) {}\n", fields.join(" "))
}

#[test]
fn runnable_blocked_and_sleeping_workers_count_without_exiting_flag() {
    let fixture = Fixture::new();
    for (index, state) in ["R", "D", "S", "T", "t", "I"].into_iter().enumerate() {
        let tid = u32::try_from(index + 1).unwrap();
        fixture.task(
            tid,
            "ams-gra-oms-lac",
            &stat(tid, "ams-gra-oms-lac", state, "4194368"),
        );
    }
    assert_eq!(
        workers_at(&fixture.0),
        (1..=6).map(|id| id.to_string()).collect()
    );
}

#[test]
fn only_kernel_exit_flag_excludes_even_runnable_entries() {
    let fixture = Fixture::new();
    for (index, state) in ["R", "S", "D", "X", "Z"].into_iter().enumerate() {
        let tid = u32::try_from(index + 1).unwrap();
        fixture.task(
            tid,
            "ams-gra-oms-lac",
            &stat(tid, "ams-gra-oms-lac", state, "4194380"),
        );
    }
    assert!(workers_at(&fixture.0).is_empty());
    // A state character alone must never grant tolerance.
    fixture.task(6, "ams-gra-oms-lac", &stat(6, "ams-gra-oms-lac", "X", "0"));
    assert_eq!(workers_at(&fixture.0), BTreeSet::from(["6".into()]));
}

#[test]
fn comm_parentheses_and_spaces_do_not_shift_flags() {
    let fixture = Fixture::new();
    let comm = "ams-gra-oms-lac ) ( ) worker";
    fixture.task(9, comm, &stat(9, comm, "S", "0"));
    assert_eq!(workers_at(&fixture.0), BTreeSet::from(["9".into()]));
}

#[test]
fn malformed_extant_metadata_and_identity_mismatch_fail_closed() {
    for record in [
        "".into(),
        "7 (ams-gra-oms-lac) R 1".into(),
        stat(7, "ams-gra-oms-lac", "?", "4"),
        stat(7, "ams-gra-oms-lac", "R", "-1"),
        stat(7, "ams-gra-oms-lac", "R", "4junk"),
        stat(8, "ams-gra-oms-lac", "R", "4"),
        stat(7, "wrong comm", "R", "4"),
    ] {
        let fixture = Fixture::new();
        fixture.task(7, "ams-gra-oms-lac", &record);
        assert!(
            std::panic::catch_unwind(|| workers_at(&fixture.0)).is_err(),
            "{record}"
        );
    }
}

#[test]
fn unreadable_extant_metadata_fails_closed_but_disappeared_task_is_absent() {
    for name in ["comm", "stat"] {
        let fixture = Fixture::new();
        let task = fixture.task(7, "ams-gra-oms-lac", &stat(7, "ams-gra-oms-lac", "R", "0"));
        std::fs::remove_file(task.join(name)).unwrap();
        assert!(std::panic::catch_unwind(|| workers_at(&fixture.0)).is_err());
        std::fs::remove_dir_all(task).unwrap();
        assert!(workers_at(&fixture.0).is_empty());
    }
}

#[test]
fn deliberately_blocked_live_worker_negative_control() {
    let pid = std::process::id();
    let before = application_workers(pid);
    let (ready_tx, ready) = mpsc::channel();
    let (release_tx, release) = mpsc::channel();
    let worker = std::thread::Builder::new()
        .name("ams-gra-oms-lacal".into())
        .spawn(move || {
            ready_tx.send(()).unwrap();
            release.recv().unwrap();
        })
        .unwrap();
    ready.recv().unwrap();
    let blocked = application_workers(pid);
    assert_eq!(
        blocked.difference(&before).count(),
        1,
        "live blocked worker was hidden"
    );
    release_tx.send(()).unwrap();
    worker.join().unwrap();
    assert_eq!(application_workers(pid), before);
}

#[test]
fn runnable_live_worker_negative_control() {
    let pid = std::process::id();
    let before = application_workers(pid);
    let running = Arc::new(AtomicBool::new(true));
    let flag = running.clone();
    let (ready_tx, ready) = mpsc::channel();
    let worker = std::thread::Builder::new()
        .name("ams-gra-oms-lacal".into())
        .spawn(move || {
            ready_tx.send(()).unwrap();
            while flag.load(Ordering::Acquire) {
                std::hint::spin_loop();
            }
        })
        .unwrap();
    ready.recv().unwrap();
    assert_eq!(application_workers(pid).difference(&before).count(), 1);
    running.store(false, Ordering::Release);
    worker.join().unwrap();
    assert_eq!(application_workers(pid), before);
}

#[test]
fn repeated_joined_threads_have_no_live_worker_without_grace_period() {
    let pid = std::process::id();
    let before = application_workers(pid);
    for _ in 0..20_000 {
        std::thread::Builder::new()
            .name("ams-gra-oms-lacal".into())
            .spawn(|| {})
            .unwrap()
            .join()
            .unwrap();
        assert_eq!(application_workers(pid), before);
    }
}
