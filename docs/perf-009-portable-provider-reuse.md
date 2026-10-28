# PERF-009 — portable provider reuse experiment

## Current disposition: PERFORMANCE_ENVIRONMENT_BLOCKED (R2)

Portable exact-head correctness and mandatory Rust acceptance also remain
incomplete; this is not a functional qualification pass. Historical checkpoint
dispositions below remain preserved.

**R1 update:** see the appended R1 section for portable live controls,
read-time configuration hardening, benchmark contention rejection and a
mandatory Rust regression failure. Neither original CI command is replaced.

This is a **partial implementation and qualification checkpoint**, not the
requested completed CI rollout. Neither portable CI command is changed. Do not
merge this checkpoint as evidence that PERF-009's acceptance criteria passed.
The experiment passes both complete positive portable matrices and demonstrates
substantial repeated provider work, but mandatory negative controls, controlled
performance acceptance and exact-final-head hosted qualification are incomplete.
No performance target or readiness disposition is asserted.

## Isolation and source identity

Fetched `origin/main` before creating the branch:

- Starting commit: `ac046599c492241aefc9b1d6b742668c0e9cd39a`.
- Starting tree: `d04978dc1087b40d842210724a04c458f425edbd`.
- PERF-005 / PR #109 is merged at
  `86ffa50d38f4462a5b23a3714ec183cd79dc35df`.
- PERF-007 / PR #120 is merged at the starting commit above.
- Both merge commits were checked as ancestors of `origin/main`.
- Isolated worktree:
  `/home/zboll/git/ams-gra-codegen-oms-perf-009-portable-provider`.
- Branch: `feature/perf-009-portable-provider-reuse`.
- Raw evidence:
  `/home/zboll/git/ams-gra-codegen-oms-perf-009-evidence`.

The production generator was built with Rust 1.95.0, `--locked`, from the
starting source. The candidate ran with uncommitted experimental scripts; its
recorded Git commit/tree identify the production baseline, **not** a clean
final implementation head. Later provenance fields explicitly report dirty
state and hash the experiment tooling. Final commit/tree are available from
the PR head, but no exact-final-head complete campaign or hosted success is
claimed. Existing worktrees, PRs and uncommitted changes were not changed.

## Original graph and repeated work

The authoritative source remains
`/home/zboll/git/ams-gra-codegen-oms-perf-009-portable-provider/scripts/check-task093-helper-demand.py`.
It creates ten independent naming/namespace directories, each owning a support
copy, private dependency deployment and three forced builds:

1. `ArraySample`
2. `Array_Encode_Hex_Binary`
3. `Encode_Hex_Binary`
4. `Array_encode_hex_binary`
5. `Array_Decode_Hex_Binary`

Order is OAM then qualified namespace; Normal, Check, Ignore within each name.
Every executable must emit exactly
`TASK093_C4_HELPER_DEMAND_COMPLETE checks= 7` followed by a newline.

The consumer imports portable `ams_gra_oms_json`, which imports GNATCOLL Core
and Minimal 26.0.0. `alr build -- -f` recompiles both providers in every policy
cell. Live traces show **106 provider compiler invocations per cell** (89 Ada
and 17 C/assembly), plus five owned compilations and one binder-generated
compilation. This repeated provider work is the bottleneck.

### Independently derived portable inventory

Portable compilation is **not** PERF-005's eight-source Float32 inventory:

- `helper_demand_consumer.adb`
- `service_codec.adb` (with its spec)
- `programs.ads`
- `programs-oam.ads` or `programs-task093.ads`
- `src/ams_gra_oms_json_guard.adb` (with its spec)

`Service_API` is generated but not imported by this executable. It remains in
the byte-comparison inventory. Portable support imports only `src`, not the
Float32 support project. An independent `gprls --closure` inspection agreed
with this graph. Unexpected/removed source names fail the experiment verifier.

