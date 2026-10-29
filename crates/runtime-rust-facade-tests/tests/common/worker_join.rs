//! Test-only evidence from the actual worker's thread-local teardown, not
//! procfs task removal (which can lag a successful pthread join on Linux).
use super::{QUIET, WAIT};
use std::cell::RefCell;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, ThreadId};

thread_local! {
    static EXIT: RefCell<Option<ExitSignal>> = const { RefCell::new(None) };
}

struct ExitSignal {
    entered: Sender<ThreadId>,
    release: Option<Receiver<()>>,
    finished: Sender<ThreadId>,
}

impl Drop for ExitSignal {
    fn drop(&mut self) {
        let id = thread::current().id();
        let _ = self.entered.send(id);
        if let Some(release) = &self.release {
            // Do not panic in a TLS destructor. A missing release withholds
            // the completion witness and fails the bounded application wait.
            if release.recv_timeout(WAIT).is_err() {
                return;
            }
        }
        let _ = self.finished.send(id);
    }
}

pub struct WorkerExit {
    signal: Option<ExitSignal>,
}

pub struct ExitWitness {
    pub entered: Receiver<ThreadId>,
    pub release: Option<Sender<()>>,
    pub finished: Receiver<ThreadId>,
}

impl WorkerExit {
    pub fn new(block_exit: bool) -> (Self, ExitWitness) {
        let (entered_tx, entered) = mpsc::channel();
        let (finished_tx, finished) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        (
            Self {
                signal: Some(ExitSignal {
                    entered: entered_tx,
                    release: block_exit.then_some(release_rx),
                    finished: finished_tx,
                }),
            },
            ExitWitness {
                entered,
                release: block_exit.then_some(release_tx),
                finished,
            },
        )
    }

    /// Call only from the generated handler. The witness is stored in TLS,
    /// not the subscription capture: unsubscribe must not falsely signal exit.
    pub fn arm(&mut self) -> ThreadId {
        if let Some(signal) = self.signal.take() {
            EXIT.with(|slot| {
                assert!(slot.borrow().is_none(), "one witness per actual worker");
                *slot.borrow_mut() = Some(signal);
            });
        }
        thread::current().id()
    }
}

pub fn assert_still_blocked<T: std::fmt::Debug>(done: &Receiver<T>, phase: &str) {
    assert!(
        matches!(
            done.recv_timeout(QUIET),
            Err(mpsc::RecvTimeoutError::Timeout)
        ),
        "shutdown returned while actual worker was blocked in {phase}"
    );
    eprintln!("WORKER_JOIN_BLOCKED phase={phase}");
}
