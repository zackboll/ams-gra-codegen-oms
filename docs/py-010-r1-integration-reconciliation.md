# PY-010-R1 — PR #132 integration reconciliation

## Source authority and scope

Existing PR: <https://github.com/zackboll/ams-gra-codegen-oms/pull/132>.
The clean, existing isolated worktree is
`/home/zboll/git/ams-gra-codegen-oms-py-010`, on
`feature/py-010-visible-ascii-strings`.

| Identity | Git object |
| --- | --- |
| Original PR baseline | `a44ce80f278c0a64c604a1b800679e02f8687cdf` |
| Pre-merge PR HEAD | `927be1e4ce6c0c81880264df2eb74fd27b67747a` |
| Pre-merge PR tree | `8613cc7f12009d43c51ad1789961f66d6622b0c6` |
| Incorporated main | `e4c73249d836114bc7585181b9557be6ae113011` |
| Incorporated main tree | `48d18aa1ba7bff889eebcea115518ecddeaa88e9` |

Fresh fetch, `ls-remote`, and GitHub API checks agreed on both branch heads.
There were no dirty files, Git operation locks, or concurrent PY-010 work.
The integration uses a normal merge, preserving the original feature commit;
no rebase, amend, force push, replacement PR, approval, or PR merge is allowed.
Other worktrees, PR #133, and its completed acceptance evidence are untouched.

## Semantic conflict resolutions

There were four textual conflicts:

- `/home/zboll/git/ams-gra-codegen-oms-py-010/.github/workflows/ci.yml`:
  retain CI-SELECT-003's normal/optimized fixed owned runner and negative
  controls immediately after PY-009, then retain the complete PY-010 Fast
  generated-code, gate-control, and historical-parity commands.
- `/home/zboll/git/ams-gra-codegen-oms-py-010/.github/workflows/deep-ci.yml`:
  keep the explicit PY-010 successor service baseline, CI-SELECT-003's UCI
  runner, and PY-010's profile example build and Python 3.11/3.13 loop.
  Keep Task101's independently owned job unchanged.
- `/home/zboll/git/ams-gra-codegen-oms-py-010/scripts/ci_policy.py`:
  project verified PY-010 additions first, CI-SELECT-003 additions second,
  and PY-009 additions third before the original immutable payload checks.
  No historical payload or source-security hash is recomputed from this
  candidate. Main's Task101 and four Python ownership contracts remain intact.
- `/home/zboll/git/ams-gra-codegen-oms-py-010/scripts/run-real-sleet-test.sh`:
  execute PY-010 after PY-009 and before both owned Sleet runner calls in the
  existing interpreter loop. Retain one pinned broker checkout/build.

The inventory merged textually without conflict; its only additive difference
from incorporated main is the mandatory `py010-sleet` artifact on `real-sleet`.
Semantic follow-ups repair successor projection tests and the obsolete
40-runner assertion. Split guards now validate the supplied workflows' additive
projection chain, rather than only checking the worktree's own workflows.

Each projection verifies its own mandatory addition exactly once in its owning
job. The PY-010 Sleet projection verifies its adjacency to PY-009; the next
projection independently verifies the owned hooks and loop end. Consequently
missing owned hooks are attributed to CI-SELECT-003, not an earlier unrelated
PY-010 error. The remaining script must still match its historical hash.

Mutation tests cover removal, disabling, filtering, duplication and relocation
of additions; incorrect Sleet artifact paths/ownership; Task101 compiler and
artifact removal; 40-runner/12-Deep rollback; final-gate removal/disablement;
and infrastructure incorrectly treated as Python-only. Loop-line mutations
target the complete PY-010 block, not an unrelated earlier `done`.
PY-010's workflow projections also require the established following Fast/UCI
section, interpreter-setup location, and final owning Sleet upload location;
moving a valid command/upload elsewhere within the same job is rejected.

## Preserved behavior and inventory

No production source changes are required. The generator, backends, Ada
runtime, dependency manifests/lockfile, and frozen historical Python evidence
are unchanged from the original PR. VisibleAscii retains exact `str`, the
U+0020–U+007E interval, all ten bound pairs, space preservation, constructor and
forged-carrier validation, strict JSON/member paths, restriction ancestry,
demand gating, and fail-closed unsupported profiles. Original BEFORE/AFTER
evidence and truthful `NOT READY` UCI service dispositions remain immutable.

Inventory: **20 ordinary cells, 14 Deep cells, 42 registered runner cells**,
with five common main lanes. Task101 GNAT14/GNAT16 retain lanes 3/5 and artifacts
`task101-ada-real-uci-dlz-14` / `task101-ada-real-uci-dlz-16`. Its source provenance,
final-gate checks, and partial-rerun reconciliation remain unchanged.