| Count per compiler | Original | Experiment |
|---|---:|---:|
| Positive cells / executions | 30 | 30 |
| Core+Minimal preparation sets | 30 | 3 |
| Individual library constructions | 60 | 6 |
| Actual provider compiler invocations | 3,180 | 318 |
| Actual owned compiler invocations | 150 | 150 |
| Binder-generated compiler invocations | 30 | 30 |
| Binder executions / final links | 30 / 30 | 30 / 30 |

Both original campaigns ran to completion before either optimized positive
campaign. Original script and logs were retained separately. No existing
Float32 qualification was substituted for portable qualification.

## Implementation boundary

New explicit experiment entry points:

- `/home/zboll/git/ams-gra-codegen-oms-perf-009-portable-provider/scripts/perf009-baseline.py`
  instruments the original full portable runner, without provider reuse.
- `/home/zboll/git/ams-gra-codegen-oms-perf-009-portable-provider/scripts/perf009-portable.py`
  adapts that runner's `checked` boundary using the existing `Pilot` abstraction.
- `/home/zboll/git/ams-gra-codegen-oms-perf-009-portable-provider/scripts/perf009_portable.py`
  implements portable source accounting and full ordered outcome comparison.
- `/home/zboll/git/ams-gra-codegen-oms-perf-009-portable-provider/scripts/test_perf009_portable.py`
  exercises inventory, completeness, policy, compiler, freshness and bind/link
  verifier rejection.

The original runner, PERF-005 implementation, production code, workflow files,
job IDs, runners, main-lane scheduling, selector and security contracts are
unchanged. There is no CI mode or successful fallback. Partial profiles,
policies, representative subsets and intentional-red switches are not accepted
by the experiment interface. Its successful experiment marker explicitly says
`NOT_CI_QUALIFIED`; its report retains `QUALIFICATION_FAILED` while mandatory
controls remain outstanding.

The experiment uses existing campaign-local PERF-005 storage names internally;
this is shared code reuse, not cross-campaign artifact reuse. Each compiler has
a separately copied Alire state and fresh provider artifacts. The new adapter
requires a distinct original state with the same `seed.json` and a generator
hash matching the original source manifest. Alire states include only copied
index/toolchain seeds, not precompiled providers.

## Effective provider input proof observed locally

Local Linux x86_64 / glibc 2.41; external GNAT 14.2.0 and Alire GNAT 16.1.0;
GPRbuild crate 26.0.1 (actual GPRBUILD 26.0.0); verified release Alire 2.1.1.
Private index seed: `842d7165e3e1e644f4d03563e7a4f35c05415af4`.
The initial system Alire binary was not used as the qualified executable: the
workflow-pinned archive was downloaded, SHA-256 verified and extracted privately.
Verified binary SHA-256:
`2e2b8de009b793ad2aed10096e162649bbcc5b0711e3c21785ebe8a1261c5d64`.

The existing provider abstraction checks compiler executable/hash, runtime
`system.ads` and `libgnat.a`, GPRbuild hash, dependency source/project contents,
26.0.0 release archive origin hashes, and recorded environment/project
variables. Check and Ignore providers each receive their own exact ADC;
Normal has no assertion-policy override. No objects cross compiler or policy
boundaries. Provider manifests are checked before and after dependent builds;
read-only modes and Landlock prevent dependent compiler processes from writing
the provider tree.

Independent disposable preparation experiments passed the existing concrete
read-time input reconciliation for **all six compiler/policy variants**:
106 provider invocations and 89 Ada mappings each. Final positive experiment
preparations were also reconciled, rather than trusting the earlier artifacts.
GPR-generated configuration, recursive response/specs and ADC contents remain
in effective arguments. Pre-execution map bytes come from actual `gnat1` reads,
not mutable post-build mapping files. Collision checks cover both unit and
filename keys. Only concrete qualified-runtime cache differences are reconciled
with identical paired source-read bytes. No map is filtered before compilation.

The first two independent OAM naming cases were additionally compared under
both compilers and all policies: each had 106 matching non-map effective inputs
and identical read source bytes after naming-directory relocation. Other naming
cases passed source/configuration identity gates and their full consumers, but
an exhaustive independent cross-naming mapping/replay campaign is outstanding.
These observations are **not blanket proof for new provider graphs/ABIs**.

