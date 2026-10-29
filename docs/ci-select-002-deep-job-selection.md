# CI-SELECT-002: job-level Deep qualification

## Verified baseline

Incorporated main: `7958c4e0d8f78768fd4c7d4c1bd34877c31d893a`, tree
`a94fc3bb844349780772566eafd50db7a7328efe` (fetched before creating the
isolated feature worktree). Existing worktrees and active PRs are not modified.

At this baseline `deep: true` invokes `deep-qualification`, which executes
**eleven static Deep job IDs, twelve expanded runner cells**. The task's
description of ten families is not the current source inventory. All twelve
cells remain registered; none are added or removed by this change.

The inventory remains 40 expanded runner cells, 39 main-eligible cells, one
PR-only trusted signal and five shared main lanes. Runner placement, matrix
names, pinned containers, timeout policies, inherited commands, artifacts,
Pages deployment, trusted signal/disabled dispatcher and security sources are
unchanged. Every main-eligible job retains its existing `queue: max` lane.
PR concurrency identities remain independent.

## Canonical manifest v2

The existing selector remains the sole classifier. Existing ordinary fields
are retained. Additional fields are:

* `deep_jobs`: sorted, duplicate-free array of exact registered Deep job IDs;
* `deep_selected_cells` and `deep_skipped_cells`: expanded compiler-cell counts
  whose sum is twelve;
* `deep`: exactly `bool(deep_jobs)`;
* each path reason includes the Deep IDs it requires, including transitive
  prerequisites. Full decisions assign all Deep jobs to every changed path.

`version` is now 2. A selected matrix job always selects both GNAT14 and GNAT16;
there is no compiler-subset interface. `rules[].deep_jobs` and
`rules[].deep_rationale` in the existing qualification inventory are the reviewed
ownership interface for future work. Deep record `prerequisites` closes
transitively; the three PERF-007 shards form one evidence group so the original
seven tasks still reconcile as their complete disjoint union.

Malformed or incomplete classification falls back to full ordinary plus full
Deep qualification. The independent final gate still rejects missing or
malformed classifier output. Source verification is not bypassed by fallback:
base/head/merge commit identities, exact merge parents, complete nonshallow
trees, and the independent no-renames Git diff path inventory remain mandatory.
Deleted, renamed, copied, type/mode-changed and ambiguous paths are broad.

## Reviewed ownership

| Rule | Deep jobs | Source/test dependency reasoning |
| --- | --- | --- |
| Documentation | None | Only the nonexecutable documentation allowlist; compiled documentation retains its ordinary Rust test. |
| Python runtime, CLI Python tests, Python fixtures | `real-sleet`, `real-uci-regressions` | Sleet harness executes PY005/006/007 network integration on the Python runtime and fixtures; regressions executes the pinned PY007 current-service and inheritance/profile qualification. |
| Python backend | Above plus `msrv-real-uci` | Python generation dispatch is confined to `PythonBackend` in CLI. MSRV also compiles CLI/backend-python transitively via facade `build.rs`; runtime/fixture-only Python changes do not affect that generated Rust/toolchain contract. |
| Optional Ada WebSocket sources/projects and dedicated fixtures | `ada-websocket-real-sleet` (both compilers) | Task100 compiles the optional transport and generated local/pinned-Sleet consumers. `real-sleet` uses Rust/Python clients, not Ada transport sources. Unrelated UCI Ada drivers do not import this runtime. |
| Ada LA-CAL runtime and dedicated runtime fixtures | `ada-websocket-real-sleet` (both compilers) | Task100 imports shared connections, transports, envelopes, typed delivery and JSON composition. Ordinary Task099/100 jobs remain selected. Shared OMS JSON/codegen changes retain broader qualification. |
| Rust runtime API/adapter | `msrv-real-uci`, `real-sleet`, all three PERF-007 shards | In addition to MSRV/Sleet, Task062 IPv6 and Task066 Unicode vertical drivers generate consumers with direct path dependencies on both Rust runtime crates. The entire PERF-007 evidence group closes transitively. |
| Rust facade-test harness | `msrv-real-uci`, `real-sleet` | Both compile the facade harness and generated pinned UCI codecs. PERF-007 vertical consumers import runtime crates directly, not the facade test crate. |
| Rust generator | All Deep jobs | Generator consumers span multiple pinned drivers; retain broad qualification rather than claim an unproven narrower dependency graph. |
| Ada JSON/backend/support, temporal fixtures | All Deep jobs | Existing shared/composition contracts retained conservatively. |
| Shared IR/frontend/codegen/CLI, Cargo/toolchain, infrastructure, unknown paths | Full ordinary and all Deep jobs | Shared inputs or unresolved ownership cannot authorize a narrow run. |

