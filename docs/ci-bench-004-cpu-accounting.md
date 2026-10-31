# CI-BENCH-004 — raw CPU evidence and independent arithmetic

Infrastructure correction only. No benchmark is authorized/dispatched, historical
measurement requalified, PERF-009 source/correctness requirement changed, or
optimization merged. Run 41219887391 remains rejected. Its GNAT 14 artifact
13050453430 matches SHA-256
`847be818f0ba1212b670b3d7b672a88bf5b46c5f38082ac8f4edafaf82bf2afe`.
Starting main was `fc36c97f0c78a9fdae4f1c03715a5fbeb9db01c2`, including merged
PR #124. PR #121 remains separate at
`06f33bdda655b37f578a81bbca99ef7567407926`.

## Proven defects and limits

The original aggregate-first sampler can hide a single-CPU counter regression
or stall behind another CPU's advance. It does not check per-CPU tick/wall
consistency. It discards signed negative host-minus-cgroup residuals. These are
reproduced with adversarial fixtures, **not asserted as the cause of the failed
hosted run**. That archive lacks per-CPU endpoints/read timestamps needed for
historical attribution; the original 35 excursions remain failures.

## Source trust and acquisition

`scripts/ci_bench/cpu.py` reads only host `/proc/stat`, host cgroup v2 `cpu.stat`,
actual limits and `cgroup.procs`, using `CLOCK_MONOTONIC`. Nothing supplied by the
candidate is used as CPU accounting. `cpu_audit.py` separately parses and
recomputes evidence with rational arithmetic. It does not import producer CPU
calculation/parsing. The existing reconciler retains producer-attempt resolution,
archive SHA-256, packaging digests, upload receipts, source/workflow identity,
same-runner/compiler ABBA/resource validation and mandatory artifacts.

The host captures a zero-workload baseline **before Docker create/start**.
First-interval cgroup usage is cumulative from launch, not rebased to discovery.
Trusted acquisition is bracketed separately for host and cgroup reads. Missing
counters, replacement cgroup, long discovery or unknown boundaries fail closed.
The collector still sleeps one second between active samples; none are dropped.
Append-only bounded/flushed JSONL avoids rewriting the entire growing raw array.
Partial journals survive errors; they can never satisfy complete reconciliation.

A fixed non-root offline supervisor runs the worker as a child, waits for its
exit and then retains the cgroup while awaiting the original fixed
`accounted\n` acknowledgement. The host requires the cgroup contain **only the
trusted init PID** for the final sample. Candidate-reported result/readiness
cannot replace this host process/counter check. After acknowledgement/exit,
one more host interval covers shutdown, conservatively subtracting **zero**
workload CPU for that tail; supervisor/runtime tail CPU is not forgiven.
The terminal host endpoint is reused exactly as the next iteration's initial
endpoint, so cleanup and the next container create/start are covered by the
next interval with zero pre-launch workload subtraction. No inter-iteration
host interval is silently discarded.

The initially considered persistent parent-cgroup approach is **not used**:
it would require a broader host/systemd resource configuration change. The
quiescent supervisor plus conservative shutdown interval preserves Docker's
existing resource ceilings and acknowledgement mechanism without that change.

## Raw schema `ci-bench-cpu-v2`

Five extra files in each existing measurements artifact:
`cpu-preflight.jsonl` and `cpu-0.jsonl` through `cpu-3.jsonl`. Each journal is
bounded to 16 MiB; overflow fails, never truncates/downsamples. Existing transport
and expanded-archive bounds remain in force. `preflight_raw` / each row's
`cpu_raw` contains an exact `{schema,file,sha256,bytes}` reference to the journal;
the reconciler verifies it independently against the mandatory artifact member.
Raw records are not duplicated into progress/result JSON. Legacy derived-only artifacts are diagnostic, never valid
new evidence.

The concrete schema is the dictionaries emitted/validated by `cpu.py` and
independently consumed by `cpu_audit.py`:

- `context`: `schema`, complete producer `identity`, `allocation`, `iteration`
  (-1 preflight, 0–3 ABBA), exact `monitored_cpus`, actual `clk_tck`, independent
  host `getconf_clk_tck`, clock ID/origin/resolution, boot UUID, kernel release,
  `/proc/stat` source, cgroup mount and host cgroup namespace inode.
- `event`: `name`, trusted `at_ns` and event-specific identity. Ordered events:
  `coverage_start`, `create_requested`, `created` (full Docker container ID),
  `start_requested`, `cgroup_discovered` (path/device/inode/PID), `worker_exited`
  (final snapshot index), `accounted`, `container_exited`, `coverage_end`.
  Preflight has coverage start/end only.
- `snapshot`: contiguous zero-based `sequence`, lifecycle `state`, `host` and
  `cgroup`. `host` has read `begin_ns/end_ns`, exact monitored raw CPU lines and
  matching ten-field `cpus` map. `cgroup` has its read bracket, canonical host
  Docker path/device/inode, complete raw `cpu.stat` and matching complete counter
  map, host PID set, actual CPU/cpuset/memory/swap/pids limits. It is null only
  before launch, preflight and conservative post-exit shutdown.
