# CI-SELECT-001 — change-aware PR qualification and five main lanes

## Stacked PERF-007 graph amendment

The stacked PERF-007 change replaces only the seven-task serial `real-uci`
record with mandatory `real-uci-temporal-naming` (061/071/065, lane 4),
`real-uci-schema-ipv6` (069/062, lane 1) and `real-uci-integral-unicode`
(063/066, lane 3). Deep therefore has ten jobs and the full runner inventory
36 cells; the five simultaneous main lanes remain unchanged. Real-Sleet moves
from lane 2 to lane 4 using observed Fast/Deep execution loads, with no payload
change. The final gate checks each actual job and independently reconciles
source-bound structured artifact contents and the original disjoint task union.
Check-name migration, full original marker inventory, baseline evidence, lane
model and pending hosted acceptance are documented in
`docs/perf-007-deep-real-uci-parallelism.md`. The historical observations below
describe CI-SELECT-001 before that stacked graph amendment, not measured PERF-007
performance. PERF-007 must not merge before this parent is qualified/integrated.

## Disposition and source reconciliation

**HOSTED_CI_PENDING**. This is not merge authorization. Independent review,
hosted integration categories and actual merged-main acceptance remain required.

Non-draft implementation PR: <https://github.com/zackboll/ams-gra-codegen-oms/pull/116>.
Auto-merge was verified disabled. Initial implementation commit `ac3470a1109101ea65ccbe7cecb067e064aff794`
(tree `ea5b4e41a2b66ab8d76b320c1780985cf999d338`) produced synthetic merge
`f2c026ff0aadc19d6654c751c573fe5d7a44c3f4`. Its real complete 17-file diff
selected all **16 ordinary cells and eight Deep jobs**, with no ordinary cells
excluded. Hosted classification succeeded in run `40439985470`; the expected
reusable check-name prefix was confirmed in actual queued job records. This is
classification/job-graph evidence, **not** successful qualification. A follow-up
commit adds the independent diff-completeness cross-check and final evidence.

Starting and incorporated main commit:
`77066e8e2cf4875274c0227cee16dcf0dc593c05`.
Starting tree: `e088a7c68797c8da65ea2d1a92f85a963c22b949`.
Fetched from origin before implementation; the GitHub commits API agreed.
Implementation is isolated in
`/home/zboll/git/ams-gra-codegen-oms-ci-select-001`, on
`feature/ci-select-001-change-aware-pr-validation`. The original checkout was
left on its unrelated branch. PR #110 (staged Ryzen qualification) and #113
(trusted dispatch) are separate efforts, not incorporated or activated here.

Reconciliation API observations: main protection returned **404 Branch not
protected**, rulesets returned `[]`, repository runners returned zero. The
workflows use GitHub-hosted Ubuntu runners (some with pinned Debian containers),
not a dedicated five-runner pool. No runner registration/security settings were
changed. Actual account concurrency entitlement could **not** be verified:
the available OAuth credential lacks the user/billing access needed by billing
endpoints. Do not infer an entitlement of 20 from historical simultaneous jobs.
The lane cap is independent of that entitlement; less available hosted capacity
can only lower throughput, not raise the five-lane maximum.

## Topology and preservation

Before: CI had 13 job IDs / **16 expanded cells**; Deep CI had **eight jobs**;
Pages had two conditionally triggered jobs. CI and Deep workflow concurrency
cancelled older main runs. This was not an aggregate runner limit. Older main
runs `40434520843` and `40434520755` were actually cancelled.

After:

1. CI always triggers on PR, main push and manual dispatch.
2. `changes` checks out the exact event SHA with complete history, emits and
   uploads the deterministic complete manifest.
3. `baseline` always performs policy/documentation validation, selector and
   scheduling controls, existing split and wrapper guards, and pinned actionlint.
4. Existing ordinary job IDs remain; job-level conditions read the manifest.
   Their original commands, compilers, counts, markers and artifacts remain.
5. `deep-qualification` invokes the local reusable Deep workflow for every main
   push and applicable PR (including this orchestration PR). No runner is
   allocated to a reusable-workflow caller. Nightly/manual Deep remain available.
   The original Deep PR-path list is preserved in the inventory; those paths
   now cause the caller to run the full suite. This avoids a polling gate
   holding a lane while waiting for Deep jobs queued on the same lane.
6. `pr-ci-required` has explicit `always()` and depends on classification,
   baseline, all ordinary jobs and the Deep caller. It executes on main too.
7. Pages keeps its original paths, main-only deployment permissions/environment,
   and latest-publication cancellation semantics; both its jobs acquire common
   main lanes.

