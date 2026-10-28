# PERF-007 — parallel primary real-UCI qualification

## Disposition and delivery restrictions

**HOSTED_CI_PENDING**. This document does not authorize merge. No measured
candidate speedup, exact-head hosted success, overall Deep nonregression, or
review-ready status is claimed until the evidence below has been collected.
PR #116 must be successfully qualified and integrated first. Do not merge,
approve, enable auto-merge, rewrite history, or force-push either PR.

Parent PR: <https://github.com/zackboll/ams-gra-codegen-oms/pull/116>, feature
branch `feature/ci-select-001-change-aware-pr-validation`.
Reverified parent head: `befed75bf2a4dc68e0ee5c562545a4ed70ebceda`.
Parent tree: `bb5327a76ceaede12cb5f94a243b403c477f1ff3`.
Fetched main: `d603077c6d38ccc0658391776e3af9eecb009255`.

Implementation branch: `feature/perf-007-shard-real-uci`. Isolated worktree:
`/home/zboll/git/ams-gra-codegen-oms-perf-007-real-uci` (explicitly approved path
change). The originally requested path was occupied by unrelated, uncommitted
Float64 work belonging to another repository; it is left entirely untouched.
PR #116's branch and worktree are not modified.

## Independently inspectable serial baseline

Historical successful standalone Deep run:
<https://github.com/zackboll/ams-gra-codegen-oms/actions/runs/40451020802>.
Source: `904ad3baebbf237a91e118c91457ac7207e8edd9`.
Primary job: `122013509549`.
Run starts `2026-10-27T04:40:25Z`, completes `2026-10-27T06:53:28Z`:
**2h13m03s** run elapsed, including scheduling.
Job API interval: `05:42:58Z–06:53:27Z`: **70m29s**.
Actual first setup step `05:42:59Z` to job completion: **70m28s execution**.
Eligibility-to-first-setup delay: **62m34s**, including scheduler/provisioning
delay; not a measurement of pure scheduler queue time.
First task begins `05:46:19Z`: **3m20s** initial setup.
Last task finishes `06:53:24Z`; final cleanup is separately visible in the API.

The complete API snapshots, original job/step inventory and raw-log SHA-256 are
in `docs/evidence/perf-007-baseline.json`. The raw log can be independently
retrieved through the recorded GitHub job logs endpoint while retained.
These durations are observations, not future guarantees. The workflow at the
PR #116 parent was inspected before edits: no additional primary gates existed.