`scripts/**` remains a full fallback, including component-owned qualification
scripts. Script ownership refinement and stable component runners are out of
scope. No filename-substring heuristic chooses a Deep subset; existing unknown
Task099/100 fail-broad guards remain additional protections.

## Workflow and authoritative final gate

The reusable workflow accepts a string `selection` passed directly from the
verified classifier output through `with`. Every registered Deep job has the
same reviewed job-level selection expression with its own static ID, evaluated
before runner allocation and before matrix expansion. Non-PR events always
run all jobs. Empty/old-version input is conservative, never a no-test success.
Direct nightly/manual qualification remains full. If no Deep job is needed,
the caller is skipped entirely.

The mandatory baseline pins the caller/input and Deep header, ownership rules,
baseline cell inventory, original step records/payloads and runner headers.
It permits only the reviewed Deep `if:` addition to original runner headers.
It rejects missing/new jobs, altered commands, failure masking, unsafe
cancellation, lane/matrix/compiler/permission changes and source-control drift.
The inherited CI002/trusted-dispatch hashes are unchanged.

`pr-ci-required` independently recomputes the complete proposed-merge decision.
Selected Deep cells require unique authoritative current-attempt job records,
successful completion, every mandatory step, and all nonexpired/nonempty,
nonduplicate, nonstale evidence. A green reusable aggregate cannot substitute
for executed mandatory steps. PERF-007 and Task100 source/tree/run/attempt
reconciliation applies precisely when those jobs are selected.

Unselected Deep jobs must have no qualification steps or runner allocation and
no owned artifacts. The gate expects one collapsed skipped job when the caller
runs (job-level `if` precedes matrix expansion), and permits no child records
when the entire caller is skipped. Unexpected execution, extra compiler cells,
unknown job names or artifacts fail. The actual GitHub representation must be
confirmed with retained hosted jobs before acceptance is claimed.

## Validation and evidence

Retain all inherited tests. Added tests exercise narrow ownership, canonical
IDs/order/types/counts, matrix indivisibility, transitive closure, full events,
malformed classification, selected missing/skipped/failed/cancelled jobs,
unexecuted steps, expired/stale/duplicate evidence, unexpected execution or
artifacts, collapsed matrix skips and immutable input/condition mutations.
Negative controls assert their intended diagnostic rather than relying on an
unrelated earlier failure. Existing source identity, Task100/PERF-007 evidence,
sixth-lane, cancellation, compiler and security controls remain mandatory.

Historical reference: [PY-008 run 40794646389](https://github.com/zackboll/ams-gra-codegen-oms/actions/runs/40794646389),
attempt 1, successful, head `af467b4789758cab544829a15bc0ae5d2d96cbba`.
API job-step timestamps give **603.55 total runner-minutes**, **260.40 Deep
runner-minutes**. Run creation to update spans **61 minutes 8 seconds**. This
historical run has ten Deep cells; current main has twelve after Task100.
These are historical measurements, not achieved savings or an apples-to-apples
fixed-workload benchmark.

Delivery requires exact-head full CI followed by disposable Python, WebSocket,
documentation, infrastructure and selected-failure PRs against this feature
branch. Positive narrow fixtures must not change workflow YAML. Retain jobs,
artifacts, identities, selected/skipped counts, elapsed/runner-minute reports,
and required-check results; close disposable PRs without merging. Until those
complete, do not claim `READY_FOR_INDEPENDENT_REVIEW` or measured selectivity
savings. Auto-merge remains disabled. No approvals, merges, deployments,
permission changes, force pushes, rebases or amended commits are authorized.