The version-controlled machine inventory is
[`/scripts/ci-qualification-inventory.json`](../scripts/ci-qualification-inventory.json).
It contains every original job and expanded check name; compiler/runtime setup;
direct and shared dependencies; test/fixture surfaces; prerequisites; original
steps and normalized step text; exact completion assertions and artifact names;
PR/infrastructure/full-main obligations; and each cell's lane. It is intentionally
verbose so preservation can be reviewed without trusting filename heuristics.
`ci_policy.py` rejects missing/new jobs/workflows, changed runner/matrix/header
policy, changed inherited steps or evidence and bypassed scheduling controls.
Existing Task077B normalized command digests are unchanged. Static policy tests
are not substitutes for compiler-backed qualification.

### Source identity change

Some ordinary jobs previously checked out raw PR head while others used the
synthetic merge. They now explicitly use `github.sha`, with existing exact SHA
assertions updated to that same identity. Main source identity is unchanged.
PR qualification therefore executes the **proposed merge result** classified by
the selector, not a raw head that ignores main changes. The inventory keeps the
original step text as well as the identity-adjusted payload. No moving `main`
checkout replaces qualification source.

## Source-to-qualification policy

The classifier is stdlib-only. It validates full 40-hex base, head and merge
identities, readable commit/tree objects, non-shallow history, exact checkout,
and ordered synthetic merge parents `[event.base.sha, event.head.sha]`.
`git diff --raw -z --no-ext-diff --no-textconv --find-renames BASE MERGE --`
collects the entire proposed result, not a capped API files response and not a
single-commit diff. A second independent `--name-only -z --no-renames` inventory
must agree, detecting even truncation at a complete record boundary. Nonzero
Git commands, malformed/truncated NUL records,
missing/mismatched identities and ambiguous history select full qualification
with an explicit reason. Classification failure is not successful qualification:
the preserved source identity checks and final gate can still fail the run.

Rule order is deterministic and explicit. More specific support rules precede
parent directories, but infrastructure/dependency configuration overrides local
component paths. Deletions, renames, copies, type changes and nonregular files
expand to full ordinary **and Deep** qualification.

| Change | Ordinary PR qualification | Deep |
|---|---|---|
| Nonexecutable README/docs Markdown | Mandatory baseline only | No |
| `docs/task-044-enum-identifier-remapping.md` | Rust (compiled via `include_str!`) | No |
| Python runtime/backend/CLI tests/Python fixtures | Python 3.11 + 3.13 and Rust workspace/shared CLI | Yes (conservative real integration) |
| Float32 support and probe inputs | Float32, guard, Float64 14/16, TimeZulu 14/16, Rust | No unless full fallback applies |
| Lossless/Float64 support and inputs | Float64 14/16, TimeZulu 14/16, guard, Rust | No unless full fallback applies |
| Other Ada support/backend/generated consumers | All Ada jobs and Rust | No unless full fallback applies |
| TimeZulu consumers | TimeZulu, DateTime, Duration, repeated temporal, Float64, guard, Rust | No |
| Frozen temporal corpus | All Ada and Rust | No |
| Rust backend/runtime | Rust workspace/MSRV/Clippy and Python shared contracts | Yes |
| Shared IR/schema/codegen/CLI/contracts/other shared crates | Full ordinary | Yes |
| Cargo/toolchain/workspace config | Full ordinary | Yes |
| Workflows/security/scripts/classifier/inventory | Full ordinary | Yes |
| Other shared fixtures/unknown files | Full ordinary | Yes |

Float32 is **not** independent of Float64: `task097/inherited.py` reruns its
campaigns, and `task098/qualify.py` composes Float32/lossless profiles. Float64
boundary/environment also reuse Float32 probes. Temporal fixtures are frozen
cross-consumer inputs, not documentation. Test-only changes follow owning
inventories; unknown ownership expands to the full suite. Shared fixtures are
not guessed to be local from their basename. PR independence here is at job
granularity; commands inside selected jobs are not filtered or shortened.

## Stable required check: `pr-ci-required`

The final job independently recomputes selection from the complete Git diff and
requires exact manifest equality. It checks every selected ordinary aggregate
`needs` result, prerequisite closure, and every expanded check's authoritative
current-attempt REST job conclusion and step conclusions. Selected failures,
cancellations, missing/ambiguous cells, queued/never-started cells, skipped
qualification steps and missing/expired/empty/stale evidence fail. Required
artifact names are recorded per cell, and original upload steps remain hard
gates. API pagination is checked against authoritative totals, with failures
propagated. Deep cells are checked the same way under their reusable check-name
prefix. Intentionally unselected ordinary/Deep jobs must explicitly be skipped.
There is no `continue-on-error`, success-shaped fallback, or interpretation of an
accidental skipped job as qualification. Failure to run the final gate is not
equivalent to a successful gate.

