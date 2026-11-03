# ADA-CI-PACK-001

## Owner-directed implementation (2026-11-02)

The repository owner explicitly authorized implementing the two packed GNAT 16
qualification jobs **without waiting for the queued performance pilot**.
The earlier 15% runner-minute threshold is no longer a prerequisite. Four
independent suites run in pack A (four workers on one 4-vCPU hosted runner);
three run in pack B (three workers on another runner). Each pack runs the full
original JSON guard, builds the generator once, uses independently isolated
Alire/compiler workspaces, verifies every original success marker, and uploads
both its own receipts and the original suite artifacts. Both packs are
mandatory dependencies of `main-ci-required`.

This replaces eight independent runner jobs with two, freeing six *potential*
runner allocations for future main workflow runs. It is a structural runner-slot
reduction, **not yet measured wall-clock or runner-minute savings**. The
immutable existing pilot remains a separate historical experiment; no new
benchmark was dispatched or treated as a gate. Exact-head hosted smoke and
subsequent main qualification must still prove the new workflow valid.

Existing main jobs already in GitHub's backlog retain their original workflow
graphs; this change applies to new runs after integration. No existing main
runs were canceled by this implementation.


Incorporated main: `afe0c510866c08fc188fef335e74879840794a70`, tree
`a384c4375098fb5293c0a4fa11595f5de7bbe014`.

## R1 independent review (2026-11-03)

Reviewed owner-authorized implementation `aa6372f38d34f52a26e015d598b1656d83be5c9b`
(tree `f8d4ff2aed6d66c780855cadddf83c04c2d17257`). The eight old jobs are absent;
exactly `ada-codec-pack-a` and `ada-codec-pack-b` replace them. All other jobs
are unchanged except `main-ci-required`, which requires both packs and fails
on any non-successful dependency. The existing ten-slot policy is unchanged.

Found and corrected a real integration defect: the mandatory compiler-placement
regression still looked for the removed `ada-hexbinary-codec` workflow block,
so pack A would fail. It now follows pack A into the frozen command inventory,
retaining the four-invocation / 120-execution compiler/profile/policy matrix.
Its negative controls mutate the real inventory instead of an absent workflow
command, and reject missing execution, verification, suite membership, changed
compiler placement, narrowed policies/profiles, and conditional/error bypasses.
No Ada production code, corpus, qualification command, or expected result changed.

The independent artifact comparison confirms every original upload glob/name
is retained with a task-private prefix; only the duplicated guard name changes
to `task087a-ada-json-guard-pack-a` / `task087a-ada-json-guard-pack-b`. There are
no active workflow artifact consumers requiring an update. Lane regressions
now also check every original artifact glob, not just artifact names.

Initial R1 checks: 19 pack, two lane, 34 routing, 13 compiler-placement, and 11
PERF-005 CI regressions pass; pinned actionlint 1.7.12, Rust 1.95.0 formatting,
locked CLI/Ada all-target checks, and diff checks pass. PR smoke and trusted
signals passed on the reviewed head; corrective-head checks must pass separately.
Live local pack qualification evidence is retained outside the checkout under
`/tmp/ada-ci-pack-001r1-evidence/` (logs) and
`/home/zboll/ada-ci-pack-001r1-evidence/` (pack scratch). It is correctness
qualification, not a benchmark or a substitute for mandatory hosted main jobs.

Isolation review: Alire settings, caches, toolchains, dependency builds, TMPDIR,
and generated consumers are private copies; the generator is shared read-only
after its one build. Each worker inherits one allowed CPU and inner GPRbuild
`-j1`; Cargo is not invoked concurrently by workers. The full guard precedes
every pack's workers. Every worker must have successful step exits, artifacts,
markers and matching source identity; timeouts/cancellation terminate its
process group and cannot become success. Resource evidence is mandatory.