## Fresh consumer evidence and semantics

Actual GCC drivers and `gnat1` backends are required for each of the five owned
sources; missing, duplicate and unexpected sources fail. Effective ADC contents
and `-gnat2022 -gnato -O0` are checked. Actual successful `gnatbind` and linker
executions, fresh object/ALI timestamps and a fresh executable are required.
Source and artifact hashes are retained per cell.

Both full baseline/candidate pairs passed ordered result, lock, policy and
generated Ada byte comparison, readiness checks and all 30 exact output markers.
The unchanged authoritative runner also enforces:

- no unwanted Binary helpers or Binary_Vectors;
- no Float32 platform dependency or occurrence ingress;
- exact schema-derived identifier spelling;
- byte-identical model/API between codec-disabled and codec-enabled output;
- portable output unchanged when the optional runtime argument is supplied.

Portable qualification does not have Float32's twenty negative runtime-CLI
cases. This checkpoint does not manufacture or report those as portable tests.
Existing negative generator and compiler implementations remain unchanged.

## Negative-first results and incomplete controls

Disposable genuine GNAT builds, independent of positive evidence, passed these
controls for each compiler and all three policies (**18 rejected builds**):

| Mutation | Actual observation |
|---|---|
| Reference an undeclared helper | GNAT undeclared helper diagnostic, nonzero driver |
| Add `Model.Binary_Vectors.Vector` | GNAT `Binary_Vectors` not declared diagnostic |
| Plant `Compile_Time_Error` in portable guard | GNAT rejection proves support is compiled |

Synthetic regression fixtures separately reject missing/duplicated compiler
invocations, missing backend/bind/link execution, wrong effective assertion
policy, changed compiler path, stale timestamps, unforced builds, wrong family,
partial/duplicated/reordered cells and changed output markers. These fixtures
are verifier unit tests, **not fabricated GNAT diagnostics**.
Existing PERF-005 artifact corruption, identity, mapping and sandbox tests pass.

Still required before enabling portable CI reuse: a complete independently
executed portable corruption suite covering changed provider sources, indirect
mapping/configuration, missing artifacts, falsely successful preparation,
post-preparation mutation, stale-object bypass, truncated evidence and skipped
consumer evidence; actual policy/forced-build controls tied to the final head;
and complete independent provider/source/ABI replay checks. Passing inherited
unit tests is not substituted for these mandatory portable live controls.

## Local measurements (instrumented attribution, not accepted benchmarks)

Both compilers received four-CPU affinity allocations, independent Alire states
and the same index/toolchain seed. Baselines and candidates were traced, but
other workloads ran on this shared machine, and the baseline used original
default GPR parallelism while provider preparation explicitly used four jobs.
Thus **these are not comparable clean acceptance runs**. Tracing, source-read
parsing and verification overhead are substantial. Post-campaign provider
equivalence analysis is excluded from candidate campaign elapsed time below.

| Metric | GNAT 14 original | GNAT 14 experiment | GNAT 16 original | GNAT 16 experiment |
|---|---:|---:|---:|---:|
| Campaign elapsed seconds | 1,041.705 | 211.153 | 1,236.916 | 273.372 |
| Child CPU seconds | 2,025.648 | 215.047 | 2,640.539 | 274.747 |
| Harness self CPU seconds | 564.083 | 104.823 | 630.293 | 151.237 |
| Sum of build-command elapsed seconds | 700.465 | 115.728 | 855.629 | 144.180 |
| Sum of build-command child CPU seconds | 2,010.069 | 196.250 | 2,623.968 | 254.288 |
| Sampled peak build process-tree RSS, KiB | 2,584,456 | 724,896 | 2,740,252 | 784,388 |

Observed instrumented campaign reductions are approximately 79.7% and 77.9%.
They are **not** a qualified claim that the 35% campaign or 10% complete-job
target has been met. Aggregate CPU must include harness CPU; sample peaks are
simultaneous process-tree RSS during build commands, not kernel largest-child
RSS and not a bound on postprocessing/packaging memory.

