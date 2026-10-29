# CI-BENCH-001 — manual portable-provider benchmark infrastructure

## Scope and authorization

Infrastructure only, reconciled in R2 with main
`f025748445db2954186bea39079bd34afb63080f` through ordinary merge
`e8fdb432478009d80dbc6cf60f59bfa18b54ff28`. This preserves PR #125's
producer-attempt verification, Python Float64, Task100 and the current
CI-SELECT-001 inventory. This does **not** modify
PERF-009's branch, correctness certificate, acceptance flags or production
portable qualification. The infrastructure PR must remain unmerged, without
auto-merge, for independent review. No benchmark is dispatched by this PR.

After independent review and explicit authorization, the trusted definition
and controller must be merged to **main** before use. Dispatching a feature
version of this workflow is not supported. Only maintainers with repository
Actions write access can manually dispatch. The `source_sha` input must be the
reviewed, exact, lowercase 40-character repository commit on the controller's
explicit allowlist (currently PERF-009 `06f33bdda655b37f578a81bbca99ef7567407926`);
branch names, tags,
abbreviations, shell expressions and fork repository overrides are rejected.

## Independent policy audit

- CI-SELECT-001's `ci_policy.py` inventories every workflow and job, pins
  inherited payloads/security sources, and rejects unmapped workflows. Existing
  `ci.yml`, `deep-ci.yml`, Pages, CI002, signal and trusted-dispatch workflows
  are unchanged. Main continues full qualification; PR source rules are unchanged.
- All main-eligible runner cells share exactly five `oms-main-slot-1` through
  `oms-main-slot-5` groups. The new manual job takes **slot 3**, not a new lane.
  `queue: max` preserves pending jobs; there is no cancellation expression.
  GitHub documents a maximum of 100 queued jobs per group, after which additional
  arrivals are canceled by the service. Operators must not flood dispatches.
  A single approved campaign is the operating contract, not a bulk queue.
- Existing `workflow_dispatch` on ordinary/Deep CI is qualification, not an
  arbitrary benchmark interface. The new registration is an explicit
  `manual_benchmark` inventory section, outside automatic selection and the
  mandatory aggregate. The existing **40 automatic cells/five lanes** remain
  unchanged; there is **one additional manual runner cell**, never selected by
  push, PR, nightly schedule, workflow call or workflow-run events.
- The exact normalized workflow digest is independently pinned in
  `ci_bench/policy.py` and the inventory. Controller/image/worker/provisioning
  hashes are mandatory. A new unrelated workflow still fails closed.
- Existing CI002 trust tests, sources, permission boundaries, required-gate
  behavior and historical dispatcher provenance are unchanged.

## Trust boundary and resource limits

The workflow has only `contents: read`, no secrets/environment, OIDC, write
permissions, app token, self-hosted label or destination dispatch. Actions are
SHA-pinned. Checkout uses the dispatch's exact main SHA and explicitly disables
credential persistence. The controller independently verifies current main,
public repository commit metadata, fetched commit/tree and archive identity.
If main moves between dispatch and authorization, dispatch must be repeated.
Inputs enter Python via environment variables, never inline shell expressions.

Candidate sources are fetched as data, not checked out over trusted controller
files. Symlinks/submodules and source archives over 128 MiB are rejected.
Candidate `Cargo.lock` must match trusted main, so arbitrary feature dependency
updates require separate main authorization. The image is built from trusted
main only; `.dockerignore` excludes Git metadata, outputs and bytecode. There
is no feature Dockerfile, feature Python import **on the host**, candidate host command,
credential-bearing mount, host socket, SSH agent or Actions environment in the
worker. Candidate Python, generator build scripts, generated binaries and project files still
execute **untrusted code**, exclusively inside the sandbox.

Each fresh worker container has:

- GitHub-hosted ephemeral `ubuntu-24.04` host; no persistent cache or runner;
- network disabled; all capabilities dropped; no-new-privileges; non-root UID;
- read-only image and candidate source; only bounded private tmpfs writable;
- two fixed guest logical CPUs, two Cargo/GPRbuild workers, 6 GiB RAM with no swap expansion,
  256 processes, 4 GiB work tmpfs, 256 MiB temporary tmpfs;
