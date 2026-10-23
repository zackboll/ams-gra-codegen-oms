# PERF-004 — pinned Task 065 qualification

## Scope and provenance

Starting fetched `origin/main` commit:
`23c8b2e4f7ebd600b8856b0e0e85d1c6eca25c39`.
Starting tree: `61f9e921e89133bd43b65071186567e3f0f9d5bf`.
The initial checkout was clean but behind main. A separate clean worktree was
created at `/home/zboll/git/ams-gra-codegen-oms-perf004`, on
`feature/perf-004-task065-parallel`. No prior PERF-004 branch or PR existed.
No PERF-001, PERF-002, or PERF-003 branch was changed or imported.

Historical successful hosted run:
<https://github.com/zackboll/ams-gra-codegen-oms/actions/runs/39699205279>,
same starting SHA. Its `real-uci` Task 065 step ran from
`2026-10-22T01:36:15Z` to `2026-10-22T01:57:11Z`: **20m56s**.
Local timings below are not predictions of GitHub runner performance.
That historical `real-uci` job ran 98m34s in total (00:30:09Z–02:08:43Z).
No whole-job improvement is attributed to PERF-004 until an optimized hosted
measurement exists; unrelated steps and hosted load must not be conflated.

## Measured bottlenecks

An unmodified original campaign and a separate original algorithm with
timers/source capture both passed, with matching complete deterministic
qualification output. The original source was retained using `git archive`;
the instrumentation-only patch is retained outside the repository.

| Original phase (seconds) | UCI 2.5 | UCI 2.6 |
|---|---:|---:|
| Root integrity | 0.006 | 0.006 |
| Schema loading | 5.788 | 5.682 |
| Hiding/collision/emission inventory | 11.377 | 11.304 |
| Both worlds, all backend coverage/preflight | 149.692 | 192.218 |
| Repeated closed-schema analysis construction | 38.971 | 47.964 |
| All 33 former-gap candidates | 53.594 | 55.771 |
| Minimum-service generation and actual model GNAT | 7.044 | 7.242 |
| Selected-service analysis/hiding | 8.212 | 9.709 |
| Production service-check (Ada/Rust/C++) | 17.622 | 17.348 |
| Production service-generate (Ada) | 6.025 | 5.832 |
| Actual model/API GNAT | 7.185 | 7.266 |
| Release total | 305.516 | 360.342 |

Coverage/preflight dominates, **not** the candidate loop. Therefore the change
also schedules the three independent per-language coverage evidence cells
with the same bounded scheduler, and retains the existing immutable closed
analysis for the subsequent dependency-closure checks. Neither world, any
language, nor any preflight/coverage assertion is omitted. Construction still
validates the same full schema and computes every existing preflight/index;
there is simply no second construction of the identical object.

The most expensive original selections include `Task` (3.611/3.668 seconds),
`MissionPlan` (2.748/2.846), and `MissionPlanCommand` (2.632/2.849), for
2.5/2.6 respectively. Per-message phase records are retained as JSONL.

Candidate phase sums (seconds, across 33 messages; these are not additional
release phases):

| Candidate phase | UCI 2.5 | UCI 2.6 |
|---|---:|---:|
| Dependency closure | 0.025 | 0.024 |
| Service resolution | 0.985 | 1.010 |
| Projection | 2.303 | 2.356 |
| Readiness | 29.454 | 30.668 |
| Explicit Ada preflight | 4.267 | 4.450 |
| Actual Ada generation | 16.502 | 17.207 |

## Architecture, integrity, and thread safety

Only the qualification harness changes. All candidates still call
`resolve_service_plan`, `project_service_generation_schema`,
`analyze_service_readiness`, `backend_preflight`, and `AdaBackend.generate`.
They retain the `QueryPET` dependency-closure assertion. No production source,
algorithm, naming rule, or generated API is changed.

`SchemaIr` owns strings, vectors, and ordinary value types. `CoverageAnalysis`
owns immutable maps/sets and references to that IR. Its construction populates
its indexes before sharing; analysis methods borrow immutably. Production
resolution, projection, preflight, and Ada generation use local state, not
mutable globals. The inspected interior-mutable coverage/projection probes
are `cfg(test)` thread-local counters in codegen-core's own unit-test build,
not this integration test's production dependencies. Scoped threads and Rust's
`Sync`/`Send` bounds enforce the sharing requirements without unsafe code.