| Provider preparation | GNAT 14 seconds / child CPU | GNAT 16 seconds / child CPU |
|---|---:|---:|
| Normal | 18.526 / 45.747 | 22.543 / 59.024 |
| Check | 20.294 / 50.040 | 22.977 / 62.882 |
| Ignore | 18.354 / 47.295 | 24.144 / 64.793 |

Every cell has an individual command profile with CPU, elapsed, sampled RSS,
raw exec/read trace and diagnostics. Summing overlapping compiler lifetimes is
not used as elapsed time. Raw evidence sizes (uncompressed, including traces,
sources and artifacts) were 10,346,721,788 → 2,660,358,856 bytes for GNAT 14 and
11,916,237,997 → 4,360,373,959 bytes for GNAT 16. These are **not upload sizes**.

## Hosted observations and performance acceptance blockers

Exact-current-main Fast CI run
<https://github.com/zackboll/ams-gra-codegen-oms/actions/runs/40626798700>
was still pending during investigation. Its timings are unavailable; historical
20m48s / 14m32s figures were not reused as current baselines.

Latest inspected successful main run:
<https://github.com/zackboll/ams-gra-codegen-oms/actions/runs/40451914377>, source
`d603077c6d38ccc0658391776e3af9eecb009255` (not current main):

| Hosted job | Portable step | Full job | Evidence upload step | Artifact bytes |
|---|---:|---:|---:|---:|
| ada-hexbinary-codec, GNAT 16 | 16m31s | 25m22s | 56s | 325,324,137 |
| ada-float32-codec, GNAT 14 | 11m28s | 39m06s | 68s | 759,376,915 |

Artifacts are whole-job artifacts, not portable-only archives. Upload step
durations include action packaging; separate packaging/network time, hosted
CPU and hosted peak process-tree memory are not available from the Jobs API.
Both complete historical jobs total 64m28s of job wall time; that is not billed
runner-minute accounting and excludes queueing. No optimized hosted job exists.
Whole-job speedup, candidate upload size/time, total workflow runner-minutes,
critical-path change and five-runner scheduling impact are **unmeasured**.
The five shared main lanes remain unchanged.

Required follow-up: complete clean original/candidate campaigns on comparable
hardware with equivalent compiler settings and repeatable methodology, then
exact-final-head Fast/Deep CI and mandatory evidence verification. If measured
targets fail, use `PERFORMANCE_TARGET_NOT_MET`, not these instrumented numbers
as a substitute. This checkpoint remains `QUALIFICATION_FAILED` because
qualification itself is incomplete, independently of performance.

## Validation performed

- Existing PERF-005 tests: **32 passed**; portable regression tests: **4 passed**.
- Task093 compiler placement: **12 passed**.
- CI-SELECT-001 inventory/selector and required-gate tests: **48 passed**.
- CI split guard and mutation suite: **529 checks passed**.
- Task077B layout/mutations: **3 tests passed**, two 364-case ownership runs.
- CI002 trust/security contracts: **30 passed**.
- Rust 1.95 fmt/check/clippy (`-D warnings`) and full workspace tests: passed.
- Pinned actionlint 1.7.12 under existing queue-key policy: passed.
- `git diff --check`: passed.

Initial local test invocations without strace and with incomplete Task077B CLI
arguments failed; these were tooling/invocation errors, corrected and rerun.
Those failed logs are retained, not described as passing runs.
There is no exact-final-head hosted validation yet.

## Evidence and reproduction

The compact committed provenance/accounting record is
`/home/zboll/git/ams-gra-codegen-oms-perf-009-portable-provider/docs/perf-009-portable-provider-reuse-evidence.json`.
It retains provider identities and artifact manifests once per variant, every
positive record, preparation/mapping report hashes, CPU/memory accounting and
the live-control results. Raw logs, traces, all per-cell profiles, immutable
provider artifacts and disposable control sources remain in the external
evidence directory above; the JSON is not a substitute for those raw files or
a completed mandatory uploaded evidence package.