Operational risks remain: hosted 4-vCPU/16-GB packs have not yet completed as
main jobs, toolchain copies consume disk, and serialized inner builds may take
longer or hit the retained original suite timeouts. No performance savings are
claimed. The optional historical pilot is not an acceptance gate; do not dispatch
another benchmark or wait for it. Existing queued main runs keep their old graphs.

### Completed R1 live qualification

Both full packs **and both independent verifiers passed** on corrective commit
`40a5281d46cbea5fb9e2abf9be0147c936ca844f`, tree
`dfb8e67b1da0b73e77c2778699b3ac3fb9ede387`. This includes all seven codec
workers, two complete guards, every original profile/assertion policy, and all
30 portable helper-demand cells. Every pack/worker/step exit was zero; both
verifier exit files contain zero. The additional Ada-backend library run passed
all 64 tests, none ignored. The complete summaries, constituent exits, marker
hashes, source identities and resource-sample hashes are retained in
`local-pack-completions.json`; full logs and scratch remain at the paths above.
Local timings are not hosted performance evidence, and host-wide memory/disk
measurements must not be presented as isolated pack resource requirements.

After qualification, main advanced to
`27e9c3d3d656e3724490f0743f9ab93f9060bae1` with Rust-client reconnect changes.
Strict branch protection requires an up-to-date branch, so main was incorporated
by ordinary merge `ba742cbf102af7b21b3a6a0c6853dd92c6cadef1`. That merge changes
no generator crate, root lockfile, Ada source, workflow or frozen qualification
input. Full live pack evidence remains explicitly bound to the corrective head,
not relabeled as a final-head execution. Final-head focused checks and hosted
smoke/signals are requalified separately. No amend/rebase/reset/force-push used.

## Historical pilot handoff (superseded; not an acceptance gate)

The following records describe the pre-authorization pilot only. Its former
performance gate and continuation instructions no longer apply.

PR: https://github.com/zackboll/ams-gra-codegen-oms/pull/181

The bounded pilot is queued at
https://github.com/zackboll/ams-gra-codegen-oms/actions/runs/41547959433,
commit `5a40e9cbe9acd5d25917210998318f4e03a5a8d0`, tree
`8eba11c38db5f3656afb43272804cbe797254732`.
It uses the temporary branch revision of the already registered
`portable-provider-benchmark.yml`: **one** Ubuntu 24.04 pilot job, existing
shared slot 3, no main matrix dispatch. The deliverable HEAD restores that
unrelated workflow byte-for-byte; GitHub retains the dispatched immutable
pilot revision. Main CI, Deep CI, PR smoke and main dependencies are unchanged.

Two superseded dispatches were cancelled **before** runner allocation
(`runner_id=0`, no steps); their individual job records are committed alongside
this report. No hosted performance benchmark has executed and no fallback has
been used. They were superseded for a locally discovered Alire CLI syntax fix
and stronger independent evidence checks, not to select favorable timings.
The queued pilot still has the entire permitted benchmark budget available.

At 2026-11-02 23:45 UTC the existing slot-3 owner was main run 41371432996,
job 125042547873 (`deep-qualification / real-uci-schema-ipv6`), started at
23:43:48 UTC with a configured 240-minute timeout. This queue is not bypassed.
There is no automatic/background agent promised to implement after completion.
The next action is to inspect this one pilot, evaluate the gate, then implement
both mandatory packs immediately in **this same PR**, without another approval
or research task. Do not merge the pending pilot-only PR as the optimization.

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
`git diff`). Production CLI/core changes since that source add C++/Python
codec behavior, so the generator revision is not identical. Ada backend,
qualification scripts and consumers are unchanged. Newer individual successes
are retained for context, not selected
opportunistically. Queue delays are reported separately and never included
in occupied runner-minute calculations. Packed job occupancy must include
checkout, Rust/Alire setup, generator build, guard, workers, and artifact upload
using the final hosted job record, not merely the worker wall clock.

