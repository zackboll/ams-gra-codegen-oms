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
strict, ASan+UBSan and TSan configurations, each 3 tests / 6007 checks.

Independent base/head PositionReport repetitions and stderr, exact exits,
sanitizer output, and thread snapshots are retained under
`/home/zboll/cpp-oms-007r1-evidence`. The original failing observation is kept;
passing reruns alone are not used to dismiss it. Independent review is required.