The four CI-SELECT-003 categories remain Fast tests, pinned-UCI tests,
pinned-Sleet tests, and Python CLI evidence examples. Infrastructure, shared
dependencies, unknown paths, deletions, renames, and mode changes still fall
back to full CI. Fixed-runner skip/expected-failure/duplicate/missing/incomplete
execution rejection, stale-receipt removal, atomic success publication, and
failed-qualification final-gate rejection remain mandatory.

## Qualification and handoff

Fresh raw command logs, exits, generated sources, numeric results, UCI
measurements, broker observations, compiler setup, and source identities are
retained separately under `/home/zboll/py010-r1-evidence`; the isolated build
directory is `/home/zboll/py010-r1-build`. Original evidence directories are
read-only inputs, never overwritten or substituted for fresh qualification.

The broker source is pinned at `e38f61d8ce0d75c8508434a52f2ed77c69cf6a3b`,
tree `7e705f4eb5f658957bd414b4abc8207311f3c1e4`, with authenticated existing
binary SHA-256 `4a2e9ee872d1deaa1a6ce1a1494a2f601660eb57c0df28a447b28a08eba21c94`.
Pinned UCI roots are authenticated by the unchanged existing SHA-256 checks.
Both Python 3.11/3.13 run normal and optimized generated-code/broker controls;
the full independent Float64 corpus requires 27,167 conversions and 20,474
encodings per version/policy cell, zero mismatches, and all negative controls.

The script suite additionally needs real `strace` in `PATH`. Enable the actual
disposable different-owner fixture with `TASK101_OWNERSHIP_FIXTURE=podman`;
without it the broad script suite deliberately reports one opt-in skip.
Neither prerequisite is grounds to disable or weaken tests.

Fresh feature/regression results:

- Both Python versions: 21 backend tests and 28 CLI/generated-code tests;
  Task039's frozen corpus, all ten bounds, standalone/package imports,
  normal/optimized execution, and deterministic regeneration are exercised.
- PY-010 pinned UCI: 24 service selections and 416 isolated profile tests per
  interpreter; `real_ready=0`. PY-007/008 inheritance and Float64 campaigns and
  PY-009's 24 selections/32 DateTime profile tests also pass against pinned roots.
- VisibleAscii-free historical byte parity: 20 selections, 50 sources.
- Full Float64 corpus: four cells, 27,167 conversions and 20,474 encodings each,
  zero mismatches; all independent numerical negative controls pass.
- Pinned Sleet: PY-005/006/007/008/009/010 and owned runner pass for both Python
  versions, retaining normal/optimized observed rejection and recovery evidence.
- Rust workspace with GNAT required: 1,501 passed, eight explicitly ignored
  external-campaign tests (not counted as passed). Mandatory feature-specific
  external campaigns are separately executed; hosted full Deep remains required.
- Task101: both compilers complete 12 real UCI/Sleet cells each, including
  all assertion policies and optimizations; both real bundles pass the unchanged
  independent verifier, including source/schema/generated-byte provenance.

For pre-merge Task101's mandatory clean-source check, a disposable standalone
repository was committed from candidate tree
`da11b7e35ddb531fe8a2580b8989444d4e9e9633` (snapshot commit
`3d81ff1ac90ec198ba0ccc6a972a9c235f356d29`). Evidence truthfully records that
local snapshot identity, not a hosted run or final merge HEAD. A subsequent
guard-only location-strengthening change affects only the projector, its tests,
and this report. Every Task101 input-source hash and the release generator are
independently checked unchanged against the final staged source. Final policy,
selection, broad script, split/mutation, source, and whitespace checks are rerun
after that strengthening. Exact-final-head hosted source-bound Task101 evidence
is still mandatory; local snapshot evidence is not substituted for it.

The final handoff records the normally committed merge HEAD, tree, both
parents, local results, fresh remote/GitHub HEAD agreement, clean worktree,
mergeability, and exact-head hosted run/artifact identities. The old 35/35
run `41147321182` qualifies only the original revision. A fresh full hosted
run, including Task101, security/trusted signal, required artifact provenance,
and `pr-ci-required`, is mandatory for the integrated head.

Disposition is `CORRECTIVE_WORK_REQUIRED` for a mandatory failure,
`HOSTED_CI_PENDING` while mandatory hosted checks remain outstanding, and
`READY_FOR_INDEPENDENT_REVIEW` only after all mandatory checks pass. Stop before
approving, auto-merging, or merging PR #132; independent review remains required.