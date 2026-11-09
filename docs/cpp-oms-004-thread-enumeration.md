# CPP-OMS-004 live-thread observation

The immediate post-close assertion must test for live runtime workers, not
Linux task-directory unlink timing. No production runtime correction is needed:
`Core::close()` synchronously joins the I/O and dispatch workers, then destroys
the Asio context (including its resolver). These files and the PositionReport
test are byte-identical between CPP-OMS-007 base `a6a0f575` and its review head
`e5c4320e`.

On Linux 6.12.115, glibc pthread join completion can precede `/proc/self/task`
removal. A separate 20,000-join probe, with no broker or runtime, observed 14
residual named task entries under clang 19 TSan and 7 without instrumentation.
The worker completion flag was set and `join()` had returned in every case.
A subsequent stat-capturing probe observed `PF_EXITING` (`flags=0x40004c`);
some entries disappeared between reading comm and stat. A residual task may
still report `R`, not just `X`, so filtering the displayed state is insufficient.

Linux `do_exit()` sets `PF_EXITING` through `exit_signals()` before `exit_mm()`;
`mm_release()` clears the child TID and wakes the joiner before the later
`exit_notify()`/`release_task()` unlink. See Linux v6.12 `kernel/exit.c` and
`kernel/fork.c`. An exiting task cannot return to userspace to execute a callback
or runtime work. This is an observation race, not an additional shutdown grace
period or evidence that `join()` was omitted.

The helper now excludes only `PF_EXITING` tasks and tasks proven to have vanished
between reads. Other stat read/parse failures return -1, making zero-worker
assertions fail closed. Ordinary runnable, blocked and sleeping matching workers
continue to count. The immediate PositionReport assertion stays unchanged;
there is no timeout increase, test skip or permissive nonzero count.

`thread_enumeration_test.cpp` tests stat parsing, a deliberately blocked live
worker (must count as one), and 2,000 joined workers with completion flags.
`scripts/check-cpp-live-thread-enumeration.sh SCRATCH` passes all six g++/clang++
strict, ASan+UBSan and TSan configurations. The initial correction required
3 tests / 6007 checks; R1 extends the exact marker to 6 tests / 6094 checks.

Independent base/head PositionReport repetitions and stderr, exact exits,
sanitizer output, and thread snapshots are retained under
`/home/zboll/cpp-oms-007r1-evidence`. The original failing observation is kept;
passing reruns alone are not used to dismiss it. Independent review is required.

The identical assertion was reproduced on the original CPP-OMS-007 base
`a6a0f575ccdb6a850205d3e93e6fbd21823dfeaa` in isolated repetitions
`base-more-a/1`, `base-more-a/4`, and `base-more-b/4` (each exit 1, 1/247 failed).
The unmodified feature head failed `head-tsan-runs/4` in the same way. Initial
base runs (40 total) passed; this does not invalidate the later baseline
reproduction. Corrected-helper clang-TSan PositionReport passed 20/20 isolated
executions (2 tests/247 checks each) with identical runtime objects, broker,
schema, compiler and sanitizer flags. No sanitizer report was emitted.

## CPP-OMS-004R1: fail-closed observation review

Starting reviewed commit: `dfa38fd79d828ec672f0b8e351f4757b1a1379f3`;
tree: `ca120db15ca1a9f94de49e031cf5e73825a7a9cb`. It already contains main
`43fa1934235ae1f4c818a586a5022ddd7366b8d1`; no additional main merge was
needed. Existing commits and baseline reproduction evidence are preserved.

Independent review found two incomplete-observation paths in
`tests/cpp-lacal/check.hpp`:

* The task-name `getline()` result was unchecked. A failed open/read left an
  empty name that did not match the worker prefix, allowing a false zero count.
* A null `readdir()` result was always treated as successful end-of-directory,
  and `closedir()` failure was ignored.

The helper now checks name extraction and rejects empty names. A failed name
or stat read is ignored only if the existing task-directory existence check
proves ENOENT; any other outcome returns -1. Malformed stats still fail closed.
It clears errno immediately before **each** directory-read call and examines
errno immediately on a null result, before other I/O. Directory completion is
successful only with errno zero. All opened-directory paths converge on one
close; close failure also returns -1, including after an otherwise valid zero
count. No retry, sleep, timeout change or permissive nonzero check is introduced.