To reproduce, first build the exact-head generator, download/verify the pinned
Alire release, and prepare **two** distinct states using the existing
`perf005-state.py`, with the same index and toolchain seeds. Use names
`<baseline>-state` and `<candidate>-state`; keep both campaign roots outside
`/tmp` for Landlock. Set HOME and PATH separately for each process. Run
`perf009-baseline.py --compiler <14|16> --scratch <baseline> --generator <binary>`
to completion before
`perf009-portable.py --compiler <14|16> --scratch <candidate> --baseline <baseline> --generator <binary>`.
Both interfaces require the full matrix; repeat independently per compiler.
Install strace and retain ample storage. This procedure reproduces the
instrumented experiment, **not** a clean performance benchmark or CI acceptance.

## Rollback

CI rollback is unnecessary: original portable workflow commands never changed.
Stop invoking the explicit experiment adapter and use the authoritative runner.
No production semantics, shared Alire settings, provider cache, selection
policy or runner configuration must be reverted. Remove only task-owned scratch
directories when evidence retention permits; provider read-only permissions
may require owner write permissions to remove them. Never alter other worktrees
or unrelated campaigns to clean up this experiment.

## R1 — source reconciliation and safety hardening

R1 fetched main, the remote feature branch and PR #121. Local/remote/GitHub PR
heads all matched `162d2f04d5110186456db4565caf51024dbab4cb`; main remained
`ac046599c492241aefc9b1d6b742668c0e9cd39a`. No merge was necessary. All R1
evidence is beneath
`/home/zboll/git/ams-gra-codegen-oms-perf-009-evidence/r1`; earlier evidence is
preserved. Only normal new commits are used on the same PR/branch.

### R1 implementation

- Added a portable-only `PortablePilot` around the unchanged PERF-005 provider.
  It retains independent preparation-manifest hashes and rehashes actual GCC,
  GNAT, GPRbuild, `system.ads` and `libgnat.a` before/after dependent builds.
- Provider preparation requires 106 successful actual compiler invocations,
  unique compilation records, complete real traces and exact dependency-lock
  versions/origin hashes. Artifact mutation cannot be hidden by rewriting the
  provider's own manifest.
- New evidence verification matches actual execution records and pre-execution
  mapping bytes against raw traces. ADC, GCC specs and response evidence require
  actual compiler read witnesses. Mutable post-build GPR inputs are not proof.
- Added complete naming/policy-boundary comparison code: all ten independent
  source deployments per policy, 106 provider inputs and 89 Ada mappings per
  boundary. It preserves indirect inputs and checks collision-free runtime
  cache deltas against actual read source bytes. The expensive exhaustive run
  must finish before this can be called qualified; implementation alone is not
  evidence of all 60 boundary checks passing.
- Require exactly one binder-generated compilation, not just five owned
  compilations plus a binder marker. Reject ABI/runtime and optimization
  overrides. Consumer executions are traced and checked against their actual
  executable rather than accepting an arbitrary printed completion marker.
- Evidence seals inventory every file, reject truncation/additions/symlinks and
  retain a separate hash. These are integrity records, not signatures against
  an attacker able to rewrite both records and hashes.
- Both qualification interfaces require clean committed implementation, equal
  explicit worker budgets, and unchanged start/end source and generator
  identities. They fail rather than pairing a rebuilt generator with a
  different historical baseline binary.

### R1 disposable controls

The committed `perf009-controls.py` replays genuine retained provider compiler
inputs into new disposable directories. It never mutates positive evidence.
The third complete attempt passed **108 controls per compiler** (36 per policy):

| Category per compiler | Count | Meaning |
|---|---:|---|
| Actual compiler failures | 12 | Changed provider source, GPR configuration, GCC response option, imported dependency map; all three policies |
| Actual policy compile/link/execution probes | 3 | Assert(False) raises under Check; Ignore does not evaluate it |
| Actual execution rejected by verifier | 6 | False marker process and skipped executable, each policy |
| Verifier-only disposable mutations | 87 | Provider artifacts/manifests/policy, compiler/runtime hashes, compile/backend/bind/link records, read-time input records, false preparation, lock provenance, stale/duplicate objects, evidence seals |