### Branch-protection migration

No branch settings were changed. There currently are no required checks to
remove. After independent review and hosted genuine-category/injected-failure
demonstrations, an administrator should require `pr-ci-required` and retain any
other applicable security/review requirements. For repositories/environments
with legacy per-job required checks, **do not remove them until** the stable gate
has visibly rejected real and injected failures, cancellation, missing evidence
and missing prerequisites. CI YAML and policy are PR-controlled code; review or
an independently managed required-workflow/ruleset must protect orchestration
changes. A self-consistent malicious rewrite of policy and its guard is not
prevented by tests living in that same PR.

## Five common main lanes and queue policy

Every runner job uses job-level concurrency with `queue: max`; no job has
`cancel-in-progress: true` in that block. Matrix cells carry explicit `slot`
metadata but keep their prior check names. PR groups include workflow, run ID,
attempt, job and matrix variant; they cannot acquire the shared main lane names
or serialize independent PR matrix cells. There is no runner-holding semaphore.

Main CI run identities use unique run IDs; only PR numbers share stale-run
cancellation groups. Deep inherits caller cancellation and has no independent
main-replacement group. A subsequent main merge cannot replace an older required
run merely by sharing its workflow ref. Pages retains its separate latest-only
publication cancellation; its build/deploy cannot bypass the runner budget.

GitHub documents `queue: max` as up to **100 waiting jobs per group**; saturation
can still cancel additional jobs. These cancellations must visibly fail required
qualification, never be counted as passing. The policy guarantees the five-runner
ceiling and preserves queued jobs within this platform limit, not an unbounded
backlog. Monitor queues; do not respond by silently dropping revisions.

Lane assignment was informed by real hosted job durations from CI run
`40262155323`, Deep run `40262155334`, and current-main runs `40435025211` /
`40435025204`. The first Deep sample predates three-way sharding; current-main
shards were used when available. Allocation spreads Float64 14/16, Float32,
TimeZulu and real-UCI shards. It is a static scheduling choice, not a measured
optimal schedule. Rebalance only with inventory and policy review.

| Workflow | Job / expanded check | Variant | Main slot | Prerequisites |
|---|---|---|---|---|
| ci.yml | `ada-float64-codec (14)` | compiler=14 | 1 | none |
| ci.yml | `ada-float64-codec (16)` | compiler=16 | 2 | none |
| ci.yml | `ada-time-zulu-codec (14)` | compiler=14 | 4 | none |
| ci.yml | `ada-time-zulu-codec (16)` | compiler=16 | 3 | none |
| ci.yml | `python-model-backend (3.11)` | python-version=3.11 | 1 | none |
| ci.yml | `python-model-backend (3.13)` | python-version=3.13 | 2 | none |
| ci.yml | `ada-float32-codec` | single; internal compiler loops preserved | 3 | none |
| ci.yml | `ada-repeated-temporal-codec` | single; internal compiler loops preserved | 4 | ada-json-guard |
| ci.yml | `ada-datetime-codec` | single; internal compiler loops preserved | 5 | ada-json-guard |
| ci.yml | `ada-json-guard` | single; internal compiler loops preserved | 5 | none |
| ci.yml | `ada-generated-codec` | single; internal compiler loops preserved | 2 | ada-json-guard |
| ci.yml | `ada-repeated-codec` | single; internal compiler loops preserved | 1 | ada-json-guard |
| ci.yml | `ada-duration-codec` | single; internal compiler loops preserved | 5 | ada-json-guard |
| ci.yml | `ada-hexbinary-codec` | single; internal compiler loops preserved | 4 | ada-json-guard |
| ci.yml | `ada-repeated-choice-codec` | single; internal compiler loops preserved | 5 | ada-json-guard |
| ci.yml | `rust` | single; internal compiler loops preserved | 5 | none |
| deep-ci.yml | `ada-repeated-choice-real-uci` | single; internal compiler loops preserved | 5 | none |
| deep-ci.yml | `real-uci` | single; internal compiler loops preserved | 4 | none |
| deep-ci.yml | `real-uci-regressions` | single; internal compiler loops preserved | 1 | none |
| deep-ci.yml | `real-uci-regressions-ascii` | single; internal compiler loops preserved | 2 | none |
| deep-ci.yml | `real-uci-regressions-alternating` | single; internal compiler loops preserved | 3 | none |
| deep-ci.yml | `msrv-real-uci` | single; internal compiler loops preserved | 5 | none |
| deep-ci.yml | `real-sleet` | single; internal compiler loops preserved | 2 | none |
| deep-ci.yml | `ada-hexbinary-real` | single; internal compiler loops preserved | 5 | none |
| uci-pages.yml | `build` | single; internal compiler loops preserved | 4 | none |
| uci-pages.yml | `deploy` | single; internal compiler loops preserved | 4 | build |
| ci.yml | `changes` | single | 5 | none |
| ci.yml | `baseline` | single | 5 | changes |
| ci.yml | `pr-ci-required` | single | 5 | all selected qualification and Deep |