- 2,250-second iteration limit, bounded 16 MiB retained output, 360-minute job
  limit; no candidate output is interpreted as a runner workflow command;
- fresh generator build, private Alire settings/cache and build directories.

Docker is used by the trusted host controller as the isolation primitive. The
candidate never receives Docker access, privileged devices, host PID namespace,
host filesystem or additional Linux capabilities. A container is not a VM and
does not defend against a host-kernel vulnerability. Only approved public
repository commits should be benchmarked. Read-only permissions do not make
untrusted source safe to run directly on the Actions host.

## Campaign and measurement contract

One dispatch runs eight sequential full 30-cell portable campaigns:

| Compiler | Order |
|---|---|
| GNAT 14.2.0 | original / reuse / reuse / original |
| GNAT 16.1.0 | original / reuse / reuse / original |

Both modes use the trusted Task093 runner, independently compared with the
reviewed PERF-009 commit with no delta. Reuse imports the exact candidate's
`PortablePilot(..., traced=False)` **inside the sandbox**. Its entire `build`
path is timed: inherited preparation, lock/source checks, live compiler/runtime
verification, all-three-provider manifest checks before each consumer build,
and post-build integrity verification. Only source commit
`06f33bdda655b37f578a81bbca99ef7567407926`, tree
`4b2612df3cfcf5c5bfc0498b8ea75f8d3e453fce`, is accepted. Scope changes require
reviewed controller/inventory changes, not dispatch overrides.

This measures the PERF-009 **untraced provider implementation**, not its traced
qualification program or correctness certificate. `perf009-portable.py` adds
syscall/read-mapping instrumentation, equivalence and certification processing;
those are intentionally not timed. `perf009-benchmark.py` wraps the same
`PortablePilot(traced=False)` but has local Git/state/certificate and preflight
entry checks. This adapter replaces those entry checks with trusted host
authorization, exact source/tree validation, full-cell evidence validation and
sustained host accounting. It does not run that CLI or establish its sealed
negative-qualification prerequisite. Candidate `inventory` checks run for each
consumer; traced `validate_build`/mapping proofs remain separate correctness
evidence. PERF-009's documents explicitly say its prior certificate is not
final-tree qualification after Task100 integration. That limitation is unchanged.

The old PERF-005 `Pilot` adapter omitted real hardening work. Semantic output
equivalence is not timing equivalence. Its historical timings are **inapplicable
to PERF-009 acceptance**. Unknown source/tree or PERF-005 scope fails closed
with `BENCHMARK_SCOPE_INSUFFICIENT`, not relabeling. PR #121 and all of its
qualification/certification files remain unchanged.

The pinned Rust 1.95.0 image digest, Alire 2.1.1 archive hash, frozen Alire index
commit `842d7165e3e1e644f4d03563e7a4f35c05415af4`, GNAT/GPRbuild/runtime hashes,
GNATCOLL 26.0.0 SHA-512 origin, source/tree/archive, built generator and dependency
identity hashes are retained. Apt package versions and image content are
resolved once per dispatch, then the **same immutable image ID** is used for
all eight workers. An upstream package drift that violates compiler/runtime
hash checks fails the campaign instead of silently establishing a new baseline.

The GNATCOLL archive is downloaded and hash-verified once during trusted image
setup. Only its index URL is relocated to an in-container loopback HTTP server;
archive hashes are preserved and no external network is enabled. Both modes
receive identical index/toolchain seed bytes and no compiled provider seed.
This relocation is recorded in the seed manifest, not represented as an
unmodified public-index checkout. Worker copies are independent.

Required counts per full iteration: original/reuse provider compilations
**3,180 / 318**, generated/support **150**, bind/link/execution **30 each**.
Counts come from trusted-runner build logs and exact output checks, not tracing.
No strace, compiler interception or mapping collection occurs in timed runs.
As with any untrusted executable, emitted logs are not cryptographic attestations.