- Published intervals reference both endpoint sequence IDs and preserve wall
  elapsed, per-CPU ten-field deltas and total/idle/busy/steal ticks, aggregate
  ticks, usage/throttle deltas and cumulative counters, signed residual,
  resolution bound, feasible boundary elapsed min/max, source schema/sequence,
  host-only flag, normalized external busy and steal.

Raw duplicate keys, malformed/truncated journal, missing/duplicate CPU lines,
unexpected parsed monitored IDs, changing field inventories, nonfinite/negative
counters, wrong identities, reordered/gapped sequences or lifecycle coverage,
and raw/derived disagreements reject independently of the producer's gate.

## Equations and justified consistency bounds

Let H be actual `SC_CLK_TCK`, N the number of monitored logical CPUs, U cgroup
usage delta in microseconds, and V[c,f] per-CPU field differences. First eight
fields are user/nice/system/idle/iowait/irq/softirq/steal; guest and guest_nice are
retained/checked but excluded from total because Linux already counts them in
user/nice.

```text
T[c] = sum(V[c,0:8])
I[c] = V[c,idle] + V[c,iowait]
B[c] = T[c] - I[c]
S[c] = V[c,steal]
signed_residual_seconds = sum(B[c])/H - U/1,000,000
external_busy_fraction = max(0, signed_residual_seconds*H) / (sum(T[c])/N)
steal_fraction = sum(S[c]) / sum(T[c])
```

**No normalization or threshold change:** each interval must have external busy
<=5%, steal <=1%, and cgroup throttle delta <=1% of its elapsed interval.
No averaging or quantization allowance is applied to those acceptance limits.

Each endpoint acquisition spans [begin,end]. It must fit within one USER_HZ
unit, `end-begin <= 1/H` seconds. Feasible interval length is
`L=(next.begin-prev.end)` through `R=(next.end-prev.begin)` seconds, with ordered
non-overlapping brackets. Every logical CPU must advance and satisfy
`L*H - 8 <= T[c] <= R*H + 8`. Eight is not an empirical percentage: the delta of
two independently truncated tick counters has error <1 tick per non-overlapping
field, across eight total fields. It detects per-CPU stalls/impossible excess,
not only aggregate behavior. The one-unit acquisition cap bounds observed read
skew; it does **not** assert a bound on kernel deferred accounting.

For signed residual integrity, the six busy fields per logical CPU give a
counter-resolution envelope of `6*N/H` seconds. Acquisition skew adds at most
`N*(width_before+width_after)` CPU-seconds; microsecond workload endpoints add
2 microseconds. A residual less than minus this derived bound is rejected,
never clamped into a clean result. The signed value and the complete bound are
retained even inside the resolution envelope. User/system/usage cgroup deltas
must agree within three microseconds (three separately truncated fields),
throttled periods cannot exceed total periods, counters cannot regress/change
field sets, and usage cannot exceed two CPUs times feasible elapsed capacity
plus two endpoint microseconds.

These are conservative **consistency** bounds, not claimed hardware error bars.
They do not establish physical isolation, compensate unknown accounting lag,
permit interpolation, or attribute activity to a host process. Linux explicitly
documents unreliable iowait/decreasing values; such data is retained but rejected
as unqualified rather than silently repaired. Unknown platform accounting
behavior can still make an interval unqualified. H/getconf agreement is a trusted
host consistency check, not an independent hardware attestation.

## Security, validation and review

No workflow, action pin, source allowlist, five main lanes, job matrix, permissions,
Docker network/rootfs/user/capability/socket restrictions, worker count, quota,
memory/swap/process ceiling, compiler ABBA or production correctness gate changes.
New executable sources are added to the existing benchmark-only checksum
inventory; inherited policy pins are preserved. Tests are picked up by the
existing mandatory `test_ci_select*.py` discovery without editing workflow jobs.

New tests include clean startup/shutdown positives, masked reversal/stall,
impossible ticks, negative residual, endpoint skew, missing lifecycle intervals,
sequence corruption, bad HZ, missing SMT, truncation, cgroup identity/split/limits,
steal/throttle/external-busy failures, raw-journal identity and full-bundle mutation
with producer validation disabled. They test the independent consumer, not only
producer arithmetic. Existing benchmark/provenance/security tests remain.

Independent review must examine the two arithmetic implementations, lifecycle
coverage, kernel counter semantics and conservatively treated shutdown tail.
Local synthetic tests and ordinary hosted Fast/Deep CI do **not** qualify a live
benchmark environment or prove counter freshness. No benchmark is launched to
test this change. A future benchmark needs new explicit authorization, qualified
environment review and all original gates. Do not move candidate execution to
an unqualified self-hosted runner.

Kernel references:
<https://docs.kernel.org/filesystems/proc.html> and
<https://docs.kernel.org/admin-guide/cgroup-v2.html>.