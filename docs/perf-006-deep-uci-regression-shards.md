# PERF-006: parallel Deep CI real-UCI regressions

## Disposition and source identity

**R1: HOSTED_CI_PENDING** — the source-consistency correction and current-main
integration require new hosted qualification. **Historical R0 qualification
completed successfully:** ordinary CI 16/16 and Deep CI 8/8. The sharded
regression critical path improved 65.7%, but the complete-workflow improvement
was only 6.6%: the original >=30% target was **not achieved**. R0 success does
not qualify the corrected R1 head.

Original R0 starting and incorporated `origin/main`:
`a849b87d94948980133038ba909c0b6528cadde6`, tree
`f3802d1b31e56a8afb6cc46db78b741dea4dde08`.
PR #109 is merged (merge commit
`86ffa50d38f4462a5b23a3714ec183cd79dc35df`) and is an ancestor of this main.
No existing PERF-006 branch or PR was found before creating
`feature/perf-006-deep-uci-regression-shards` in the independent worktree
`/home/zboll/git/ams-gra-codegen-oms-perf-006`.

PR: <https://github.com/zackboll/ams-gra-codegen-oms/pull/112> (open, non-draft,
auto-merge disabled). The final commit/tree, exact-head run IDs and latest Jobs
API snapshots are recorded in the PR handoff and external evidence directory
`/home/zboll/perf-006-evidence`. This avoids embedding a self-referential final
commit hash in its own tree. No other worktree, runner configuration, Alire
index, production source, Rust test, schema or fixture is changed.

## Topology and complete ownership inventory

Previously six independent jobs included one serial `real-uci-regressions`.
There are now eight jobs: five jobs with unchanged qualification behavior,
`real-uci`, `msrv-real-uci`,
`real-sleet`, `ada-repeated-choice-real-uci`, and `ada-hexbinary-real`, plus:

| Original gate | New mandatory owner | Preserved qualification |
| --- | --- | --- |
| Task061 newly READY Time | `real-uci-regressions` (A) | Production service-impact driver, batch-equivalence controls, per-selection evidence |
| Task052 Binary provenance | A | `uci_binary_provenance`, 2 passed, both inventory markers |
| Task053 constrained Binary | A | `uci_constrained_binary`, 3 passed, both inventory and impact markers |
| Task054 member identifiers | A | `uci_member_names`, 4 passed, both inventories/first blockers and real category-A GNAT compilation |
| Task056 generated-support parity | A | `uci_generated_support task056_real_uci`, 2 passed, all parity markers |
| Task064 generated-support binding | A | Listed binding test, exact execution, 1 passed, both binding markers |
| Task057 Duration | A | `uci_duration`, 4 passed, both inventory/impact markers, real category-A service |
| Task058 bounded ASCII | `real-uci-regressions-ascii` (B) | `uci_bounded_ascii_string`, 5 passed, both inventory/impact markers and newly-READY compiler/codec service |
| Task059 structured ASCII | B | `uci_structured_ascii`, 2 passed, both inventory/impact markers |
| Task060 alternating ASCII | `real-uci-regressions-alternating` (C) | `uci_alternating_admission` (2 passed) **and** `uci_alternating_after` (3 passed) together, all admission/projected-impact/compiler-codec markers |

Task060 still requires both releases' exact
`baseline=19 admitted=15 DEFERRED_UNICODE=3 DEFERRED_COMPLEX=1` markers and
all six projected-impact, OrderOfBattle and SMTI_SettingsCommand compiler-codec
markers. No partial gate substitutes for this combined campaign.

The original ten regression run bodies are unchanged byte-for-byte. All 17
inherited normalized qualification SHA-256 digests remain unchanged; only the
ownership entries for Task058/059/060 change. Existing test filters, counts,
`--exact`, `--test-threads 1`, compiler settings, assertions, nonzero-status
propagation, `set -euo pipefail` and here-string SIGPIPE protection are retained.
The Task061 artifact remains `task061-service-check`, at
`${{ runner.temp }}/task061-service-check-*/**`, with `if: always()` and
`if-no-files-found: error`.

Each shard independently checks out the immutable workflow event SHA, installs stable
Rust and verified working GNAT, fetches both pinned UCI releases using unchanged
revision/root-SHA-256-verifying scripts, exports both roots, fetches locked Cargo
dependencies and builds the exact-source release CLI. Each has private runner
temporary storage, `ubuntu-latest`, and the inherited finite 240-minute timeout.
There are no shard `needs`, caches, downloaded inputs, shared target directories,
optional gates or self-hosted runners. PERF-006 does not alter Fast CI; R1
inherits PR #111's new Python/Pages qualification through a normal main merge.

