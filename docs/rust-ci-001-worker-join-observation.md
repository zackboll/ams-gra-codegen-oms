# RUST-CI-001: actual-worker shutdown observation

## Scope and source

Separate corrective branch based on fetched main
`ac046599c492241aefc9b1d6b742668c0e9cd39a`, tree
`d04978dc1087b40d842210724a04c458f425edbd`.
No production runtime, dependency, CI workflow, runner, selection, provider or
PERF-009 change. PR #121 and its worktree/evidence remain untouched.

The production `SleetRuntime::stop()` sends its shutdown oneshot, takes the
actual worker's `JoinHandle`, and joins it (except its documented same-worker
drop case). Explicit close maps a failed join to `RuntimeError::WorkerPanicked`;
normal application-thread Drop invokes the same stop path best-effort.

## Independent review and before/after reproduction

R2's hashed failing pthread traces and the standalone POSIX probe were checked
against its manifest, not accepted solely from prose. The same worker callback
returned, real pthread_join succeeded, and the original assertion then counted
its lingering procfs task name. Linux clears the child-TID/futex used by glibc
join before completing kernel task removal. `/proc/self/task` disappearance is
therefore not a synchronous worker-lifecycle contract. The independent C probe
recorded 501 visible task entries after 1,000 successful joins.

Fresh main and branch binaries were built with Rust 1.95.0
(`59807616e1fa2540724bfbac14d7976d7e4a3860`, LLVM 22.1.2), locked dependencies.
Five runs per test binary/allocation were retained, including every failure:

| Binary | main multi failures | main CPU-0 failures | corrected multi / CPU-0 failures |
|---|---:|---:|---:|
| fairness | 1/5 | 2/5 | 0/5 / 0/5 |
| lifecycle | 0/5 | 0/5 | 0/5 / 0/5 |
| initial retry | 0/5 | 3/5 | 0/5 / 0/5 |

These are bounded focused executions on shared hardware, not statistically
exhaustive or exclusive-core measurements. A passing full-main workspace run
does not invalidate its separately reproduced focused failures.

## Replacement evidence

Fairness retains publisher progress, MSG delivery during load, late server
error, unsubscribe, exact publication totals/bytes, close success and connection
closure. Its generated handler installs a test-only thread-local exit witness
and records the actual worker's Rust ThreadId. Unsubscribe cannot complete that
witness: it is in worker TLS, not the subscription capture. Immediately after
close returns, the witness must already be available (`try_recv`, no retries).

The discriminating lifecycle tests exercise close and normal Drop separately:

1. A real generated subscription callback records its worker ThreadId/name and
   blocks on a bounded test-controlled channel barrier.
2. Another application thread announces its shutdown attempt and calls close
   or Drop. Shutdown must not complete while the callback is held.
3. The controller releases the callback and requires its completion record.
4. The same worker enters a second, bounded barrier in its TLS destructor.
   Shutdown must still not complete. This distinguishes actual join from
   waiting for callback completion, connection closure, or Rust
   `JoinHandle::is_finished()` followed by detachment.
5. The controller releases TLS teardown; successful shutdown must return,
   and the same worker's completion witness must already exist.
6. The application shutdown thread is joined, the peer must observe closure,
   callback-channel disconnection proves dispatch captures are gone, and the
   old subscription cannot execute another command (`WorkerStopped`).

Every introduced receive is bounded by the existing WAIT (5 seconds); each
non-completion observation uses the existing QUIET (300 ms). The bounded quiet
window gives a faulty application thread an opportunity to return; it is not
a sleep used to make procfs eventually disappear or a passing retry. As with
any black-box concurrency test, a thread not scheduled during the observation
window can limit detection; no test claims an adversarial scheduler proof.
Positive completion additionally requires causal callback/TLS records and
successful close. There is no OS thread enumeration or production test hook.

A separate generated-handler-capture destructor deliberately panics outside
Worker::deliver's handler-panic containment. Actual close must report
`WorkerPanicked` via a bounded application-thread result, not claim success.
The normal Drop contract cannot surface a panic; its joining is covered by
the two-barrier test instead. Self-drop inside a handler is explicitly outside
these application-thread join regressions.

The existing lifecycle procfs count was removed in favor of those tests.
Initial-retry's immediate procfs count independently failed 3/5 CPU-0 runs, so
it was also removed; refusal/error provenance, INIT rejection, no-reconnect,
no-replay and generated publish controls remain intact. Failed-connect join
paths are unchanged and reviewed, but the new generated-handler TLS witness
cannot be armed before a failed handshake. No claim of new deterministic
failed-connect join instrumentation is made.

## Disposable negative controls

All mutations live outside the branch in disposable uncommitted source
directories. Each compiled before execution, and each test exited 101 rather
than being credited for compiler rejection or an external timeout:

| Defect | Rejection |
|---|---|
| close drops active JoinHandle | shutdown returned at handler barrier |
| Drop signals but detaches | shutdown returned at handler barrier |
| asynchronous join permits callback after purported close | handler barrier |
| missing shutdown signal (sender retained) | no TLS exit within WAIT |
| worker shutdown branch never terminates | no TLS exit within WAIT |
| failed join/panic discarded as success | expected WorkerPanicked missing |
| wait for is_finished then detach | **TLS teardown barrier** |

The intentionally defective runtime sources are not committed. A correct
runtime never consults procfs in these tests, so successful join plus temporary
procfs visibility is not classified as a failure.

A separate diagnostic-only preload observed the correct worker immediately
after successful pthread_join during five CPU-0 corrected fairness executions.
All five tests passed, including **four runs with the joined worker's procfs
name still visible**. This is a positive executable control, not just an
inference from removing procfs reads. It does not modify names or join results.

## Qualification and evidence

Full Rust 1.95 locked workspace runs passed on main and the corrected branch.
Both sources passed fmt, locked workspace/all-target check and Clippy with
warnings denied. Policy, selector/final-gate security regressions, CI002,
Task077B security layout, compiler placement, split controls and pinned
actionlint are retained separately; final result records distinguish completed
checks from pending/incomplete attempts. A runner interrupted by an outer
tool timeout is not a successful check and is rerun under a background recorder.

All command arguments, complete stdout/stderr, exit codes, binary SHA-256,
source SHA/tree, synchronization markers, disposable mutation diffs, and
before/after failed and passing trials are retained under
`/home/zboll/git/ams-gra-codegen-oms-rust-ci-001-evidence`.
Compact results are in
`/home/zboll/git/ams-gra-codegen-oms-rust-ci-001/docs/rust-ci-001-results.json`.
The final commit/tree is recorded by the PR handoff, not embedded
self-referentially. Exact-final-head hosted CI remains required; until it
completes and is checked, disposition is **HOSTED_CI_PENDING**, not approval or
merge readiness. No merge, approval or auto-merge is authorized.