The fourth run additionally exercises ABI and optimization overrides, increasing
the inventory to **114 per compiler** when its completion record is present.
Raw completion/status is recorded separately in the R1 handoff JSON. The suite
continues to label itself non-exhaustive: full portable consumer mapping replay
and complete independent ABI qualification remain mandatory before rollout.

Initial attempts exposed invalid negative controls and were retained:

1. Remapping the explicitly named primary GNAT source did not redirect it;
   GNAT succeeded, so the harness correctly failed the attempted control. The
   corrected map redirects an imported `GNATCOLL` dependency and GNAT rejects
   the planted compile-time error.
2. Mutating a retained but unused temporary file did not change actual inputs.
   The corrected control selects the actual backend `-gnatec` argument.

No diagnostic was fabricated to turn those failures into passing results.
Full-suite results are not inferred from synthetic unit tests.

### R1 performance gate: incomplete

Added a separate **untraced** full-campaign benchmark interface. Original and
reuse paths specify the same worker budget; timed reuse requires a sealed
traced qualification with mandatory negatives complete. The mode cannot accept
tracing as timing evidence, and reports packaging, counts, CPU, elapsed and
sampled tree RSS separately. Its acceptance flag remains false pending an
independent uncontended complete A/B/B/A review. Setup is outside campaign
measurement and must be recorded by the caller; no accepted setup comparison
or complete clean benchmark exists yet.

Two bounded benchmark preflights rejected the available allocation:

- Four selected CPUs plus SMT siblings measured approximately **91–99% busy**.
- A second four-CPU allocation was also contended and rejected.
- Host load was approximately 46–49 on 32 logical CPUs, with unrelated
  documentation, compiler and CI processes. No unrelated process, mining
  workload or runner was stopped, reprioritized or modified.

Therefore no R1 clean campaign timings, 35% acceptance result, complete-job
10% result, CPU-throughput acceptance or A/B/B/A result is claimed. Historical
instrumented numbers above remain attribution only.

### R1 exact-source campaigns and regression blocker

Fresh original qualification attempts used four workers and separate private
states. Early attempts were stopped as obsolete after source-identity
hardening; subsequent attempts are explicitly recorded as incomplete, not as
30-cell successes. The later attempts reached five completed positive cells
per compiler before the independent acceptance blockers were recorded and
task-owned processes stopped. No unrelated process was stopped.

Candidate pairing with the historical baseline failed closed because the
rebuilt exact-head generator SHA-256 was
`e67f5235bd6363f21c941a6487ab49e8652d521bb43227c5d8edad240faf133b`, not the
historical manifest's
`2521e9d969b0235c89706bc48defd12020efc04f7c4116f5cfecdf8539c9936b`.
No binary-hash exception or successful fallback was added. Complete current
30-cell original/candidate pairs and exact-final-head requalification remain
outstanding; prior complete matrices do not substitute for them.

R1 local validation:

- **43** provider tests pass: 32 PERF-005 plus 11 portable tests.
- Selector/final-gate **48**, placement **12**, CI split **529**, Task077B
  layout/mutation controls, CI002 **30**, pinned actionlint and diff check pass.
- Rust 1.95 fmt, workspace check and clippy pass. Full workspace tests **fail**
  the existing `runtime_fairness` assertion at line 193:
  `a runtime worker thread outlived its runtime`.
- A single focused rerun also fails that assertion. The test is not changed,
  ignored, retried to green or explained away as proven contention; the cause
  is not established. Its complete logs are retained in R1 evidence.

The existing Float32 path, authoritative helper runner, workflow commands,
selection rules, five-lane concurrency and CI002 contracts remain unchanged.
No production-qualified portable CI entry point is enabled. This is the
required fail-closed response while safety, clean performance and regression
gates do not all pass.

### R1 hosted and handoff limits

Hosted status is checked at bounded reconciliation points, not a polling loop.
At R1 investigation the current-main affected jobs were still incomplete;
there was no successful complete current-main comparison. No optimized hosted
jobs exist because the integration gate was not met. Full-job elapsed,
aggregate runner-minutes, uploads, critical path and five-runner scheduling
impact remain unqualified. A later green unmodified workflow would not itself
establish portable reuse or its performance target.

