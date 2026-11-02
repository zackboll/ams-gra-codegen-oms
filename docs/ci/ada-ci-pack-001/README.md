# ADA-CI-PACK-001

Incorporated main: `afe0c510866c08fc188fef335e74879840794a70`, tree
`a384c4375098fb5293c0a4fa11595f5de7bbe014`.

## Bounded pilot (implementation gate not yet evaluated)

The temporary branch revision of the already registered
`portable-provider-benchmark.yml` dispatches **one** Ubuntu 24.04 pilot job,
using existing shared slot 3. It cannot execute from main. This file will be
restored after the pilot; main CI and Deep CI are unchanged at this stage.
No dispatch of the full CI matrix is required.

The pilot configuration is the production candidate: four workers; each with
one allowed CPU selected via `sched_getaffinity`, `CARGO_BUILD_JOBS=1`, and
Alire build forwarding with explicit GPRbuild `-j1`. The generator is built
once, with four build jobs, before the full original guard and its negative
wrapper controls. The guard provisions and validates GNAT 16.1.0, GPRbuild
26.0.1 (executable 26.0.0), and pinned GNATCOLL dependencies. Every worker gets
its own settings, cache (including compiler copies), temporary directory,
scratch outputs, original logs, command/exit records, and completion identity.
There is no concurrently writable shared Alire/compiler cache.

The frozen `scripts/ada_ci_pack/inventory.json` preserves every original suite
command, assertion step, artifact pattern, and qualification source hash.
All original qualification scripts/consumers remain unchanged. Independent
completion checks are additional, not replacements for the script checks.
A missing artifact pattern, failed process, missing marker, timeout, or
termination makes its worker and therefore the pack fail. Other workers are
collected regardless of a peer failure.

## Historical baseline

`baseline.json` contains individual GitHub job endpoint records, including
created/start/completion times and step times, for the requested three runs
and successful comparable run 41332273331. Pending jobs have **no** runner
occupancy or queue duration assigned: their API `started_at` may just equal
creation and is not evidence of runner allocation.

Use the four successful jobs from run 41332273331 as the coherent initial
separate-job baseline. Its source is
`bf6228ddee8b9e9fc70704c2fc68a409cdc37729`; all qualification scripts used by
Pack A and the guard are byte-identical to incorporated main (verified with
`git diff`). Newer individual successes are retained for context, not selected
opportunistically. Queue delays are reported separately and never included
in occupied runner-minute calculations. Packed job occupancy must include
checkout, Rust/Alire setup, generator build, guard, workers, and artifact upload
using the final hosted job record, not merely the worker wall clock.

Current branch protection requires only `PR smoke`; repository rulesets are
empty. No maintainer-controlled required-status-check rename is needed.

## Local validation

- `python3 -m unittest discover -s scripts -p test_ada_ci_pack.py -v`
- `python3 -m unittest discover -s scripts -p test_pr_smoke.py -v`
- `python3 -m unittest discover -s scripts -p test_main_lanes.py -v`
- `python3 scripts/test-task093-compiler-placement.py`
- `ACTIONLINT=/absolute/path/to/actionlint bash scripts/check-ci-actionlint.sh`
- `git diff --check`

No performance result is claimed until the hosted pilot and its artifacts
have been independently verified.
