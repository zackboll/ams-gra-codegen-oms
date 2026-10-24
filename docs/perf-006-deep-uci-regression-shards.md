# PERF-006: parallel Deep CI real-UCI regressions

## Disposition and source identity

**HOSTED_CI_PENDING** — implementation and local structural qualification are
complete; exact-head hosted integration and timing qualification must finish
before independent-review readiness or performance success can be claimed.

Starting and incorporated `origin/main`:
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
There are now eight jobs: the five unchanged jobs `real-uci`, `msrv-real-uci`,
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

Each shard independently checks out the PR head (or event SHA), installs stable
Rust and verified working GNAT, fetches both pinned UCI releases using unchanged
revision/root-SHA-256-verifying scripts, exports both roots, fetches locked Cargo
dependencies and builds the exact-source release CLI. Each has private runner
temporary storage, `ubuntu-latest`, and the inherited finite 240-minute timeout.
There are no shard `needs`, caches, downloaded inputs, shared target directories,
optional gates or self-hosted runners. Fast CI is unchanged.

## Local structural and adversarial qualification

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

| Deep CI job | Baseline runtime | Optimized exact-head runtime |
| --- | ---: | --- |
| `real-uci` | 49m13s | Pending hosted completion |
| `msrv-real-uci` | 18m56s | Pending hosted completion |
| `real-sleet` | 28m18s | Pending hosted completion |
| `ada-repeated-choice-real-uci` | 8m39s | Pending hosted completion |
| `ada-hexbinary-real` | 17m44s | Pending hosted completion |
| `real-uci-regressions` | 99m16s (all ten gates) | Pending hosted completion |
| `real-uci-regressions-ascii` | Included above | Pending hosted completion |
| `real-uci-regressions-alternating` | Included above | Pending hosted completion |

Use exact-head workflow and Jobs API records for start/completion timestamps,
each shard's step durations, observable scheduling delay, all eight runtimes and
total runner-minutes. Compare created-to-last-completion consistently with the
baseline; separately report the job envelope and workflow metadata timestamps.
GitHub PR workflows may identify a synthetic merge SHA: record that SHA and its
PR head association, rather than confusing it with the explicitly checked-out
source head. Ordinary CI must also pass for that same PR head.

Sharding adds two independent provisioning/release-build sequences. Report
their measured step time and the total runner-minute difference when complete;
neither CPU savings nor exact billing savings can be inferred. The expected
remaining critical path is the unchanged `real-uci` (baseline 49m13s), not a
guaranteed result. The >=30% whole-workflow reduction target is **not yet
measured**. A single successful hosted comparison is observational, not a
controlled benchmark; runner hardware/region and scheduling may differ.

## Remaining qualification gaps

Initial head `44bcfc5f070dd8054ec069920f20b3fd14aaeb50` produced workflow-load
failure [40105821171](https://github.com/zackboll/ams-gra-codegen-oms/actions/runs/40105821171)
before creating jobs: `runner.temp` cannot be referenced in job-level `env`.
The correction publishes `TMPDIR=$RUNNER_TEMP` through `$GITHUB_ENV` in each
shard's existing preparation step. actionlint verifies the correction, which
does not touch any qualification command or digest. This failed zero-job run
is not a timing observation. Initial ordinary CI run `40105847560` cannot
qualify the corrected final head regardless of its eventual conclusion.

Until all eight Deep CI jobs and ordinary CI finish successfully for the final
PR head, hosted timing, added provisioning cost, actual remaining critical path
and the performance objective remain unqualified. Keep the PR open, non-draft,
unmerged and without auto-merge. If any mandatory shard fails or provenance
changes unexpectedly, use `CORRECTIVE_ACTION_REQUIRED`; only complete passing
qualification and recorded timing permit `READY_FOR_INDEPENDENT_REVIEW`.