The host records wall time, cgroup cumulative CPU and cgroup peak memory for
each **whole worker**, including private seed copy and generator build. The
worker separately reports helper-only wall/child CPU and largest-child RSS
(not simultaneous process-tree peak). These scopes must not be mixed.
Generator hashes and seed identity must agree across all eight runs; dependency
identities must agree within each compiler. Each campaign uses the same CPU
and worker budget and fresh independent build directories, including a fresh
reuse provider for every B iteration.

### Contention is an acceptance gate, not a cosmetic annotation

Before execution, the controller requires reciprocal guest package/core/SMT
topology for both selected CPUs and every sibling, including those outside the
workload cpuset. It prefers two guest cores; two SMT siblings are labeled **one
guest core**, never two physical cores. Missing topology or unavailable sibling
accounting fails closed. Actual cgroup quota must equal two CPUs, effective
cpuset must match the two selected CPUs, Cargo/GPRbuild must use two workers,
and worker affinity must expose exactly two CPUs. It requires
at most **5% external busy time per core-equivalent** and **1% steal time** over
a ten-second idle sample. Throughout each worker, host `/proc/stat` is sampled
each second; host cgroup CPU usage is subtracted to estimate work unrelated to
the benchmark on the selected CPU and siblings. Missing samples, gaps over
three seconds, non-monotonic counters, OOM, timeout or threshold violations
invalidate the campaign. Cgroup throttled time above **1% of an interval** is
rejected (a new explicit gate, not relaxed contention thresholds). Counters are
read outside the worker; it waits for a fixed host acknowledgement so its cgroup
cannot disappear before the final snapshot. Samples are
persisted during execution, including on failure.

GitHub-hosted guests do **not** expose reliable physical-host topology or every
form of hypervisor co-tenancy. Guest-visible SMT and steal measurements cannot
prove exclusive physical-core allocation. A passing `valid` gate means the
documented guest-observable measurement contract passed, **not physical-host
isolation, statistical significance, PERF-009 completion or performance success**.
The report always sets `performance_success: false`; independent acceptance is
required. Never override a failed identity/contention gate to obtain timings.

## R2 execution-budget analysis and feasibility limitation

**Current disposition: PERFORMANCE_BUDGET_BLOCKED.** The implementation is
bounded, but complete-workload hosted feasibility is not yet qualified.

The original configuration cannot defensibly promise completion: eight
1,800-second iteration maxima already total **240 minutes**, before image,
generator, setup, packaging or uploads, against a 180-minute job. Historical
instrumented original campaigns used approximately **2,026 / 2,641 child CPU
seconds** (GNAT 14/16). At one vCPU, those child CPU quantities alone exceed
1,800 wall seconds. Tracing changes work and these are not clean lower bounds
for untraced runs, but they invalidate any assertion of demonstrated margin.
Even granting reuse no regression but **no speedup**, four campaigns per
compiler would consume `4 × (2026 + 2641) = 18,668` child CPU seconds: **311.1
minutes at one CPU**, excluding harness/setup. It is unproven that the old
configuration can finish; neither its iteration nor overall budget is qualified.

For two logical CPUs the idealized child-only number becomes **155.6 minutes**.
That is not a prediction: serial work, SMT sharing, virtualization, filesystem
and Python integrity work prevent an assumed 2× gain. The historical traced
harness self-CPU was approximately **564 / 630 seconds** per original campaign,
also not clean performance evidence. A deliberately pessimistic planning
scenario gives **no parallel speedup**, retains that traced self-CPU, applies
**25% operational variance** and reserves **120 seconds per iteration** for
generator/setup:

`4 × [(2026 + 564) × 1.25 + 120] +
 4 × [(2641 + 630) × 1.25 + 120] = 30,265 seconds = 504.4 minutes`.

Thus even the revised envelope is not a conservative *completion proof*. With
ideal two-worker child scaling but serial historical harness CPU, that same
scenario is approximately **310.0 minutes** for eight iterations. This explains both why
180 minutes is unsafe to retain without data and why a finite 360-minute
envelope is proposed. It does **not** qualify the scaling assumption, source
overhead or 25% margin. No observed provider-reuse improvement is credited to
this model. Historical and smoke timings cannot prove the PERF-009 target.