A small test-only `check::detail` I/O adapter permits deterministic failures
without globals, privileged operations, real `/proc` changes or timing races.
Normal callers still use `threads_named(prefix)`. Negative tests use the same
enumeration algorithm; name-open and name-read failures exercise the real
line-reader on `/dev/null/not-a-file` and `/dev/null`, respectively. Further
tests cover empty names, failed disappearance proof, vanished tasks, empty
and nonempty directories, mid-enumeration failure, close failure (also after
an earlier failure), malformed/unavailable stats, and stale errno from other
successful operations. Positive controls count R/S/D live tasks and exclude
only PF_EXITING tasks; the original parser and real blocked/joined worker tests
remain. Exact markers are 6 tests / 6094 checks in the script and consumer.

### R1 qualification

Evidence is retained in `/home/zboll/cpp-oms-004r1-evidence`; historical baseline
failures remain in `/home/zboll/cpp-oms-007r1-evidence` without modification.

| Configuration | Focused enumeration |
|---|---|
| g++ 14.2 strict C++17 | PASS, 6 tests / 6094 checks |
| clang++ 19.1.7 strict C++17 | PASS, 6 tests / 6094 checks |
| g++ ASan+UBSan | PASS, 6 tests / 6094 checks |
| clang++ ASan+UBSan | PASS, 6 tests / 6094 checks |
| g++ TSan | PASS, 6 tests / 6094 checks |
| clang++ TSan | PASS, 6 tests / 6094 checks |

All six completed with exit 0 and no sanitizer findings. Smoke-harness regression:
40 tests passed; local C++ PR smoke passed. Shared runtime qualification passed
all six configurations: OWP 6 tests/101 checks, mock 30/364, shutdown 12/217,
and OAM 1/13 per configuration. Live-worker and completed-shutdown failures
remain observable. Five compiled negative controls were all detected: unchecked
name, unchecked readdir, unchecked close, stale errno, and inverted live-worker
filtering (each executed test exited 1; compilation failures do not count).

Full CPP-OMS-004 PositionReport qualification passed both authentic pinned UCI
releases and all six configurations, including clang-TSan in 2.5 and 2.6:

| Configuration | UCI 2.5 | UCI 2.6 |
|---|---|---|
| g++ strict | PASS, 2 tests / 247 checks | PASS, 2 tests / 247 checks |
| clang++ strict | PASS, 2 tests / 247 checks | PASS, 2 tests / 247 checks |
| g++ ASan+UBSan | PASS, 2 tests / 247 checks | PASS, 2 tests / 247 checks |
| clang++ ASan+UBSan | PASS, 2 tests / 247 checks | PASS, 2 tests / 247 checks |
| g++ TSan | PASS, 2 tests / 247 checks | PASS, 2 tests / 247 checks |
| clang++ TSan | PASS, 2 tests / 247 checks | PASS, 2 tests / 247 checks |

No unavailable configuration or sanitizer findings. The unchanged standalone
script rebuilt runtime objects from source, authenticated both full schema
roots and SecurityMarkings, and verified original clean Sleet commit
`e38f61d8ce0d75c8508434a52f2ed77c69cf6a3b` / tree
`7e705f4eb5f658957bd414b4abc8207311f3c1e4`. Its numerical, locale and six mutation
controls passed: 27167 conversion and 20474 encoding oracle rows; per-release
codec consumers 253 checks/35 rejections. The immediate post-close assertion
remains exactly `threads_named("ams-lacal") == 0`.

Implementation commit `dfa85d44d68ecb8f71fc2cadbf45d405b0111739`, tree
`7547eeed727c70cafdec187a730dd840add4a520`, passed exact-head hosted PR smoke
run `42558512562` and trusted signals. This subsequent receipt-only commit gets
its own exact-head hosted inspection; prior green checks are not extrapolated.

Only this existing PR #200 branch is updated. Production LA-CAL runtime, broker,
codec, schema, immediate PositionReport assertion, and PR #198 are unchanged.
Final source identities and exact-head hosted status are attached to the PR
receipt after pushing ordinary corrective commits. Independent review remains
required; no approval, merge, auto-merge or history rewriting is performed.