Final R1 status and exact commit/tree are reported in PR #121 and its handoff;
documentation-head source identity is obtained from Git, not a self-referential
embedded commit hash. **Disposition remains QUALIFICATION_FAILED.**

## R2 — diagnosed Rust observation race; performance environment blocked

**R2 disposition: PERFORMANCE_ENVIRONMENT_BLOCKED.** This does not imply that
the portable functional or Rust regression gates passed. CI remains unchanged.
R2 stops with reproducible prerequisite evidence rather than restarting long
Ada campaigns or selecting a passing Rust rerun.

### Reconciliation and isolation

Fetched main, the feature branch and PR #121. Local, remote branch and GitHub PR
heads matched `51576c69525891f5aff78de4a35a935f54dc5885`, tree
`f5be301d6fdd009dfdf76c0d08093d4d02f7bf5d`. Main remained
`ac046599c492241aefc9b1d6b742668c0e9cd39a`; no merge was required.
New evidence is exclusively beneath
`/home/zboll/git/ams-gra-codegen-oms-perf-009-evidence/r2`.
Main was extracted with `git archive` into a disposable source directory and
built with a separate Cargo target directory. No existing worktree was used
as a scratch checkout. Earlier results are preserved.

### Unmodified Rust comparison

The runtime, fairness test, test support, Cargo manifest and lock inputs are
identical between main and the starting PR head (`git diff` is empty).
Fairness source SHA-256:
`1b5b39ca32b3a1f59a6039c1ae0fa21b099c48ef4ca7189e762f9e317cd89cc7`.
Compiler: Rust 1.95.0, commit
`59807616e1fa2540724bfbac14d7976d7e4a3860`, LLVM 22.1.2,
`x86_64-unknown-linux-gnu`. Both `--locked --no-run` focused builds succeeded.

| Source | Test binary SHA-256 |
|---|---|
| main | `01e47833a49d48b4ad012ab0e8f02f6fd9a024acc9d165da9be7a335fa92a67f` |
| PR head | `edd3957527c97e644655ac9643dc34bb3e1975cfd9068113ab35128827a95e6d` |

The executable test command was the independently built binary followed by
`--exact task049_receive_progresses_under_sustained_publish_load --nocapture --test-threads=1`.
Single-CPU runs prefix `taskset -c 0`. All stdout/stderr, commands, exits,
source/binary hashes and load snapshots are retained per run, including failures.

| Fixed bounded series | main failures/runs | head failures/runs |
|---|---:|---:|
| Initial focused runs, inherited affinity | 0/3 | 0/3 |
| Syscall-traced runs, inherited affinity | 0/3 | 0/3 |
| Frequency series, inherited 32-CPU affinity | 0/10 | 0/10 |
| Frequency series, single CPU | **7/10** | **5/10** |

These were focused, single-test processes on a loaded shared host, **not**
exclusive isolated hardware. An uncontended reproduction frequency is
unavailable. Host load was approximately 24–29 on 32 logical CPUs; unrelated
compiler/documentation/CI workloads remained untouched. Passing runs are not
substituted for the reproduced failure.

### Executable diagnosis: successful join is not immediate procfs removal

A diagnostic-only LD_PRELOAD observer records `pthread_create`, thread names,
callback returns, real `pthread_join` results and the exact `/proc/self/task`
name bytes read by the **unmodified** fairness test. It delegates calls and
does not alter return values, task names or the assertion. Observer runs are
separately labeled because their timing differs from uninstrumented runs.

In the third observer series, two of five runs failed on each source. Every
failure has this ordering for the **same worker handle/TID**:

1. Worker `ams-gra-oms-lac` callback returns.
2. Test thread's real `pthread_join(worker)` returns **0**.
3. The original `live_workers()` reads that worker's procfs `comm` successfully.
4. The original line-193 assertion fails with exit **101**.