The independently policy-pinned resource/time envelope is:

| Component | Maximum/planning reserve |
|---|---:|
| Eight sequential whole-worker iterations, including generator/private setup | 8 × 2,250 s = 300 min |
| Trusted image build, dependency/compiler provisioning | 1,800 s = 30 min |
| Authorization, source fetch, extraction, preflight | 300 s = 5 min |
| Bounded artifact packaging, both uploads, receipt | 600 s = 10 min |
| Operational margin, cleanup/inter-step overhead | 900 s = 15 min |
| Overall Actions hard limit | **360 min** |

`contract.budget()` tests that these sum exactly. The 37.5-minute per-iteration
limit exceeds the worst historical child-only ideal of 22.1 minutes, but the
worst modeled serial-harness/25%-variance/setup iteration is **42.6 minutes**.
Therefore **the per-iteration budget remains unqualified**; the limit is a
fail-closed bound, not permission to call incomplete work complete. A full
authorized feasibility campaign must show all eight independently complete
iterations below 2,250 seconds, with enough margin, or this PR needs another
reviewed budget correction. No sixth lane or automatic job is added.

Runner cost is at most **360 Linux runner-minutes per attempted job** (not
720: two vCPUs are within the existing runner). Dollar/billed-minute cost
depends on repository plan and public-runner eligibility; record actual job
duration/queue delay from Jobs API, never fabricate a price. Image setup time,
worker generator/setup and helper time are separate measurements. Packaging
records elapsed/bytes/hashes; uploads record envelope/bytes/ID/digest. No hosted
upload measurements exist before authorized dispatch.

The public `ubuntu-24.04` standard runner is documented as 4 vCPU/16 GB RAM/
14 GB SSD; labels alone are not a resource guarantee. The controller demands
at least **8 GiB available memory** and **10 GiB free disk** after image setup,
then verifies 6 GiB cgroup memory, zero swap expansion, 256 processes and exact
CPU controls. Work tmpfs is 4 GiB plus 256 MiB `/tmp`, both charged within the
6 GiB memory cap, leaving at most 1.75 GiB for resident non-tmpfs work when full.
OOM/process exhaustion is failure, not a missing-data pass. If a hosted image
cannot supply headroom or complete reciprocal guest topology, fail explicitly.
Physical-host exclusivity is never inferred from guest topology.

### Local R2 evidence

Fresh evidence is outside the repository at
`/home/zboll/git/ams-gra-codegen-oms-ci-bench-001-evidence/r2`; original evidence
was not overwritten. Trusted image builds succeeded. Four restricted local
functional smokes passed (GNAT 14/16 × original/reuse): original counts were
106 provider + 5 generated/support; reuse prepared all 318 provider units + 5
generated/support; each bound, linked and executed once with exactly seven
checks. Every smoke emits `completed: false`, and full acceptance rejects it.

These used two inherited `taskset` CPUs, two-worker quota and offline non-root
read-only sandboxes. Local rootless Podman lacks delegated `cpuset`; the exact
cpuset probe failed and is preserved in `cpuset-rejected.log`. No root access
is available. The guest preflight was contended (over 200% busy per monitored
core-equivalent with SMT accounting, above 5%; steal zero). These are **not
hosted Docker allocation qualification or clean timings**. Four one-cell
smokes do not prove eight full 30-cell campaigns. Full feasibility was not run
in this unqualified environment; unrelated processes were not stopped.

Security negatives cover unauthorized events, workflow/registration changes,
runner lanes, candidate host execution/Docker argument injection, credentials,
source scope, quota/cpuset/worker mismatch, reciprocal SMT topology, headroom,
missing/gapped/non-monotonic counters, contention/steal/throttling, timeout/OOM,
ABBA order, missing/duplicate cells and build/source/bind/link/execute evidence,
traced-as-untraced results, source/generator/dependency drift and upload
failure/stale producer receipts. Inherited CI-RETRY artifact stale/duplicate/
missing/invalid controls and required-gate tests remain mandatory and unchanged.