## Local and hosted validation

Local evidence is retained outside the checkout at
`/home/zboll/ci-select-001-evidence`; no historical qualification evidence was
deleted. Focused tests cover documentation/Python/Float32/shared/test ownership,
frozen fixtures, real multicommit Git merges with advancing main and incorporated
main, actual deletions/renames, malformed/truncated/failed diff collection,
missing identities/history, prerequisite closure, hidden selected failures,
cancellations/skips/missing evidence, matrix lanes, six-lane bypass, missing/new
jobs/workflows, Pages budget and prior-main cancellation. Interval-report tests
reject six simultaneous jobs and do not count pending timestamps as execution.

Actionlint **1.7.12** is pinned and hash-verified. It predates GitHub's documented
`queue` field. `check-ci-actionlint.sh` suppresses **only** its specific unknown
`queue` diagnostic, while the independent policy checker requires exact
`queue: max` blocks. No expression/matrix/context errors are suppressed. Shellcheck
and pyflakes integrations are disabled (not available locally); this is disclosed,
not claimed as a full shellcheck run. GitHub accepted the actual pilot YAML.

The hosted pilot uses separate branch `ci-select-001-five-lane-pilot`, commit
`9b9ed6c`, 15 short jobs, five **synthetic** lane names containing the run ID,
and no production lane. It is not a qualification replacement. Run:
<https://github.com/zackboll/ams-gra-codegen-oms/actions/runs/40438716306>.
At the initial checkpoint jobs remained queued and none had cancelled; completed
pilot metrics will be recorded separately. Do not infer maximum concurrency from
pending jobs' `started_at`, which GitHub can set before actual execution.

### Acceptance ledger (do not interpret pending as passed)

| Requirement | Evidence/status |
|---|---|
| Focused selector/gate/scheduler tests | 29 passed locally |
| Underscore Python discovery | 75 passed locally |
| `check-ci-split.sh` | Passed |
| `test-check-ci-split.sh` | 477 checks passed, including final repeat |
| Task077B layout self/mutation test | 3 tests passed; 312 mutation cases per inspected fixture |
| Task060 / Task061 wrappers | Passed (including adversarial failure/marker cases) |
| Task093 compiler placement | 12 passed, including exact-only PR condition control |
| `cargo fmt --all -- --check` / `git diff --check` | Passed locally |
| Actionlint + exact queue guard | Passed with disclosed single unsupported-key diagnostic suppression |
| Controlled hosted queue pilot | In progress |
| Orchestration PR full ordinary + Deep | Pending; this PR must select all 16 + 8 cells |
| Genuine hosted docs/Python/Float32/shared/CI/unknown categories | Pending |
| Hosted selected failure + missing prerequisite | Pending |
| First actual merged-main full qualification | Not authorized / not performed |
| Subsequent main merge preservation | Not authorized / not performed |

## Measurements and follow-up procedure

No PR speedup, final main wall time, production maximum concurrency, actual
runner-minute saving or queue-wait improvement is claimed. Static selected-cell
counts are not measured elapsed time. Expected ordinary counts are 0 for harmless
docs, 3 for Python (two Python + Rust), 7 for Float32 support (Float32 + guard +
two Float64 + two TimeZulu + Rust = **7**), and 16 for shared/CI/unknown paths.
Baseline/classifier/final gate overhead is additional; Python conservatively
also runs all eight Deep cells, so its real elapsed-time benefit is unproven.

For hosted category acceptance, independently create genuine commits atop the
implementation with one category per candidate and generate verified merge
objects against the corresponding base. Do not reuse fabricated manifest input
as hosted proof. Record merge/base/head identities, selected/skipped cell list,
job evidence and final stable conclusion. Intentional failures must be reverted
before final-head qualification. The implementation PR's own infrastructure diff
must continue to select full ordinary + Deep; appending README changes to it is
**not** a documentation-only demonstration.

