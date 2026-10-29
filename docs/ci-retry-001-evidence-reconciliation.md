# CI-RETRY-001 — exact producer attempts for partial Actions reruns

Corrective scope: final-gate evidence reconciliation only, based on main
`3f69cb0e5b0801cf01b5cbde9018bb7a3b43fb9b`
(tree `13f9261ec176572d09f47f9392eeaf51741a7bc7`). No workflow, inventory,
qualification payload, result publication, compiler placement, concurrency lane,
secret, permission, CI002 policy, PERF-009 certificate or benchmark rollout is
changed. PR #124 and PR #121 branches/worktrees are not modified by this PR.

## Retained failure and independent reproduction

Reference: PR #124, CI run `40810885664`, attempt 2.

- Workflow head: `b59dbf2a3cf8b8154b71e8082f59decedf8a7f76`.
- Proposed merge: `05f7a5b6bfc3a5a69b14f3889c73babe7e4311e4`.
- Proposed tree: `340c064fa93c05daf8ebc2e19bfb82c761499cae`.
- Base: `7958c4e0d8f78768fd4c7d4c1bd34877c31d893a`.
- Attempt-1 `ada-lacal-runtime-core (16)` job `123243677185` failed on APT
  downloads (`Unable to connect to deb.debian.org:80`). Its attempt-2 execution,
  job `123265279073`, succeeded. All 34 qualification jobs in attempt 2 were
  successful; `pr-ci-required`, job `123268291588`, failed at
  `stale/wrong Task100 source/run/attempt/compiler identity`.
- All four Task100 and three PERF-007 bundles were actually published by
  successful attempt-1 executions. Their embedded attempt is exactly `1`.
  Passing final-gate attempt `2` to either existing bundle verifier reproduces
  the stale-attempt failure.

GitHub's attempt-2 jobs endpoint **copies successful attempt-1 executions**:
`ada-websocket-transport (14)` has attempt-1 job ID `123243677260`, then appears
as job `123265309995` with `run_attempt: 2`, but with the **same** original
`03:36:21Z`–`03:43:58Z` timestamps. Attempt 2 did not start until `04:34:46Z`.
The artifact's `03:43:51Z` creation lies in the original successful upload
step. Therefore neither latest-job `run_attempt` nor artifact self-reporting
is sufficient provenance.

The retained API snapshots, both failure logs, all seven ZIPs, public artifact
SHA-256 metadata and original selection manifest are stored outside the repo at
`/home/zboll/git/ams-gra-codegen-oms-ci-retry-001-evidence/`.
`replay-ci-retry-001.py` verifies all ZIP digests and exact merge parents, then
reproduces all seven original attempt failures and verifies the complete
bundles using independently resolved producer attempts. PERF-007 source files
are read from Git objects for the original proposed merge: today's source
hashes are **not** substituted and hash checks are not disabled.

```bash
python3 /home/zboll/git/ams-gra-codegen-oms-ci-retry-001/scripts/replay-ci-retry-001.py \
  --evidence /home/zboll/git/ams-gra-codegen-oms-ci-retry-001-evidence
```

The retrospective replay is not a hosted final-gate success and does not
rewrite the outcome of the failed historical run.

## Corrected fail-closed contract

1. Keep the current workflow run/attempt API check and exact checkout/proposed
   merge verification. Additionally require the final gate's own job to be
   genuinely in progress in the **current** attempt, with matching run/head and
   a start no earlier than that attempt's start.
2. Fetch complete paginated jobs and run metadata for every attempt through
   the current attempt. Missing/inconsistent history fails. A defensive maximum
   of 100 attempts bounds API work; it is not an old-attempt fallback.
3. Require unique, nonexpired, nonempty named artifacts from the exact workflow
   run/head/repository and head repository, with a published SHA-256 digest.
4. Identify the actual historical execution using the exact job name, source,
   complete step metadata, start/end/status/conclusion and matching latest
   execution. Ignore copied historical executions whose timestamps precede
   their purported attempt. Run/event/workflow/check-suite/repository and PR
   base/head provenance must agree across attempt snapshots.
5. Artifact creation must lie within both that successful job and its exact
   mandatory successful **publication step**. Do not infer ownership merely
   from a nearby job or a broad run lifetime. The producer must be unique.
6. Pass only the resolved producer attempt to the unchanged strict identity
   semantics of Task100 and PERF-007 validators. Embedded attempts, compiler,
   shard, source commit/tree, run and every mandatory result must still agree.
7. Verify downloaded ZIP bytes against GitHub's SHA-256 digest before parsing.
   Existing bounded data-only ZIP reading, path/duplicate/CRC checks and complete
   Task100/PERF-007 result/source/hash validation remain mandatory.
8. Publish `ci-required-result.json` and its success marker **only after all
   checks pass**, retaining the workflow's required-result artifact upload.
   The report adds the current gate attempt and each evidence producer's
   original job ID, attempt, artifact ID/digest and creation timestamp.

An earlier artifact is not accepted if a new execution superseded its producing
job, even when the old job succeeded. Full reruns require new evidence for new
executions. Duplicate names are never resolved by choosing “latest.” There is
no retry-until-accepted loop or special acceptance for a known PR/run number.

## Regression and hosted qualification

The existing mandatory `test_ci_select*.py` discovery runs new controls without
changing the baseline workflow or its pins. Coverage includes normal one-attempt
CI, failed-only reruns, complete reruns, mixed producer attempts, copied IDs,
superseded evidence, forged embedded attempts, altered source/base/head/run/job
metadata, failed/skipped steps, missing/duplicate/expired artifacts, wrong
publication times, missing/corrupt Task100/PERF-007 files, incorrect
compiler/shard identities, ZIP digest mismatch, incomplete pagination and a
stale/copied final gate. Failure-injection tests prove final-result success is
not written before provenance and both bundle families pass.

Local CI policy, selector/final-gate mutations, CI002, real-UCI evidence tests,
split controls, pinned actionlint and diff checks must pass. Exact corrective
PR head/tree and hosted outcomes are reported separately; queued/pending jobs
are not successful qualification. Existing automatic inventory remains 40
runner cells and five main lanes.

## Handoff after independent review

Leave the corrective PR non-draft, unmerged and without auto-merge. Do not
approve, merge or force-push it. **Only after independent review and merge**,
reconcile PR #124 with the then-current main using an ordinary merge in its
own worktree, then requalify its exact new head and proposed merge, including
the mandatory final result artifact. That follow-up is deliberately not done
in this corrective delivery. PR #121's correctness certificate and production
portable behavior remain unchanged.