A second, directly comparable reusable-CI serial baseline has now completed:
run [40609156659](https://github.com/zackboll/ams-gra-codegen-oms/actions/runs/40609156659),
exact PR #116 head `befed75bf2a4dc68e0ee5c562545a4ed70ebceda`, conclusion
**success**, including `pr-ci-required`. Primary job `122556890959` ran
`2026-10-28T00:32:20Z–01:59:49Z`: **87m29s execution**. Full CI run
`00:30:10Z–02:00:02Z`: **89m52s elapsed**. Task times are 061 19m18s,
071 0m30s, 069 12m52s, 062 16m11s, 063 12m35s, 065 13m49s,
066 11m45s. Both original and parent baselines must be reported; do not
silently select the slower baseline to make performance appear better.
The complete parent API snapshots are in `docs/evidence/perf-007-parent-baseline.json`.
PR #116 was externally merged at `2026-10-28T02:03:48Z` as
`d50bf7eac1fee0011683cbe8f4e804cebd52d341`, tree
`bb5327a76ceaede12cb5f94a243b403c477f1ff3` (identical to verified parent).
This agent did not approve or merge it. PERF-007 will incorporate that main
commit normally, retarget the single PR, and require new final-head qualification.

| Original order | Task | Exact command | Historical group duration |
|---|---|---|---|
| 1 | 061 | `bash scripts/check-task061-pinned.sh` | 14m18s |
| 2 | 071 | `bash scripts/check-task071-uci.sh` | 0m26s |
| 3 | 069 | `bash scripts/check-task069-pinned.sh` | 11m12s |
| 4 | 062 | `bash scripts/check-task062-pinned.sh` | 12m40s |
| 5 | 063 | `bash scripts/check-task063-pinned.sh` | 9m36s |
| 6 | 065 | `bash scripts/check-task065-pinned.sh` | 10m01s |
| 7 | 066 | `bash scripts/check-task066-pinned.sh` | 8m52s |

Original setup steps: checkout `github.sha`; verify HEAD/tree/event; stable
Rust toolchain; install GNAT; require GNAT command/version; fetch and verify
both pinned UCI roots. The complete exact step bodies, compiler settings,
dependencies and old check name are retained in the baseline JSON.

## Job graph and check-name migration

Previously `CI / deep-qualification / real-uci` executed all seven sequentially.
It is removed, not replaced with a successful placeholder. The reusable caller
and independently evaluated `pr-ci-required` remain. New mandatory jobs:

| Job/check suffix | Ordered task inventory | Main slot | Historical task sum |
|---|---|---|---|
| `real-uci-temporal-naming` | 061, 071, 065 | 4 | 24m45s |
| `real-uci-schema-ipv6` | 069, 062 | 1 | 23m52s |
| `real-uci-integral-unicode` | 063, 066 | 3 | 18m28s |

Each check is prefixed `deep-qualification / ` in the CI caller. Any external
required-check configuration referencing `real-uci` must migrate explicitly to
the three checks or to the independent `pr-ci-required` gate; do not leave an
old, never-reported required name. No repository protection settings are changed.
The qualification inventory replaces one record with three actual job records.
Deep goes from eight to ten mandatory jobs; the full policy inventory goes from
34 to 36 expanded runner cells, **not** from five to seven concurrent main lanes.

All non-target Deep jobs retain their payloads and mandatory status:
`ada-repeated-choice-real-uci`, `real-uci-regressions`,
`real-uci-regressions-ascii`, `real-uci-regressions-alternating`,
`msrv-real-uci`, `real-sleet`, `ada-hexbinary-real`.

## Dependencies and exact qualification

No generator, IR, backend, fixture, codec or generated-source semantics change.
All seven wrappers are byte-for-byte inherited and SHA-256 pinned. Their
original workflow run bodies and step names remain; a custom Python shell
executes each exact run body under Bash `-e -o pipefail`, streaming all combined
output to the hosted log and an untruncated per-task log. Nonzero command status
is preserved. Marker/count validation adds requirements; it removes none.

Every shard independently checks out `github.sha`, asserts HEAD/tree, initializes
private evidence, installs/verifies GNAT and stable Rust, fetches both pinned
roots using unchanged tag/hash-verifying scripts, explicitly `cargo fetch
--locked` and builds the release generator. This removes implicit CLI/Cargo
dependencies formerly satisfied by preceding tasks. Cargo targets and temporary
files stay runner-local; there is no cache or build-artifact transfer.

Both immutable release identities are required in every shard:

| Release | Tag revision | Root SHA-256 |
|---|---|---|
| 2.5 | `093610b7753944059360d3236770ab446d039556` | `ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27` |
| 2.6 | `78eb61b6112c8bffa40820c33124b57787fc5bd9` | `af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b` |

ClosedSchemaSet/OpenExtensions, real Ada/Rust/C++ compilation, readiness and
first-blocker measurements, naming checks, exact counts, and negative/regression
controls remain in the unchanged wrappers and their existing consumers.

## Structured evidence and reconciliation

`scripts/real-uci-contract.json` records the complete original command, wrapper
hash, marker **multiplicities**, exact Rust result counts and naming row counts.
Its reviewed SHA-256 is independently pinned in `scripts/real_uci_evidence.py`.
Task065 includes all 66 former-gap generation and two compile PASSED rows, not
only summary markers. Original test result tuples are `(passed, failed, ignored,
measured, filtered_out)`: Task061 twice `(1,0,0,0,3)`; Task069 `(1,0,0,0,0)`;
Task062 `(1,0,0,0,0)`; Task063 twice `(1,0,0,0,1)`; Task065 `(1,0,0,0,7)`;
Task066 `(1,0,0,0,0)`. Task071 is the unchanged Python/CLI route qualification,
not a Rust test. Original selected tests and listing checks remain unchanged.

Static layout and CI policy prove exact ordered ownership and a disjoint
seven-task union across all Deep jobs. Each shard emits:

- `identity.json`: actual source SHA/tree, shard, run/attempt and start time;
- `setup/{gnat,fetch,build}.{json,log}`: full commands, statuses and wall times;
- `inputs.json`: both verified tag/revision/hash identities, toolchain versions,
  contract/workflow/inventory/wrapper source hashes;
- `task/NNN.{json,log}`: full exact command/output, status, timestamps, elapsed
  time, marker multiplicities, result counts and output size/SHA-256;
- `diagnostics/**`: generated log/manifest/lock/source-digest diagnostics when
  available, excluding Cargo targets, executables and symlinks;
- `completion.json` and `completion.sha256`: status, ordered task inventory,
  setup and shard elapsed time, and every evidence file's size/hash.

Internal setup elapsed begins at evidence initialization after checkout/source
assertion; internal shard elapsed ends before artifact upload. The performance
report uses API first-setup-to-job-completion intervals to include **all** setup,
diagnostic packaging, upload and cleanup in primary execution comparisons.

Reconciliation and diagnostic upload use `always()`; only those exact evidence
steps may run after a failed gate. Uploads are mandatory nonempty artifacts named
`perf007-<job-id>`. A failed/skipped/cancelled task or setup cannot qualify.
Missing/extra task files, altered/truncated logs, missing/duplicate markers,
changed test counts, mismatched hashes or stale source/run/attempt fail closed.

`pr-ci-required` independently checks all three authoritative job records,
successful named steps and fresh nonempty artifacts. It then downloads artifact
**data**, without extracting paths or executing artifacts, verifies file hashes,
re-parses each task's complete output, checks exact source/tree/run/attempt and
proves the runtime disjoint union. API credentials are not forwarded to signed
archive-download hosts. All existing non-target final-gate checks remain.

## Five-lane scheduling and measured inputs to the model

The five shared `oms-main-slot-1` through `oms-main-slot-5` groups, `queue: max`,
PR-unique identities, stale-PR cancellation and noncancelling main behavior are
unchanged. No extra workflow, polling runner, unbounded main runner group,
self-hosted placement, token permission or CI002 trust change is introduced.

Parent run `40609156659`, source `befed75...`, supplies completed **execution-step**
durations for other Fast/Deep cells. The initial API `started_at` is not used as
execution evidence for queued jobs. Lane 2 carries Float64(16), Sleet and ASCII
regressions. Move only real-Sleet from lane 2 to lane 4; its payload is unchanged.
Assign shards A/B/C to 4/1/3. The observations and explicitly hypothetical setup
allowance are in `docs/evidence/perf-007-lane-model.json`.

| Lane | Before load model (minutes) | Candidate load model (minutes) |
|---|---:|---:|
| 1 | 90.13 | 118.00 |
| 2 | 126.55 | 101.93 |
| 3 | 101.62 | 124.08 |
| 4 | 132.30 | 115.20 |
| 5 | 115.47 | 115.47 |

These sums are **not measured candidate makespan**. Candidate shard load uses
historical task sums plus an illustrative 240-second setup per shard; it does
not establish nonregression or speedup. Classifier/baseline/final, independent
security/Pages jobs and competing main runs also consume lanes. The static
policy and mutations prove at most five lane cells run simultaneously, one per
lane, but actual throughput remains subject to hosted capacity and queueing.
`scripts/ci-concurrency-report.py --inventory-lanes` can reconcile complete
hosted main API intervals against the updated inventory. PR runs intentionally
remain independent of the shared main lane cap, as in PR #116.

## Performance acceptance and hosted handoff

Candidate setup/group/shard times, maximum shard execution duration, complete
Deep critical path, runner-minutes and queue delays: **pending actual hosted
qualification**. The historical task balance suggests a substantially shorter
primary path; no projected percentage is reported as measured.

Capture complete run/job API responses and download all three artifacts from
the final-head CI run. Use `scripts/real-uci-performance.py` with original and
candidate run/jobs JSON, artifact directory, `--source-sha` (the checked-out
synthetic merge SHA for pull_request) and `--source-tree`. It fails on incomplete
or unsuccessful runs and validates bundles before reporting:

1. Primary execution reduction: `1 - max(shard execution)/serial execution`;
   target **at least 35%**, with real setup included on both sides.
2. End-to-end elapsed improvement, including queueing, reported separately.
3. Primary and whole-Deep aggregate runner-minute changes.
4. Deep max job execution, eligible-to-completion elapsed, and per-job
   eligibility-to-first-setup delay (lane wait plus scheduling/provisioning).

Compare the parent reusable CI run as well as the historical standalone Deep
run: combined CI elapsed versus standalone Deep elapsed is not like-for-like.
Do not infer queue wait solely from the job API's early `started_at` field.
If runner contention/setup costs defeat the target or delay non-target Deep
jobs, record that blocker and use PERFORMANCE_TARGET_NOT_MET, not success.
No fourth shard is implemented or presumed necessary.

Before READY_FOR_INDEPENDENT_REVIEW require final PR head/run/attempt identity,
all three actual shard successes, verified nonempty artifacts, final-gate
runtime reconciliation, unchanged non-target Fast/Deep/security qualification,
five-lane scheduling proof and measured performance/nonregression. Hosted checks
not yet complete cannot be replaced with local synthetic verifier fixtures.

## Negative controls and local validation

Automated mutations cover: missing shard; omitted task; cross-shard duplicate;
every removed marker; skipped/changed test counts; actual command exit 23 with
diagnostics; altered UCI hash; missing/truncated/stale artifact contents;
skipped/cancelled jobs; successful placeholders; six lanes; and main cancellation.
Existing security/inventory/selector/split negative controls remain active.
The new evidence/performance tests run in mandatory baseline, alongside the
existing selector tests, split guards, wrappers and pinned actionlint policy.

Local validation records are summarized in the PR handoff. Required commands:
actionlint with existing queue exception; check-ci-split and its mutation suite;
CI-SELECT inventory/selector/final-gate tests; Task060/061 wrapper tests; all
current CI002 tests; Cargo fmt/check/clippy/tests; `git diff --check`.
Any incomplete test is a limitation, not a pass.

Implementation local results: pinned actionlint/repository policy PASS;
check-ci-split PASS; full split mutation suite PASS (**529 checks**, including
364-case layout controls); selector/inventory/final-gate suite PASS (**48 tests**);
new evidence/performance controls PASS (**18 tests**, with each original marker
individually mutated); CI002 PASS (**30 tests**); Task060 wrappers PASS (**96
checks**); Task061 wrappers PASS (**4 checks**); Cargo fmt/check/clippy `-D
warnings`/workspace tests PASS; `git diff --check` PASS. The workspace command
retains existing ignored hosted-only tests and is not claimed as real-UCI
qualification. All retained non-target Fast/Deep step payloads were independently
compared against the parent and are unchanged. Earlier transient mutation-fixture
and lane-report errors were corrected and rerun; no gate was suppressed.

## Rollback and parent integration

Before merge, correct via ordinary additive commits on the single stacked PR.
After reviewed deployment, revert PERF-007 in a normal PR, restoring the exact
serial seven-task payload and coordinating required-check names. Keep all
CI-SELECT-001 trust, final-gate and five-lane policies. Never retain both serial
and shard execution or substitute an unconditional green placeholder.

After PR #116 integrates successfully, fetch main, reconcile by a normal merge
commit if necessary (no reset/rebase/amend), change this PR's base to main and
repeat exact-final-head hosted qualification. Do not merge either PR here.

## Complete per-task mandatory marker inventory

The following appendix is rendered from the immutable original contract; `×N`
denotes required multiplicity, not N separate task executions.
### Task 061

```text
UCI 2.5 TASK061 TIME INVENTORY AND IMPACT: PASSED
UCI 2.6 TASK061 TIME INVENTORY AND IMPACT: PASSED
UCI 2.5 TASK061 DLZ COMPILER CODEC VERTICAL: PASSED
UCI 2.6 TASK061 DLZ COMPILER CODEC VERTICAL: PASSED
TASK061 PINNED GATES: PASSED
```

### Task 071

```text
TASK071 REAL UCI ROUTES: PASSED
```

### Task 069

```text
TASK069 REAL UCI SEMANTIC DIFF: PASSED
```

### Task 062

```text
UCI 2.5 TASK062 ClosedSchemaSet INVENTORY COVERAGE IMPACT: PASSED
UCI 2.5 TASK062 OpenExtensions INVENTORY COVERAGE IMPACT: PASSED
UCI 2.6 TASK062 ClosedSchemaSet INVENTORY COVERAGE IMPACT: PASSED
UCI 2.6 TASK062 OpenExtensions INVENTORY COVERAGE IMPACT: PASSED
TASK062 ALL 54 NEWLY READY CLI TUPLES: PASSED
UCI 2.5 TASK062 RDMA ada COMPILER VERTICAL: PASSED
TASK062 REAL RDMA IPV6 CODEC: PASSED  [×2]
UCI 2.5 TASK062 RDMA rust COMPILER VERTICAL: PASSED
UCI 2.5 TASK062 RDMA cpp COMPILER VERTICAL: PASSED
UCI 2.6 TASK062 RDMA ada COMPILER VERTICAL: PASSED
UCI 2.6 TASK062 RDMA rust COMPILER VERTICAL: PASSED
UCI 2.6 TASK062 RDMA cpp COMPILER VERTICAL: PASSED
TASK062 PINNED GATES: PASSED
```

### Task 063

```text
UCI 2.5 TASK063 PINNED INVENTORY AND IMPACT: PASSED
UCI 2.6 TASK063 PINNED INVENTORY AND IMPACT: PASSED
UCI 2.5 TASK063 ada ORDERS METADATA VERTICAL: PASSED
UCI 2.5 TASK063 rust ORDERS METADATA VERTICAL: PASSED
UCI 2.5 TASK063 cpp ORDERS METADATA VERTICAL: PASSED
UCI 2.5 TASK063 SMALLEST SERVICE VERTICAL: PASSED
UCI 2.6 TASK063 ada ORDERS METADATA VERTICAL: PASSED
UCI 2.6 TASK063 rust ORDERS METADATA VERTICAL: PASSED
UCI 2.6 TASK063 cpp ORDERS METADATA VERTICAL: PASSED
UCI 2.6 TASK063 SMALLEST SERVICE VERTICAL: PASSED
TASK063 NEWLY READY SERVICE-CHECK: PASSED (9 world tuples; 6 release/backend/message tuples)
TASK063 PINNED GATES: PASSED
```

### Task 065

```text
UCI 2.5 TASK065 HIDING INVENTORY: PASSED
UCI 2.5 TASK065 ORIGINAL COLLISION ATTRIBUTION: PASSED
FORMER-GAP-GENERATE	2.5	Action	578	PASSED
FORMER-GAP-GENERATE	2.5	ActionCommand	560	PASSED
FORMER-GAP-GENERATE	2.5	ActionPlan	956	PASSED
FORMER-GAP-GENERATE	2.5	ActionPlanCommand	952	PASSED
FORMER-GAP-GENERATE	2.5	ApprovalPolicy	633	PASSED
FORMER-GAP-GENERATE	2.5	Authorization	230	PASSED
FORMER-GAP-GENERATE	2.5	AuthorizationRequest	234	PASSED
FORMER-GAP-GENERATE	2.5	CommSupport	581	PASSED
FORMER-GAP-GENERATE	2.5	CommSupportActivity	258	PASSED
FORMER-GAP-GENERATE	2.5	CommSupportPlan	962	PASSED
FORMER-GAP-GENERATE	2.5	CommSupportPlanCommand	960	PASSED
FORMER-GAP-GENERATE	2.5	Effect	578	PASSED
FORMER-GAP-GENERATE	2.5	EffectCommand	560	PASSED
FORMER-GAP-GENERATE	2.5	EffectPlan	956	PASSED
FORMER-GAP-GENERATE	2.5	EffectPlanCommand	952	PASSED
FORMER-GAP-GENERATE	2.5	MissionPlan	994	PASSED
FORMER-GAP-GENERATE	2.5	MissionPlanCommand	982	PASSED
FORMER-GAP-GENERATE	2.5	PlanningFunction	623	PASSED
FORMER-GAP-GENERATE	2.5	PlanningFunctionSettingsCommand	632	PASSED
FORMER-GAP-GENERATE	2.5	PlanningFunctionStatus	628	PASSED
FORMER-GAP-GENERATE	2.5	RequirementOptions	949	PASSED
FORMER-GAP-GENERATE	2.5	RequirementOptionsCommand	943	PASSED
FORMER-GAP-GENERATE	2.5	ResponseCommand	560	PASSED
FORMER-GAP-GENERATE	2.5	ResponsePlan	956	PASSED
FORMER-GAP-GENERATE	2.5	ResponsePlanCommand	952	PASSED
FORMER-GAP-GENERATE	2.5	SupportPlan	608	PASSED
FORMER-GAP-GENERATE	2.5	SupportPlanRequest	601	PASSED
FORMER-GAP-GENERATE	2.5	SupportRequest	592	PASSED
FORMER-GAP-GENERATE	2.5	SystemsNeededRequest	579	PASSED
FORMER-GAP-GENERATE	2.5	Task	1125	PASSED
FORMER-GAP-GENERATE	2.5	TaskCommand	563	PASSED
FORMER-GAP-GENERATE	2.5	TaskPlan	956	PASSED
FORMER-GAP-GENERATE	2.5	TaskPlanCommand	952	PASSED
FORMER-GAP-COMPILE	2.5	Authorization	230	PASSED
UCI 2.5 TASK065 AUTHORIZATION MODEL API GNAT: PASSED
UCI 2.5 TASK065 NAMING EVIDENCE: PASSED
UCI 2.6 TASK065 HIDING INVENTORY: PASSED
UCI 2.6 TASK065 ORIGINAL COLLISION ATTRIBUTION: PASSED
FORMER-GAP-GENERATE	2.6	Action	578	PASSED
FORMER-GAP-GENERATE	2.6	ActionCommand	561	PASSED
FORMER-GAP-GENERATE	2.6	ActionPlan	966	PASSED
FORMER-GAP-GENERATE	2.6	ActionPlanCommand	962	PASSED
FORMER-GAP-GENERATE	2.6	ApprovalPolicy	634	PASSED
FORMER-GAP-GENERATE	2.6	Authorization	231	PASSED
FORMER-GAP-GENERATE	2.6	AuthorizationRequest	235	PASSED
FORMER-GAP-GENERATE	2.6	CommSupport	581	PASSED
FORMER-GAP-GENERATE	2.6	CommSupportActivity	258	PASSED
FORMER-GAP-GENERATE	2.6	CommSupportPlan	972	PASSED
FORMER-GAP-GENERATE	2.6	CommSupportPlanCommand	970	PASSED
FORMER-GAP-GENERATE	2.6	Effect	578	PASSED
FORMER-GAP-GENERATE	2.6	EffectCommand	561	PASSED
FORMER-GAP-GENERATE	2.6	EffectPlan	966	PASSED
FORMER-GAP-GENERATE	2.6	EffectPlanCommand	962	PASSED
FORMER-GAP-GENERATE	2.6	MissionPlan	1004	PASSED
FORMER-GAP-GENERATE	2.6	MissionPlanCommand	992	PASSED
FORMER-GAP-GENERATE	2.6	PlanningFunction	623	PASSED
FORMER-GAP-GENERATE	2.6	PlanningFunctionSettingsCommand	632	PASSED
FORMER-GAP-GENERATE	2.6	PlanningFunctionStatus	628	PASSED
FORMER-GAP-GENERATE	2.6	RequirementOptions	960	PASSED
FORMER-GAP-GENERATE	2.6	RequirementOptionsCommand	953	PASSED
FORMER-GAP-GENERATE	2.6	ResponseCommand	561	PASSED
FORMER-GAP-GENERATE	2.6	ResponsePlan	966	PASSED
FORMER-GAP-GENERATE	2.6	ResponsePlanCommand	962	PASSED
FORMER-GAP-GENERATE	2.6	SupportPlan	608	PASSED
FORMER-GAP-GENERATE	2.6	SupportPlanRequest	601	PASSED
FORMER-GAP-GENERATE	2.6	SupportRequest	592	PASSED
FORMER-GAP-GENERATE	2.6	SystemsNeededRequest	579	PASSED
FORMER-GAP-GENERATE	2.6	Task	1126	PASSED
FORMER-GAP-GENERATE	2.6	TaskCommand	555	PASSED
FORMER-GAP-GENERATE	2.6	TaskPlan	966	PASSED
FORMER-GAP-GENERATE	2.6	TaskPlanCommand	962	PASSED
FORMER-GAP-COMPILE	2.6	Authorization	231	PASSED
UCI 2.6 TASK065 AUTHORIZATION MODEL API GNAT: PASSED
UCI 2.6 TASK065 NAMING EVIDENCE: PASSED
TASK065 PINNED GATES: PASSED
```

### Task 066

```text
UCI 2.5 TASK066 AUTHORITY INVENTORY: PASSED
UCI 2.6 TASK066 AUTHORITY INVENTORY: PASSED
UCI 2.5 TASK066 AO_CAPABILITY ada VERTICAL: PASSED
TASK066 REAL UNICODE JSON ROUNDTRIP: PASSED  [×2]
UCI 2.5 TASK066 AO_CAPABILITY rust VERTICAL: PASSED
UCI 2.5 TASK066 AO_CAPABILITY cpp VERTICAL: PASSED
UCI 2.6 TASK066 AO_CAPABILITY ada VERTICAL: PASSED
UCI 2.6 TASK066 AO_CAPABILITY rust VERTICAL: PASSED
UCI 2.6 TASK066 AO_CAPABILITY cpp VERTICAL: PASSED
TASK066 PINNED GATES: PASSED
```