After explicit merge authorization and review, fetch job records with complete
pagination for the **actual merged commit** and all participating workflows.
`ci-concurrency-report.py` processes complete successful REST job records, using
real setup-step timestamps and half-open execution intervals; it fails on missing
or cancelled jobs and on observed concurrency above five. Include triggered Pages,
nightly/manual main-ref jobs overlapping the interval, all matrix cells and all
workflow attempts. Keep raw timestamps. Compute queue waits from actual
eligibility/run records, not pre-run placeholders. Confirm a subsequent main
merge does not cancel/replace the prior qualification. Historical PR success is
not post-merge proof.

Five lanes reduce competing repository slot consumption, not qualification work.
Main wall time and queue waits can increase as backlog grows. Reusable Deep
qualification avoids a waiting runner; classification/baseline/final gate still
consume brief runner time, and documentation PRs trade that overhead for skipping
heavy campaigns. Public-hosted billed minutes and execution minutes are distinct.

## Rollback

Before merge: close the implementation PR; no production branch/ruleset changed.
After a reviewed deployment: revert the orchestration commit (including selector,
inventory, guard adaptations and reusable Deep topology) in a normally reviewed
PR, rerun full qualification, and coordinate any required-check migration so the
branch cannot merge without a real gate. Reverting removes the five-slot policy;
reserve capacity or pause new merges until a corrected cap is in place. Never
disable compiler tests, alter frozen fixtures or change runner security as an
emergency performance workaround. No merge, approval, auto-merge, tags, releases
or Alire-index operations are part of this work.

## R1 — current-main reconciliation (2026-10-28 UTC)

### Identity and inherited source record

This appendix preserves, rather than supersedes, the original implementation
record above. Starting PR #116/local/remote head:
`a02903e07e58a927d4c6713b6b2c028116d38b39`, tree
`bb1b5669f04ddfa843c1f79cd293470e231aa817`. The feature worktree was clean.
Previously incorporated main was `77066e8e2cf4875274c0227cee16dcf0dc593c05`.
Fetched main remains `d603077c6d38ccc0658391776e3af9eecb009255`, tree
`5ea3873e3bf6a8e2960e8d1faa57049d867a2456`.

The normal merge (no rebase/reset/cherry-pick/amend/squash/force-push) preserves
both parents. Its only conflict was the new jobs insertion in `ci.yml`: keep
CI-SELECT orchestration AND the complete Task099 job. Seven newer main commits:

- `4c79fff41970a8ba7109df9bd185a6b166c0ffb0` ci: add disabled automatic trusted PR dispatch
- `8cc8049d34638f616cbd9caa22bb110f88f7a5d4` ci: bind trusted PR authorization to immutable workflow provenance
- `ce76179da96ad44a9bb396b478db3040a64b07f2` feat(ada): add transport-independent LA-CAL runtime core and typed bindings
- `ad1f347c5928d87e869e78db5940e1e99311bb0c` Merge remote-tracking branch 'origin/main' into feature/099-ada-lacal-runtime-core
- `5fff5e0f3d2c3a2bed81c55e57e7af48fede9e5b` fix(ada): enforce typed subscription ownership before unsubscribe
- `4978c1c5728d062b9ea64bbcdb39aa96b18361a3` Merge pull request #115 from zackboll/feature/099-ada-lacal-runtime-core
- `d603077c6d38ccc0658391776e3af9eecb009255` Merge pull request #113 from zackboll/ci/002-trusted-pr-dispatch