## Historical R0 local structural and adversarial qualification

Required commands:

```sh
bash scripts/check-ci-split.sh
bash scripts/test-check-ci-split.sh
python3 scripts/test-task077b-ci-layout.py
python3 scripts/check-task077b-ci-layout.py .github/workflows/deep-ci.yml
cargo fmt --all -- --check
git diff --check
```

The expanded layout self-test has **168 cases**, exercised through the real
shell guard. `test-check-ci-split.sh` passes **332 checks**. The Python unittest
suite exercises the mutations both in the actual layout and with an unrelated
preceding job containing overlapping setup commands; both tests pass. Positive
controls allow comments, step description changes and unrelated metadata.

Negative controls reject each missing shard; missing or duplicate gates;
renamed cross-job duplicates; wrong release ownership/root mapping; missing
GNAT, exact checkout, temporary storage, locked dependencies or release build;
debug rather than release CLI preparation; dependencies, skips, optional gates
or missing timeouts; missing Task061 batch controls/evidence; altered marker or
test-count assertions; and separated or incomplete Task060 tests. In particular,
admission present with AFTER removed is rejected. The original normalized
command digests are not regenerated and no whole-workflow hash is introduced.

YAML parsing confirms eight jobs. Comparison against starting main confirms
the five unrelated jobs are unchanged and every inherited digest is identical.
Raw local output is retained externally in `local-qualification.log`.
GitHub Actions-aware validation with actionlint 1.7.12 also passes (shellcheck
and pyflakes integrations disabled; the actual shell/guard suites run above).
Final corrective-head local output is in `local-qualification-final.log`.

The existing `test-task060-ci-wrappers.py` shell-extraction guard also includes
the new owners, retaining its complete inherited target inventory and actual
workflow-shell failure/marker controls. Its original two-owner assumption
would otherwise omit the moved campaigns rather than validate their shells.
This wrapper suite passes **96 checks**; the Task061 wrapper adversarial suite
also passes **4 checks**. Formatting and diff whitespace checks pass.

## Hosted baseline and timing method