For example, head worker TID **3358668**, pthread handle
**140080581990080**, was joined by test TID **3358665** before its name was
counted. Main worker TID **3358230** shows the same sequence. Complete logs
and executable/binary identities are included in the R2 result manifest.
Additional observed main failures captured `/proc/.../stat` flags
**0x40004c**, including **PF_EXITING (0x4)**: the visible task is already in
kernel exit, not evidence of an application worker continuing to serve.

A standalone C/POSIX probe, with **no Rust or project runtime**, creates and
joins 1,000 threads on CPU 0. It requires the callback-completion flag and
successful join before reading the joined TID's procfs record. It completed
1,000 successful joins and found **501 still-visible procfs task records**.
This independently reproduces the assumption violated by the test.

Source references retained with hashes support the observed ordering:
glibc 2.41 `pthread_join_common.c` waits for the child's TID word to clear;
Linux 6.12 `mm_release()` clears it and wakes the futex before later
`do_exit()` cleanup and `exit_notify()`. Upstream reference files explain the
mechanism; they are not claimed to be exact Debian kernel build provenance.
The running host was Debian kernel 6.12.113 and glibc 2.41-12+deb13u4.

One GDB observation also captured the worker before `close()`, its exit, and
successful return from `close()` with no worker visible afterwards. That
passing debugger run is corroboration only, not a substitute for the failing
observer traces. The first preload attempt accidentally opened its log only
in the `taskset` wrapper; empty-log attempts were retained and excluded from
diagnostic conclusions. Corrected observers append across exec.

**Supported conclusion:** the reproduced assertion failure is an inherited
join/procfs observation race, not an established runtime lifecycle defect.
There is no supported Rust production correction to import into PERF-009.
The minimum separately reviewed follow-up is lifecycle instrumentation tied
to the exact worker's successful join, with any procfs-disappearance check
treated as asynchronous kernel cleanup and bounded separately. This PR does
**not** implement that follow-up, relax the assertion, add sleeps/retries to
the test, or mark its failing executions successful. The current mandatory
Rust gate therefore remains unmet despite the diagnosis.

### R2 performance and stop decision

Ran the existing untraced benchmark interface once with `--preflight-only`,
four workers and CPUs 0–3. Physical siblings 16–19 were also measured.
Utilization ranged from **40% to 100%**, failing the existing <5% busy
precondition. The interface exited nonzero before starting a campaign.
Recorded status is **PERFORMANCE_ENVIRONMENT_BLOCKED**. No expensive rejected
benchmark, synthetic load generator, process reprioritization, runner change
or unrelated workload interruption followed. No A/B/B/A or clean performance
acceptance is claimed.

Because the environment prerequisite remains unavailable and the unchanged
mandatory Rust assertion still fails, R2 did not rebuild another Ada generator,
prepare more Alire states, or restart the long original/candidate matrices.
There are **zero new complete R2 portable campaigns or 60-boundary proofs**.
Historical 30-cell/150-owned-compilation results and R1's 114 controls/compiler
remain historical only, not current-head substitutes. Exact generator/source,
mapping, ABI and artifact gates remain mandatory for resumption.

R2 reran the **43 PERF-005/PERF-009 tests**, all passing. Documentation/CI
policy, placement, actionlint and diff checks are recorded separately. Full
workspace acceptance is not claimed from focused passing runs.
No production, Rust test/runtime, workflow, PERF-005, selector, security or
runner configuration changed. The original two portable commands stay active.

At bounded hosted reconciliation, the current-main Float32 job was successful
but hexBinary was still pending. No complete current-main performance baseline,
optimized hosted job, final-head mandatory artifact verification, full-job
speedup, runner-minute reduction or critical-path result exists.

Compact R2 results are in
`/home/zboll/git/ams-gra-codegen-oms-perf-009-portable-provider/docs/perf-009-r2-results.json`.
The external R2 directory retains standalone probe/observer source, compiled
binaries, all run transcripts, source references and hashes. Reproduce the
unmodified failing command with CPU-0 affinity; the fixed series records
intermittency rather than promising every invocation fails. Final source
commit/tree are the normal PR handoff head, not embedded self-referentially.