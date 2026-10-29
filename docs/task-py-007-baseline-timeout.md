# PY-007 baseline execution budget correction

The integrated source at `72fac379f1727c30599458cfb6bf929db496b99f`
(tree `3cd5f551590f6ea68a18d7f6c450cc3ad2607e3f`) used a ten-minute
baseline job limit. The baseline runs the mandatory policy, selector, evidence,
split/layout/adversarial controls and pinned actionlint, not Python generation.

## Measured hosted evidence

Times below are REST job/step execution timestamps, not queue time.

| CI run | Source | Baseline job | Policy step | Result |
| --- | --- | --- | --- | --- |
| [40618705997](https://github.com/zackboll/ams-gra-codegen-oms/actions/runs/40618705997) | main `d50bf7e` | 02:06:18–02:13:26 UTC, 428 s | 02:06:20–02:13:24, 424 s | success |
| [40626798700](https://github.com/zackboll/ams-gra-codegen-oms/actions/runs/40626798700) | main `ac04659` | 04:38:14–04:47:01 UTC, 527 s | 04:38:17–04:46:58, 521 s | success |
| [40638477993](https://github.com/zackboll/ams-gra-codegen-oms/actions/runs/40638477993/job/122650906752) | PR117 `72fac37` | 05:32:26–05:42:43 UTC, 617 s | 05:32:30–05:42:41, 611 s | timeout/cancelled |

All timestamps are on 2026-10-28. The PR job's failure annotation explicitly
states **"The job has exceeded the maximum execution time of 10m0s"**.
The recorded interval includes cancellation/cleanup latency beyond that limit.
The workflow continued other jobs; available evidence does not indicate a
separate workflow cancellation or runner failure as a better explanation.

Before interruption, 51 selector tests, 18 evidence tests, the split guard and
three layout tests passed. The nested layout suite reported 362.428 seconds;
the remaining adversarial suite was interrupted before its final completion
marker, and actionlint was consequently skipped. This incomplete run does not
qualify the source and is not evidence of a model, codec or inheritance defect.

## Narrow policy change

Only `ci.yml`'s baseline `timeout-minutes` changes from **10 to 20**. The matching
`orchestration_payloads.baseline` in the qualification inventory records exactly
that same change. Twenty minutes provides a bounded execution margin over the
observed 8m47s main run and interrupted PR execution without removing work.
It is an execution budget, not a performance claim or a relaxed success gate.

All mandatory commands, concurrency identities, five main lanes, selection,
Python versions, Deep shards, source/artifact provenance, security contracts
and aggregate dependencies remain unchanged. No other job budget changes.
The offline `test_ci_select_baseline.py` controls require exactly 20 minutes,
reject unauthorized timeout changes, and reject removal or masking of every
mandatory run line. The existing 544 split adversarial checks remain intact.

Acceptance requires a new exact-head run: successful baseline completion
within 20 minutes, every selected ordinary/Deep cell, source-bound shard
evidence, CI002 and `pr-ci-required`. The old run cannot authorize merging.
If the corrected baseline times out, record its runtime and incomplete
operation and stop; do not increase the limit again without new evidence.