Successful baseline: [40085862809](https://github.com/zackboll/ams-gra-codegen-oms/actions/runs/40085862809),
head `73b555843dff36c1364674b558241823cfb76375`.
Created `2026-10-24T10:25:38Z`; last job completed
`2026-10-24T12:04:58Z`. Created-to-last-job elapsed: **5,960 seconds (99m20s)**;
job start-to-last-completion envelope: **5,957 seconds (99m17s)**.
Original serial regression job: **5,956 seconds (99m16s)**.
Six job runtimes total **222.10 runner-minutes** (elapsed runtime, not billing).

Previous successful main run
[39943989498](https://github.com/zackboll/ams-gra-codegen-oms/actions/runs/39943989498)
has the same head and a **5,970-second (99m30s)** serial regression job.
Recent successful run identities and both baseline Jobs API responses are
preserved externally; cancelled runs are not performance observations.

## Completed R0 hosted qualification and measured limitations

Tested PR branch head: `43da9368db53c767ee5301b5292a0ac76dc0952e`;
tree: `23f2eede0aa81aae8548f5a5f34f0e5cfad4deda`.

- [Ordinary CI 40106583371](https://github.com/zackboll/ams-gra-codegen-oms/actions/runs/40106583371):
  **success, 16/16 jobs successful**.
- [Deep CI 40106583450](https://github.com/zackboll/ams-gra-codegen-oms/actions/runs/40106583450):
  **success, 8/8 jobs successful**. All ten regression campaigns executed,
  including both Task060 tests, with no compiler/test coverage reduction.

Both runs' API `head_sha` identifies the raw branch head, **not** the merge
actually checked out by default checkout steps. R0 logs show `real-uci`,
`msrv-real-uci` and `real-sleet` checked out synthetic PR merge
`2e6864fc123d5bc88c6af02fd593330ac88ae805`; the other five jobs checked out
`43da9368db53c767ee5301b5292a0ac76dc0952e`. Git commit API evidence confirms
both commits have the same tree `23f2eede0aa81aae8548f5a5f34f0e5cfad4deda`.
These are genuine successful historical results, but review correctly found
the mixed checkout semantics unsafe when the base advances.

All timestamps below are UTC on 2026-10-24, from the Jobs API.

| Deep CI job | Baseline runtime | R0 start | R0 complete | R0 runtime |
| --- | ---: | --- | --- | ---: |
| `real-uci` | 49m13s | 15:33:42 | 16:54:00 | 80m18s |
| `msrv-real-uci` | 18m56s | 15:29:09 | 15:54:01 | 24m52s |
| `real-sleet` | 28m18s | 15:41:22 | 15:54:52 | 13m30s |
| `ada-repeated-choice-real-uci` | 8m39s | 15:31:19 | 15:37:58 | 6m39s |
| `ada-hexbinary-real` | 17m44s | 15:54:46 | 16:30:22 | 35m36s |
| `real-uci-regressions` | 99m16s (all ten gates) | 15:42:14 | 16:16:18 | 34m04s |
| `real-uci-regressions-ascii` | Included above | 15:22:05 | 15:47:41 | 25m36s |
| `real-uci-regressions-alternating` | Included above | 15:36:53 | 16:07:05 | 30m12s |

The three regression jobs overlap from 15:42:14 through 15:47:41, demonstrating
real independent execution. R0 shard step times include Task061 3m42s,
Task052 22s, Task053 3m35s, Task054 5m45s, Task056 44s, Task064 21s,
Task057 18m43s; Task058 10m38s and Task059 9m29s; complete Task060 29m18s.
The two added shards' Rust/GNAT/fetch/release-preparation steps sum to 5m23s
(ASCII) and 48s (alternating), 6m11s combined. These are observed provisioning
steps, not a controlled causal estimate of incremental build cost.

| Measurement | Baseline | R0 PERF-006 | Observation |
| --- | ---: | ---: | --- |
| Regression critical path | 99m16s | 34m04s | 65.7% reduction |
| Complete Deep CI (workflow metadata) | 99m20s | 92m46s | 6.6% reduction; >=30% target **missed** |
| Summed hosted job execution | 222.10 min | 250.78 min | +28.68 runner-minutes, +12.9% |

R0 was created at 15:21:15Z, its last job completed at 16:54:00Z, and workflow
metadata was updated at 16:54:01Z. Thus creation-to-last-job is 92m45s;
creation-to-workflow-update is the reviewed **92m46s**. Baseline equivalent
timestamps differ by a second as well (99m20s to last job, 99m21s to update).
This one-second endpoint distinction does not change the 6.6% result. Do not
silently substitute elapsed endpoints. The 80m18s `real-uci` job was the
remaining critical path, with 12m27s of creation-to-job-start scheduling delay.

The unchanged `real-uci` rose from 49m13s to 80m18s. Source, hosted runner,
provisioning and scheduling variability prevent assigning the entire workflow
difference to sharding. Hardware/region parity is not established. This is a
single hosted comparison, not a statistically controlled performance campaign.
The cost is extra provisioning and increased summed runner execution; neither
exact CPU savings nor billing savings are established. Successful R0 runs are
preserved despite missing the whole-workflow target.

## Original pre-R0 workflow-load correction (preserved provenance)

Initial head `44bcfc5f070dd8054ec069920f20b3fd14aaeb50` produced workflow-load
failure [40105821171](https://github.com/zackboll/ams-gra-codegen-oms/actions/runs/40105821171)
before creating jobs: `runner.temp` cannot be referenced in job-level `env`.
The correction publishes `TMPDIR=$RUNNER_TEMP` through `$GITHUB_ENV` in each
shard's existing preparation step. actionlint verifies the correction, which
does not touch any qualification command or digest. This failed zero-job run
is not a timing observation. Initial ordinary CI run `40105847560` cannot
qualify the corrected final head regardless of its eventual conclusion.

## R1 source-consistency correction and current-main integration

Review: [5646908788](https://github.com/zackboll/ams-gra-codegen-oms/pull/112#pullrequestreview-5646908788).
Starting R1 head/tree are the R0 tested identities above. The dedicated
worktree was clean; local, remote and PR heads matched before changes.

Correction commit: `97642b73dd06b7d57db2c28d38c691407fcac711`, tree
`12e4cb9e47b39fc1fcea5c5e8e5a9b52c69f9ccf`.
Normal main-incorporation merge: `455c462236734585e6cb72d79891dc17d94dfd34`,
tree `88d5b603dfe77a9622ae5f987efd813fa62ea36e`.
Incorporated main: `a03684eb963e1eaa8a9df3db17c9d1783b7add03`, tree
`af76aa248a8dcef9cec23dc3b83ee5ce4956c4e1` (merged PR #111).
The merge was conflict-free, and main is an ancestor of the integrated head.
PR #111's Python/Pages CI and synthetic compiler tests are retained unchanged.

All eight jobs now start with exactly:

```yaml
- uses: actions/checkout@v4
  with:
    ref: ${{ github.sha }}
```

On PR events this is the proposed synthetic merge commit; on push, schedule and
manual dispatch it is the fixed event commit, never a moving branch name.
The immediately following mandatory Bash step executes before toolchain/setup
or qualification:

```bash
set -euo pipefail
actual="$(git rev-parse HEAD)"
tree="$(git rev-parse HEAD^{tree})"
test "$actual" = "$GITHUB_SHA"
test -n "$tree"
printf 'DEEP_CI_SOURCE_SHA=%s\n' "$actual"
printf 'DEEP_CI_SOURCE_TREE=%s\n' "$tree"
printf 'DEEP_CI_EVENT=%s\n' "$GITHUB_EVENT_NAME"
```

The topology guard binds the checkout and verification to the first two
executable steps in each required job, rejects duplicate checkouts/source
replacement and optional gates, and retains every inherited campaign digest.
The expanded suite retains all 168 R0 workflow cases and adds 144 job-local
source mutations (312 total). Negatives cover raw PR head, default checkout,
moving branch, `github.ref`, unrelated commit, missing/duplicate/later checkout,
missing/print-only/optional/skipped verification, absent tree assertion and
qualification before verification. Positive comment/description/metadata
controls still pass. Live disposable-repository tests execute the exact source
command for all four event types, SHA mismatch, absent Git repository, failed
tree resolution and empty tree output.

R1 changes no compiler algorithm, Rust test, schema, fixture, pinned input,
qualification command, marker, count, timeout or shard topology. All 17 digests
remain identical to R0. No whole-workflow digest is introduced.

### R1 final qualification status

**HOSTED_CI_PENDING.** The documentation/results update is a separate commit.
Final raw PR head/tree and its corresponding synthetic merge SHA/tree are
recorded in the PR handoff and external R1 evidence directory, avoiding a
self-referential final hash. Every hosted job must report the same merge SHA
and tree, matching the workflow event, before R1 can be declared qualified.
Run metadata's branch `head_sha` alone is not proof of the checkout.

R1 local validation and hosted results are recorded separately from R0 under
`/home/zboll/perf-006-evidence/r1-20261026`; earlier raw evidence is not replaced.
The final integrated local run includes all requested split/layout/wrapper
checks, formatting, actionlint and PR #111's full
`python3 -m unittest discover -s scripts -p 'test_*.py'` gate. A local `strace`
package is extracted outside the repository for that inherited gate because
system `strace` is absent and passwordless sudo is unavailable; no compiler
evidence is mocked or skipped by this environment preparation.

Integrated local results: split guard **PASS**; shell mutation suite **476
checks PASS**; layout unittest suite **3 tests PASS**, including 312 workflow
cases for each of the actual/overlapping layouts and live source-gate failure
tests; standalone layout check **PASS**; Task060 wrappers **96 checks PASS**;
Task061 wrappers **4 checks PASS**; inherited Python synthetic/compiler/Pages
discovery **46 tests PASS, no skips**; inherited compiler-placement suite
**11 tests PASS**; Rust formatting, diff whitespace and actionlint 1.7.12
expression/schema validation **PASS** (shellcheck/pyflakes integrations disabled).
Preservation checks confirm all eight jobs' non-checkout/non-identity steps
match R0 exactly, all 17 gate entries are identical, main's CI and synthetic
test files are unchanged, and production/test/fixture/schema/pinned-fetch
paths have no diff from main.

Only new final-head ordinary CI and all eight Deep CI successes, reconciled
source identities and recorded measurements permit `READY_FOR_INDEPENDENT_REVIEW`.
Historical R0 success is not R1 qualification and does not mean READY_TO_MERGE.
The missed 30% complete-workflow target remains explicit; no new research or
independent performance campaign is required. Keep PR #112 open, non-draft,
unmerged and without auto-merge; leave reviewer thread resolution to re-review.