One atomic original index assigns each item exactly once. Each worker retains
only its current projection/readiness/generated files and compact results;
full projected schemas or generated trees are not accumulated in memory.
Results carry their index and identity. Assembly rejects duplicate inputs,
missing results, duplicate results, and wrong identities/indices before any
successful result set is returned. Every worker is joined; a panic is resumed
on the caller, and operation errors fail the campaign. Nothing is detached.

Only the caller emits successful evidence, in the original order. Backend
cells are emitted Ada/Rust/C++, and candidates retain fixture order. All 33
projection and all 33 generation rows per release are required. The original
`(declaration_count, message_name)` sort, expected minimum assertion, and
compilation of **all minimum-cost ties** are unchanged. Authorization must
still have cost 230/231. Actual source generation and strict GNAT compilation
remain after ranking; readiness cannot replace compilation.

## Worker and memory policy

`AMS_GRA_TASK065_WORKERS` accepts exactly `1`, `2`, or `4`; unset initially
defaulted to two during initial qualification; the selected final default is
**four** based on the complete benchmarks below. Zero, negative, nonnumeric, whitespace-padded, and excessive
budgets fail closed. One worker runs on the caller thread as the serial
reference. A phase starts at most the requested number of scoped standard
library workers, with one candidate per worker. Coverage and candidate phases
never nest. There is no Rayon dependency or uncontrolled thread pool.

GNAT remains serial after candidate completion, so increasing candidate
concurrency does not multiply the compiler peak. Sampled process-tree memory
includes cargo, the harness, and compiler descendants. Summed RSS is an
upper-bound style metric that can count shared pages more than once; it is
not proportional-set-size. Sampling is every 100 ms, not an exact kernel
process-tree high-water mark.

## Complete candidate ledger

Each row represents **two distinct release-qualified candidates** (66 total).
Original and optimized readiness and actual generation passed for every entry. Costs retain
the exact selected-plus-support declaration count. Every one of these 66
release-qualified candidates has byte-identical generated sources and
identical evidence in all three optimized configurations.

| Message identity | UCI 2.5 cost | UCI 2.6 cost |
|---|---:|---:|
| Action | 578 | 578 |
| ActionCommand | 560 | 561 |
| ActionPlan | 956 | 966 |
| ActionPlanCommand | 952 | 962 |
| ApprovalPolicy | 633 | 634 |
| Authorization | 230 | 231 |
| AuthorizationRequest | 234 | 235 |
| CommSupport | 581 | 581 |
| CommSupportActivity | 258 | 258 |
| CommSupportPlan | 962 | 972 |
| CommSupportPlanCommand | 960 | 970 |
| Effect | 578 | 578 |
| EffectCommand | 560 | 561 |
| EffectPlan | 956 | 966 |
| EffectPlanCommand | 952 | 962 |
| MissionPlan | 994 | 1004 |
| MissionPlanCommand | 982 | 992 |
| PlanningFunction | 623 | 623 |
| PlanningFunctionSettingsCommand | 632 | 632 |
| PlanningFunctionStatus | 628 | 628 |
| RequirementOptions | 949 | 960 |
| RequirementOptionsCommand | 943 | 953 |
| ResponseCommand | 560 | 561 |
| ResponsePlan | 956 | 966 |
| ResponsePlanCommand | 952 | 962 |
| SupportPlan | 608 | 608 |
| SupportPlanRequest | 601 | 601 |
| SupportRequest | 592 | 592 |
| SystemsNeededRequest | 579 | 579 |
| Task | 1125 | 1126 |
| TaskCommand | 563 | 555 |
| TaskPlan | 956 | 966 |
| TaskPlanCommand | 952 | 962 |

Original completion totals: 33 projection rows and 33 actual Ada generation
rows per release; zero missing or duplicate identities. Both minimum-service
GNAT rows passed and both model/API compiler success markers passed.

## Reproduction and raw evidence

Raw data, logs, generated sources, source hashes, original archives, and
commands are outside the source tree at `/home/zboll/perf004-evidence`.
The machine has 32 logical CPUs (16 physical cores with SMT), GNAT 14.2.0,
and Rust/Cargo 1.98.1. `taskset -c 0-3` bounds each comparison campaign to
four logical CPUs. This is not a four-vCPU GitHub runner. There is no dedicated
memory cgroup limit; available system memory is recorded in the preflight.
`/tmp` had exhausted its inodes, so `TMPDIR` points to an evidence-local
directory on the home filesystem. No unrelated temporary data was deleted.