Original CI [40440457015](https://github.com/zackboll/ams-gra-codegen-oms/actions/runs/40440457015),
attempt 1, passed 27 jobs including all eight Deep jobs and `pr-ci-required`
on the starting PR head. Original pilot
[40438716306](https://github.com/zackboll/ams-gra-codegen-oms/actions/runs/40438716306)
passed 15 jobs at peak TWO. Neither qualifies the R1 implementation; the latter
never established five-way saturation. Raw starting PR/check/run/job/artifact
metadata was saved in `/tmp/ci-select-001-r1-evidence` before modification.

### Task099 preservation and dependency policy

Both `ada-lacal-runtime-core (14)` and `(16)` remain compiler-backed jobs, with
unchanged Debian digest, glibc 2.41 verification, GNAT14.2.0/external and pinned
GNAT16.1.0 setup, verified Alire 2.1.1, locked generator build, fresh GNATCOLL
inputs, source-tree/cleanliness checks, mock transport and typed OMS JSON
execution. Each compiler requires 24 combinations: OAM/qualified namespaces,
portable/lossless profiles, Normal/Check/Ignore assertions and O0/O2. Check counts
remain 53 portable and 62 lossless. All Task099 markers, result assertions and
artifact paths are preserved. Production Ada, generators, qualification scripts
and fixtures are byte-identical to incorporated main. New inheritance tests use
independent payload hashes derived from the verified main git object during
reconciliation; they do not learn expected commands from candidates or require
history in inherited shallow Rust/Pages checkouts.
Only checkout and its exact-SHA assertion change from PR head to `github.sha`,
consistent with CI-SELECT's verified synthetic merge.

Runtime and owned runtime-test changes select both compilers, Rust and the JSON
guard. Shared Ada/OMS JSON/Float32/lossless/temporal changes conservatively include
Task099. Shared IR/generator/dependency and all infrastructure changes select all
18 ordinary cells plus all eight Deep jobs. Unknown Task099 paths fail broad;
qualification scripts and service-generation fixtures also fail broad. Local
category manifests with exact selected/skipped IDs are recorded in
`docs/evidence/ci-select-001-r1.json`; these are synthetic local controls, NOT
hosted PR-event acceptance.

### Complete runner inventory and scheduling

There are **34 expanded runner cells**, not the historical 29: **18 ordinary,
eight Deep, two Pages, three orchestration, three security-workflow cells**.
**33 are main-eligible**, including the conditional dispatcher. The signal is
PR-only and is not a main lane occupant. Every main-eligible runner acquires
exactly one of `oms-main-slot-1` through `oms-main-slot-5`, using job-level
`queue: max`. No sixth lane, runner semaphore or reserved runner pool exists.
The inventory JSON records workflow/job, trigger, runner, variants, eligibility,
lanes, source policy, purpose, prerequisites, expected artifacts and conditional
behavior. Matrix `slot` values never alter inherited compiler variants/commands.

| Workflow | Expanded runner check | Hosted runner | Main slot | Prerequisites |
|---|---|---|---|---|
| `ci.yml` | `ada-float64-codec (14)` | `ubuntu-24.04` | 1 | none |
| `ci.yml` | `ada-float64-codec (16)` | `ubuntu-24.04` | 2 | none |
| `ci.yml` | `ada-time-zulu-codec (14)` | `ubuntu-24.04` | 4 | none |
| `ci.yml` | `ada-time-zulu-codec (16)` | `ubuntu-24.04` | 3 | none |
| `ci.yml` | `python-model-backend (3.11)` | `ubuntu-24.04` | 1 | none |
| `ci.yml` | `python-model-backend (3.13)` | `ubuntu-24.04` | 2 | none |
| `ci.yml` | `ada-float32-codec` | `ubuntu-24.04` | 3 | none |
| `ci.yml` | `ada-repeated-temporal-codec` | `ubuntu-24.04` | 4 | ada-json-guard |
| `ci.yml` | `ada-datetime-codec` | `ubuntu-24.04` | 5 | ada-json-guard |
| `ci.yml` | `ada-json-guard` | `ubuntu-24.04` | 5 | none |
| `ci.yml` | `ada-generated-codec` | `ubuntu-24.04` | 2 | ada-json-guard |
| `ci.yml` | `ada-repeated-codec` | `ubuntu-24.04` | 1 | ada-json-guard |
| `ci.yml` | `ada-duration-codec` | `ubuntu-24.04` | 5 | ada-json-guard |
| `ci.yml` | `ada-hexbinary-codec` | `ubuntu-24.04` | 4 | ada-json-guard |
| `ci.yml` | `ada-repeated-choice-codec` | `ubuntu-24.04` | 5 | ada-json-guard |
| `ci.yml` | `rust` | `ubuntu-latest` | 5 | none |
| `deep-ci.yml` | `ada-repeated-choice-real-uci` | `ubuntu-24.04` | 5 | none |
| `deep-ci.yml` | `real-uci` | `ubuntu-latest` | 4 | none |
| `deep-ci.yml` | `real-uci-regressions` | `ubuntu-latest` | 1 | none |
| `deep-ci.yml` | `real-uci-regressions-ascii` | `ubuntu-latest` | 2 | none |
| `deep-ci.yml` | `real-uci-regressions-alternating` | `ubuntu-latest` | 3 | none |
| `deep-ci.yml` | `msrv-real-uci` | `ubuntu-latest` | 5 | none |
| `deep-ci.yml` | `real-sleet` | `ubuntu-latest` | 2 | none |
| `deep-ci.yml` | `ada-hexbinary-real` | `ubuntu-24.04` | 5 | none |
| `uci-pages.yml` | `build` | `ubuntu-latest` | 4 | none |
| `uci-pages.yml` | `deploy` | `ubuntu-latest` | 4 | build |
| `ci.yml` | `ada-lacal-runtime-core (14)` | `ubuntu-24.04` | 1 | none |
| `ci.yml` | `ada-lacal-runtime-core (16)` | `ubuntu-24.04` | 3 | none |
| `ci002-validation.yml` | `test` | `ubuntu-24.04` | 2 | none |
| `oms-trusted-pr-signal.yml` | `signal` | `ubuntu-24.04` | PR only | none |
| `oms-ryzen-trusted-dispatch.yml` | `dispatch` | `ubuntu-24.04` | 4 | none |
| `ci.yml` | `changes` | `ubuntu-latest` | 5 | none |
| `ci.yml` | `baseline` | `ubuntu-latest` | 5 | changes |
| `ci.yml` | `pr-ci-required` | `ubuntu-latest` | 5 | ordinary + Deep + baseline |

CI ordinary/orchestration triggers remain PR, main push and manual; Deep is the
reusable all-eight-job dependency. Pages remains main/path-filtered or manual,
latest-only, and build precedes deploy. Ordinary prerequisites above additionally
include `changes`; selected prerequisites close transitively. Unique non-main
identities include workflow/run/attempt/job/variant, never a shared main group.
CI002 PRs cannot cancel/overwrite each other. Main runs retain unique workflow
identities, so newer merges cannot cancel prior qualification. The finite GitHub
100-waiter group limit still applies; overflow must fail visibly, not pass.

### Security boundary and independent checks

* **CI002 Security Contracts** retains PR and main triggers, read-only permissions,
  exact PR-head/main checkout and assertion, pinned checkout, hashed PyYAML
  source installation, all 30 security/mutation tests, and authenticated historical
  workflow provenance for run 40108707517/revision 4c79fff. It acquires slot 2 on
  main. This is an independent mandatory check, not selected/optional CI.
* **OMS Trusted PR Signal** retains every PR lifecycle event, `permissions: {}`,
  one inline signal-only command, no checkout, no dispatch credential. Its
  unique PR-only concurrency group is inventoried; the entire workflow header
  and payload are pinned against unreviewed changes.
* **OMS Ryzen Trusted Dispatch** retains workflow_run/completed semantics,
  feature gating, default-disabled behavior, destination-only App permissions,
  independent source/PR authorization, immutable executed-workflow provenance,
  private authorization, receipt and anti-replay checks. `scripts/ci002/**` is
  unchanged and hash-protected against inherited main. Passing `pr-ci-required`
  never authorizes dispatch. No credential/variable/secret was created or changed.

Actual hosted workflow_run 40456471102 reports `head_branch: main`,
`head_sha: d603077...`, event `workflow_run`, conclusion skipped. Its source
context is the default branch, NOT the signal's PR commit. The dispatcher now
acquires slot 4. Its existing per-PR workflow concurrency identity and
`cancel-in-progress: false` remain; **workflow-level `queue: max` is added** to
avoid pending-run replacement ahead of the main lane. Receipt/provenance checks
still independently reject stale/replayed work. The enable variable was absent
(404); no attempt was made to activate dispatch. Signal workflow-byte changes
remain denied by the existing default-branch provenance boundary until a reviewed
main deployment establishes the new trusted baseline.

### Fail-closed gate and policy controls

Closed-world workflow/job inventory, normalized original commands, original step
payloads, matrix compiler identities, lane assignments, Pages publication policy
and queue/cancellation controls remain protected. Missing security workflows,
signal token permissions, removed dispatch gates/authorization, unknown variants,
omitted Task099 cells, lane bypasses, and weakened inherited markers are rejected.
Comments and unrelated inventory metadata remain accepted.

`pr-ci-required` includes both Task099 needs. It independently recomputes the
manifest, requires authoritative successful completed job identities and named
mandatory steps, nonempty fresh/nonexpired expected artifacts, mandatory baseline
and classifier, selected prerequisites and all selected Deep cells. Intentional
skips must match independently proven exclusions. Final verification additionally
checks run/attempt/event/head, exact checkout and merge parents, and incorporated
main ancestry of source and expected base. Invalid classifier inputs select full
qualification; unverifiable source identity cannot pass the final gate.

**Limitation:** the aggregate checks only its own reusable CI graph. It does NOT
verify independent CI002 or signal workflow results. Preserve separately required
CI002 `test` (workflow: CI002 Security Contracts) and signal `signal` statuses
when migrating branch protection. Require the stable `pr-ci-required` plus these
independent checks; do not remove legacy required compiler checks until genuine
selective/failure acceptance completes. Independently protect orchestration and
security-policy edits with review/rulesets. The branch-protection REST endpoint
reported main unprotected (404); no protection configuration was changed.

### Local qualification

All mandated local commands passed: split guard; split mutation self-test
(**477 controls**); Task077B layout (**3 tests, including 312 mutations per layout**);
Task060 wrappers (**96 checks**); Task061 wrappers (**4 checks**); Task093 placement
(**12 tests**); selector/final-gate/security scheduling suite (**48 test executions**
after removing duplicate imported test discovery); CI002 (**30 tests**); standalone
policy (**34 cells/five lanes**); documentation guard; actionlint **1.7.12** with
only the previously documented queue-key compatibility diagnostic excluded;
`cargo fmt --all -- --check`; and `git diff --check`.

`cargo test --workspace --locked` passed with local GNAT-backed tests. Full script
unittest discovery passed **94 tests** after supplying missing `strace` from an
extracted Debian package under the temporary evidence directory. The first broad
script run's two missing-strace errors are retained in the raw logs, not hidden.
Full Python runtime integration passed **21 tests** against existing pinned Sleet
`e38f61d8ce0d75c8508434a52f2ed77c69cf6a3b` and websockets 17.2 in a temporary venv;
mock qualification passed **19 tests in each normal/optimized run**. An initial
unconfigured runtime discovery correctly failed without dependencies/broker;
configured reruns passed without skips. No test expectations/commands were
weakened to repair these environment deficiencies.

### Strong isolated pilot and measurements

R1 pilot [40607545508](https://github.com/zackboll/ams-gra-codegen-oms/actions/runs/40607545508),
attempt 1, workflow `ci-select-pilot.yml`, push source
`ec2de3db9961f8806000d3e4ef09c4545a7f6c93`, passed **15/15** jobs with no cancellation.
Three independently queued cells per lane use run-unique synthetic groups,
**not production main lanes**. The hold window increased from 30 to 150 seconds.
Real first-step/completion intervals prove **peak five**, **at most one per lane**,
and eventual successful completion of every submitted job. Timestamp tests also
reject overlapping jobs in a single lane even below the global ceiling.

Measured **37.967 execution runner-minutes**, **465 seconds** wall interval.
Run-start-to-first-setup delay: **4–316 seconds**, mean **159.667 seconds**. These
include both lane wait and hosted scheduling; pure scheduler queue time is not
available and is not inferred from queued jobs' `started_at`. Complete interval
and peak evidence is in the version-controlled evidence JSON. No acceleration,
production-main improvement, account concurrency entitlement, or reservation of
15 runners is claimed.

### Hosted acceptance and handoff status

**HOSTED_CI_PENDING** until exact-final-head full hosted CI and independent
security checks pass AND the genuine selective/failure controls are complete.
Eight local category controls cover docs, Python, Float32, LA-CAL, shared IR,
security/CI, unknown path and test-only changes. Local mutation tests cover ordinary
and Deep failure, absent/cancelled/skipped cells, missing/stale artifacts, invalid
manifest and absent baseline. These are NOT genuine hosted PR runs or hosted
failure injections. No disposable category PRs or production failure injections
have been created in R1; their scheduling/final-gate acceptance remains outstanding.
They must be bounded, unmerged, nonprivileged fixtures, never replacement PRs.

CI-infrastructure changes force conservative full qualification: **18 ordinary
cells, eight reusable Deep jobs, classifier, baseline, aggregate** = 29 CI runner
jobs, plus independent CI002 and PR signal. Exact final source/tree/run/attempt
and check conclusions belong in the PR #116 R1 handoff, after normal push and
local/remote/PR identity reconciliation. A later documentation commit requires a
new exact-head run; historical or intermediate-head green runs cannot qualify it.
No reviewer-ready disposition is claimed while these controls remain pending.

### R1 rollback and operational restrictions

Before merge, retain PR #116 and correct via normal follow-up commits; no
replacement implementation PR or history rewrite. After reviewed deployment,
revert the R1 orchestration changes with a normally reviewed PR while retaining
all current-main Task099 production/security content and full qualification.
Coordinate legacy/aggregate required-check migration before removing a guard;
pause merges if the five-lane cap is absent. Keep dispatcher disabled and its
provenance/receipt policy intact. No merge, approval, auto-merge, release, tag,
Alire index, self-hosted runner, secret or feature-flag operation is authorized.
