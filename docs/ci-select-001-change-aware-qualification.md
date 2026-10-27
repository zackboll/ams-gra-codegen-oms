# CI-SELECT-001 — change-aware PR qualification and five main lanes

## Disposition and source reconciliation

**HOSTED_CI_PENDING**. This is not merge authorization. Independent review,
hosted integration categories and actual merged-main acceptance remain required.

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
single-commit diff. Nonzero Git commands, malformed/truncated NUL records,
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
| Focused selector/gate/scheduler tests | 28 passed locally |
| Underscore Python discovery | 74 tests after final additions; rerun evidence required |
| `check-ci-split.sh` | Passed |
| `test-check-ci-split.sh` | 477 checks passed; final repeat pending |
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