Pinned inputs were fetched with the unchanged repository scripts, checked
against the pinned Git revisions, and compared against the campaign copies.
Root SHA-256 values:

* UCI 2.5: `ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27`
* UCI 2.6: `af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b`

```bash
cd /home/zboll/git/ams-gra-codegen-oms-perf004
source /home/zboll/perf004-evidence/env.sh
export CARGO_TARGET_DIR=/home/zboll/perf004-evidence/target
export AMS_GRA_TASK065_WORKERS=2
export AMS_GRA_TASK065_PROFILE=/home/zboll/perf004-evidence/run-profile.jsonl
export AMS_GRA_TASK065_ARTIFACTS=/home/zboll/perf004-evidence/run-sources
taskset -c 0-3 bash scripts/check-task065-pinned.sh
```

`AMS_GRA_TASK065_PROFILE` opts into append-only JSONL phase durations.
`AMS_GRA_TASK065_ARTIFACTS` opts into saving actual per-candidate generated
sources. Use fresh external paths for each campaign. Neither writes to ordinary
qualification stdout or substitutes a qualification verdict. Timings do not
influence any assertion. Normalization for evidence comparison removes only
cargo/libtest harness lines, blank lines, and process-specific vertical PIDs;
no coverage, diagnostic, cost, readiness, source byte, or success row is
normalized. Original and instrumented original deterministic evidence matches;
the latter saved 198 actual generated files across 66 candidates.

Reproducibility helpers are retained in the evidence directory:
`measure.py` (wait4 aggregate user/system CPU and session-tree RSS sampling),
`env.sh`, `benchmarks.sh`, `instrument-original.py`, `profile-only.patch`, and
`compare.py` (full candidate/evidence/source/vertical comparison).
Original source-file SHA-256:
`130e9503b4caa407d94a151d98d6d00976c91840320e321e53ae8456f583d1c6`.
Instrumentation-only patch SHA-256:
`a6bfb8113c093f23f1399d1631c744e8b1e573bc33021e440f965bfb76c400db`.
Original 198-file source-hash ledger SHA-256:
`5f886d3fd9d6ed6067b6b46fac55d38b956d9e3f16c97638c22ca7c909f2357d`.

The initial optimized exploratory attempts were terminated after CPU topology
inspection showed that their disjoint logical CPU sets shared physical cores.
Their cancelled measurements are retained, excluded from performance claims.
Optimized benchmark configurations instead execute sequentially on CPUs 0–3.

## Qualification status

All required local pinned performance/equivalence campaigns passed.
Hosted qualification remains pending; no review-ready disposition is claimed.

| Whole-campaign metric | Original unmodified | Instrumented original |
|---|---:|---:|
| Wall seconds | 666.227 | 665.993 |
| User seconds | 663.732 | 664.053 |
| System seconds | 1.423 | 1.086 |
| Aggregate CPU utilization | 99.84% | 99.87% |
| Sampled peak process-tree RSS (MiB) | 952.77 | 954.61 |
| Former-gap generations | 66 | 66 |
| Missing cases | 0 | 0 |
| Actual GNAT model/API qualification | PASSED | PASSED |

Optimized one-worker reference: **581.639 wall seconds**, 578.658 user +
1.453 system seconds (580.111 aggregate CPU seconds), 99.74% utilization,
953.47 MiB sampled peak process-tree RSS. All 66 candidate costs/readiness
rows, all 198 candidate source files, both Authorization model/API source
trees, and complete normalized deterministic evidence match the original.
Every required actual GNAT compilation and wrapper marker passed. The serial
improvement (1.145× whole-campaign) comes from eliminating the identical
second closed-analysis construction, not from scheduling.

Optimized two-worker campaign: **482.212 wall seconds**, 608.850 user +
1.615 system seconds (610.465 aggregate CPU seconds), 126.60% utilization,
991.44 MiB sampled peak process-tree RSS. Complete equivalence and all actual
GNAT/compiler/wrapper gates passed, including all 66 candidates and 198 source
files. Candidate-phase wall time was 30.051 + 31.060 = 61.111 seconds versus
109.365 original (1.790×). Complete Task 065 speedup was 1.382×. Coverage-phase
wall time was 125.108/158.329 seconds, with release totals 223.546/258.537.
CPU time increases over the optimized serial reference while wall time drops;
parallel scheduling is not represented as a CPU-work reduction.

### Final comparison