| Suite | Baseline occupied | Scheduling queue (excluded) |
| --- | ---: | ---: |
| Generated codec | 558 s (9m18s) | 47,021 s |
| Repeated temporal | 872 s (14m32s) | 41,248 s |
| DateTime | 446 s (7m26s) | 57,109 s |
| hexBinary | 1,460 s (24m20s) | 39,785 s |
| **Total occupied** | **3,336 s (55m36s)** | **not runner time** |

Required packed occupancy: at most 2,835.6 seconds for 15% reduction and at
most 2,920 seconds for twice the longest individual baseline. Use
`1 - packed_occupied_seconds / 3336`; no result is available yet. Packed elapsed,
setup/build, worker times, CPU utilization, peak memory/disk, runner-minute
savings and resource-contention assessment are **pending**, not zero.

## Job/artifact inventory

Replaced: `ada-json-guard` plus `ada-generated-codec`,
`ada-repeated-temporal-codec`, `ada-datetime-codec`, `ada-hexbinary-codec`,
`ada-repeated-codec`, `ada-duration-codec`, `ada-repeated-choice-codec`: eight
scheduled runner jobs. Implemented: `ada-codec-pack-a`
(first four suites), `ada-codec-pack-b` (last three), each with its own complete
JSON guard prerequisite: **eight to two scheduled runner jobs**.
`main-ci-required` requires both and no longer references the superseded jobs.

The frozen inventory retains all eight original artifact specifications.
The pilot preserves the four constituent original artifact names plus
`task087a-ada-json-guard`; it adds `ada-ci-pack-001-pilot` for resource samples,
source identities, step exits, and worker completions. In two production jobs
the guard artifacts must become pack-specific to avoid upload-artifact v4 name
collisions; those names are implemented and the consumer search is complete.

No original test/assertion or script has been removed or changed. The inventory
contains the exact original shell steps and 271 qualification input hashes;
the regression suite compares them to incorporated main. Each suite retains
all original assertion policies, profiles, corpus counts, source checks and
negative tests. The independent verifier re-reads logs, recorded exits,
artifact patterns, JSON-guard frozen output hashes and compiler trace bounds.

Current branch protection requires only `PR smoke`; repository rulesets are
empty. No maintainer-controlled required-status-check rename is needed.

## Local validation

- `python3 -m unittest discover -s scripts -p test_ada_ci_pack.py -v`
- `python3 -m unittest discover -s scripts -p test_pr_smoke.py -v`
- `python3 -m unittest discover -s scripts -p test_main_lanes.py -v`
- `python3 scripts/test-task093-compiler-placement.py`
- `ACTIONLINT=/absolute/path/to/actionlint bash scripts/check-ci-actionlint.sh`
- `git diff --check`

Results so far: 19 pack regressions (including real Alire private-settings
execution), 34 PR routing regressions, one ten-slot regression, 12 compiler
placement regressions and 11 PERF-005 regressions pass. Actionlint 1.7.12,
Rust 1.95.0 formatting, locked CLI/Ada all-target checks and all 64 Ada-backend
library tests pass (zero ignored). Live original JSON guard passes GNAT 16.1.0
normal/Check/Ignore with all frozen 199-case/four-boundary hashes, the clean
consumer, eight negative wrapper controls and independent evidence checks.
Local serial codec qualification is additional correctness feedback only, not
a hosted benchmark. Raw local logs reside under
`/tmp/ada-ci-pack-001-evidence/` on the task machine.

Known pre-existing limitation: `scripts/test-task077b-ci-layout.py` fails its
two healthy-layout subcases with `Task077B mandatory source identity:
real-sleet` and `Task077B fetch ownership: real-sleet 2.6`. Both failures were
reproduced using archived incorporated-main workflow/test/validator bytes.
The out-of-scope Deep CI files are unchanged; no assertion was removed to make
this test pass.

PR smoke and trusted signal passed on commit
`46156b704d5ee3ab484bb4e5b44c5f96e13f7136`; later exact-head checks must be
reported separately. Auto-merge is disabled; no approval or merge performed.

No performance result is claimed until the hosted pilot and its artifacts
have been independently verified.