Local regression covers selector/benchmark/retry/required-gate tests, PERF-007
and Task100 evidence, full split/mutation controls, CI002, documentation/policy,
pinned actionlint 1.7.12 and Rust 1.95 workspace checks. Exact-final-head hosted
Fast CI, Deep CI, CI002, Trusted PR Signal, mandatory selection/results and
`pr-ci-required` must still be reconciled externally. Pending is not passing.
No manual benchmark dispatch is authorized by these local results.

## Artifact contract

Two mandatory artifacts (30-day retention):

1. `ci-bench-001-measurements`: source/image identities, result/progress and
   contention JSON, bounded case-preserving compressed worker logs, packaging
   elapsed time, archive bytes and SHA-256 inventory. Failure emits invalid
   machine-readable results rather than a success marker.
2. `ci-bench-001-upload-receipt`: upload outcome, artifact ID/digest, input byte
   count and monotonic upload-step envelope (includes inter-step scheduling).

Both uploads use `always()` and `if-no-files-found: error`; the final gate
requires complete valid eight-run evidence and successful measurements upload.
The receipt upload's own completion, exact upload durations, runner minutes,
queue delay and whole-job wall time must be read **after completion** from the
Actions Jobs/Artifacts APIs. A final upload cannot measure its own completed
duration from within itself. Do not label the envelope as network transfer time.
No helper or whole-job performance threshold is automatically accepted here.

## One approved campaign (after main integration and authorization)

Review the exact source SHA and dependency compatibility, verify no benchmark
is already pending, and record authorization externally. Do not use this
command before the infrastructure is independently reviewed and merged:

```bash
gh workflow run portable-provider-benchmark.yml \
  --repo zackboll/ams-gra-codegen-oms --ref main \
  -f source_sha=06f33bdda655b37f578a81bbca99ef7567407926
gh run list --repo zackboll/ams-gra-codegen-oms \
  --workflow portable-provider-benchmark.yml --event workflow_dispatch --limit 5
gh run view RUN_ID --repo zackboll/ams-gra-codegen-oms
gh run download RUN_ID --repo zackboll/ams-gra-codegen-oms \
  --dir /tmp/ci-bench-001-approved-campaign
gh api repos/zackboll/ams-gra-codegen-oms/actions/runs/RUN_ID/jobs?per_page=100
gh api repos/zackboll/ams-gra-codegen-oms/actions/runs/RUN_ID/artifacts?per_page=100
```

Check both artifact digests, exact controller/source identities, all eight
ordered runs and every gate. Preserve failed evidence. Do not retry until an
identity/configuration error is understood; for contention wait for another
approved execution opportunity rather than loosening limits. The standalone
worker's `--smoke` mode is local infrastructure validation only: it emits
`completed: false`, cannot satisfy the controller, and is not a benchmark.

## Validation

Local validation uses the same pinned actionlint wrapper, CI policy, selector
positive/adversarial suite, split mutations, and unchanged CI002 tests as
mandatory CI. `test_ci_select_bench.py` is automatically discovered by the
existing mandatory `test_ci_select*.py` command, without editing or repinning
the inherited baseline job. Its negatives cover automatic events, altered main
trust, credentials, permissions, runner, lane/cancellation, timeouts, missing
uploads, missing registration, unknown workflows, malformed SHA, resource
isolation, partial ABBA, counts, compiler/generator/dependency mismatches,
traced runs, worker-budget changes, missing CPU/memory, steal/contention and
sampling gaps. Exact-head hosted validation and local sandbox smoke results are
recorded externally with the final commit/tree; a pending check is not a pass.

References:
- <https://docs.github.com/en/actions/reference/runners/github-hosted-runners>
- <https://docs.github.com/en/actions/how-tos/write-workflows/choose-when-workflows-run/control-workflow-concurrency>
- <https://docs.github.com/en/actions/reference/workflows-and-actions/events-that-trigger-workflows#workflow_dispatch>