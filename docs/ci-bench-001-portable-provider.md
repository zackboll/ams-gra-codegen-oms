# CI-BENCH-001 — manual portable-provider benchmark infrastructure

## Scope and authorization

Infrastructure only, based on main `7958c4e0d8f78768fd4c7d4c1bd34877c31d893a`
(tree `a94fc3bb844349780772566eafd50db7a7328efe`). This does **not** modify
PERF-009's branch, correctness certificate, acceptance flags or production
portable qualification. The infrastructure PR must remain unmerged, without
auto-merge, for independent review. No benchmark is dispatched by this PR.

After independent review and explicit authorization, the trusted definition
and controller must be merged to **main** before use. Dispatching a feature
version of this workflow is not supported. Only maintainers with repository
Actions write access can manually dispatch. The `source_sha` input must be the
reviewed, exact, lowercase 40-character repository commit; branch names, tags,
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
is no feature Dockerfile, feature Python import, candidate host command,
credential-bearing mount, host socket, SSH agent or Actions environment in the
worker. Generator build scripts, generated binaries and project files still
execute **untrusted code**, exclusively inside the sandbox.

Each fresh worker container has:

- GitHub-hosted ephemeral `ubuntu-24.04` host; no persistent cache or runner;
- network disabled; all capabilities dropped; no-new-privileges; non-root UID;
- read-only image and candidate source; only bounded private tmpfs writable;
- one fixed guest CPU, one build worker, 6 GiB RAM with no swap expansion,
  256 processes, 4 GiB work tmpfs, 256 MiB temporary tmpfs;
- 1,800-second iteration limit, bounded 16 MiB retained output, 180-minute job
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

Both modes use the same trusted Task093 runner and already-main PERF-005
campaign-local provider mechanism. This is a **benchmark adapter**, not the
activation or recertification of PERF-009's hardened production candidate.
It never imports feature Python or changes its certificate. Any acceptance of
timings as evidence for another implementation requires an independent
equivalence/correctness decision at the exact source under review.

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

Before execution, the controller requires a complete guest SMT sibling set and
at most **5% external busy time per core-equivalent** and **1% steal time** over
a ten-second idle sample. Throughout each worker, host `/proc/stat` is sampled
each second; host cgroup CPU usage is subtracted to estimate work unrelated to
the benchmark on the selected CPU and siblings. Missing samples, gaps over
three seconds, or threshold violations invalidate the campaign. Samples are
persisted during execution, including on failure.

GitHub-hosted guests do **not** expose reliable physical-host topology or every
form of hypervisor co-tenancy. Guest-visible SMT and steal measurements cannot
prove exclusive physical-core allocation. A passing `valid` gate means the
documented guest-observable measurement contract passed, **not physical-host
isolation, statistical significance, PERF-009 completion or performance success**.
The report always sets `performance_success: false`; independent acceptance is
required. Never override a failed identity/contention gate to obtain timings.

## Evidence and upload accounting

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
  -f source_sha=EXACT_REVIEWED_40_CHARACTER_LOWERCASE_COMMIT_SHA
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
- <https://docs.github.com/en/actions/how-tos/write-workflows/choose-when-workflows-run/control-workflow-concurrency>
- <https://docs.github.com/en/actions/reference/workflows-and-actions/events-that-trigger-workflows#workflow_dispatch>