| Metric | Original | 1 worker | 2 workers | 4 workers |
|---|---:|---:|---:|---:|
| Wall seconds | 666.227 | 581.639 | 482.212 | 416.684 |
| Aggregate CPU seconds | 665.155 | 580.111 | 610.465 | 605.802 |
| User seconds | 663.732 | 578.658 | 608.850 | 604.281 |
| System seconds | 1.423 | 1.453 | 1.615 | 1.521 |
| Effective CPU utilization | 99.84% | 99.74% | 126.60% | 145.39% |
| Sampled peak process-tree RSS (MiB) | 952.77 | 953.47 | 991.44 | 1011.18 |
| Former-gap candidates generated | 66 | 66 | 66 | 66 |
| Missing/duplicated candidates | 0/0 | 0/0 | 0/0 | 0/0 |
| Actual GNAT model/API qualification | PASSED | PASSED | PASSED | PASSED |
| Candidate wall seconds (both releases) | 109.365 | 115.053 | 61.111 | 32.473 |
| Candidate speedup vs original | 1.00× | 0.95× | 1.79× | 3.37× |
| Complete Task 065 speedup vs original | 1.00× | 1.15× | 1.38× | 1.60× |

One-worker candidate time varied somewhat (especially release 2.6); no
candidate speedup is claimed for serial mode. Four-worker release totals were
199.163/217.255 seconds, coverage/preflight 115.834/135.209, candidate campaigns
16.730/15.743. The 2× hotspot target was exceeded with four workers. The local
complete run is below 12 minutes, but the historical hosted runner is slower:
the hosted stretch target is **not** claimed from these local measurements.

Final default: **four workers**. It was 13.6% faster than two workers overall,
used slightly less aggregate CPU, and added only 19.74 MiB sampled peak tree
RSS. All strict qualification and negative controls passed. Compiler work is
not parallelized and still dominates the approximately 1 GiB process-tree
memory peak. Counts above four remain rejected; future larger self-hosted
budgets need new explicit qualification rather than unbounded auto-detection.

`compare.py` passed all three complete optimized comparisons: 66 candidates,
zero missing/duplicates, all 198 candidate source files byte-identical (SHA-256
ledger retained), identical costs/readiness/preflight/coverage/collision/hiding
evidence, identical Authorization model/API trees, unchanged minimum ranking,
both zero-gap worlds per release, and every GNAT/final wrapper success marker.
The original minimum-cost tie/compiler loop and final Authorization compiler
loop/GNAT-required gate were also checked byte-for-byte unchanged.

The two original campaigns overlapped on separate physical cores (0–3 and
4–7), with comparable results; optimized runs use CPUs 0–3 sequentially.
This is a shared developer host, not an exclusive benchmark appliance. Other
projects may run independently. The workspace validation suite also overlapped
part of the first optimized serial campaign. These limits are recorded rather
than treating sub-percent differences as significant.

Already passed: both original pinned campaigns; existing Task 065 Fast gates;
four scheduler controls; formatting, workspace all-target check, and Clippy
with warnings denied; CI split/layout guards including 210 split negative
controls. Synthetic controls cover reordered completions, exact-once work,
bounded concurrency, missing/duplicated/out-of-set results, operation failure,
invalid budgets, and panics without detached work.

The GNAT-required workspace suite passed: 1,442 tests, 179 test/doc groups,
eight pre-existing explicitly ignored external/preparation tests. Those ignored
tests are not claimed as locally qualified: Tasks 069/078/079 pinned probes,
Tasks 072/073 pinned verticals, and Tasks 076/077 broker/Sleet campaigns.
The unchanged Deep workflow owns its existing external payloads. No test was
newly ignored and Task 065's genuine pinned campaign executes separately.
Workspace all-target check and Clippy also passed on the workflow's Rust 1.95.0
toolchain, in addition to the local default toolchain.

Workflow YAML and the original pinned wrapper are unchanged. After feasible
local qualification, the final branch requires one explicit exact-head Deep CI
dispatch unless that head already has a run. Hosted timing/status and final
disposition must be recorded separately from the historical run.

## Follow-on candidates

Full-schema `CoverageAnalysis::new` remains serial and substantial. Production
readiness repeats projection and performs three-language preflight inside its
analysis even when measuring Ada. Backend spec/body generation repeats some
validation/planning. Reusing equivalent intermediate state would require
broader production API/capability-boundary changes and independent proof of
binding, world, and backend correctness; they are deliberately out of scope.