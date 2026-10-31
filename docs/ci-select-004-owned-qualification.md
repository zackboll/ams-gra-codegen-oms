# CI-SELECT-004 — reusable source-bound Ada/Python qualification

## Baseline and disposition

Fetched and verified `origin/main` before branch creation:
`528397be1a0a5a7a14873e63bfd47399424a2df6`, tree
`fadfe5ab620f3ad4fa5e600f73148b1c502b2da6`.
Main run [41248170138](https://github.com/zackboll/ams-gra-codegen-oms/actions/runs/41248170138)
was **pending**, not successful, at baseline inspection.

The automatic inventory remains **20 ordinary, 14 Deep, 42 total registered
runner cells**, with the existing five main lanes. No new job/matrix is added.
CI-SELECT-001's complete-diff/source verification, CI-SELECT-002's Deep closure,
CI-SELECT-003's flat Python test identity contracts, CI002 and Trusted PR Signal,
main/full qualification, CI-RETRY-001 producer-attempt reconciliation, and manual
benchmark isolation remain required. Historical workflow/source digests are not
updated to bless new payloads; exact additive hook projection precedes them.

Task102–105 PRs #134/#142/#144/#145 were inspected read-only. Their recurring
operations are authentic root/include verification, production service-check and
model/codec generation parity, compiler provisioning, assertion/optimization
matrices, real broker admission/decoder rejection separation, source/log hashes,
uploads and final-gate provenance. Their task-specific corpus, selected closure,
rejection reasons and schema differences are **not interchangeable**. These PRs,
branches and worktrees are not modified or migrated by this implementation.
PY-010's additive generation, parity, both interpreters/policies, shared broker,
and historical PY-005–009 operations remain in their original execution paths.

## Ada interface

`qualification/ada/{fast,uci,sleet}/<task-id>/manifest.json` is data, never a
command. Each task has a unique canonical lowercase identifier, sorted unique
`sources`, `fixtures`, and `tests`, `version: 1`, a fixed `profile`, and one Ada
`main` source. Files are flat, nonexecutable, regular, non-hardlinked and
nonsymlinked; all bytes in the folder must be registered. Unknown fields, duplicate
JSON keys, missing cases/files/modes, traversal, unsupported suffixes and ambiguous
ownership fail. No task-supplied GPR, external toolchain, URL or workflow steps
are supported. A generated fixed GPR owns compiler switches.

Supported profiles:

* **ada-test-v1 / fast:** Ada main emits exactly one `CI004_PASS <test-id>` per
  registered test after explicit checks; source literals independently reconcile
  test IDs. GNAT 14.2.0 and 16.1.0 each execute Normal/Check/Ignore × O0/O2.
* **ada-codec-v1 / uci:** exact `release` 2.5 or 2.6, owned service `contract`,
  `package`, `payload`, and JSON `cases` declaring ID/input/expected. Both positive
  and negative cases are mandatory. The existing batch-consumer protocol reports
  ACCEPT/REJECT for every input; the trusted verifier checks values/kinds against
  the independently authored expected corpus. Every compiler/policy/optimization
  cell compiles actual production-generated code.
* **ada-dlz-sleet-v1 / sleet:** bounded DLZ transport profile reuses the unchanged
  Task101 real broker driver, authenticated existing broker build and transport
  dependencies. Mandatory broker-rejection, decoder-rejection and recovery checks
  require real wire observations, actual Ada handlers and clean shutdown. This is
  not an arbitrary-message dispatcher: additional transport profiles need reviewed
  shared-harness work before Task102–105 migration.

Fast runs additively in `ada-lacal-runtime-core`; pinned profiles additively in
`ada-real-uci-dlz-sleet`. Task101's original runner/verifier behavior is unchanged.
No second Sleet build is introduced. New task registrations do not edit workflows,
policy scripts, the inventory or the final gate.

## Python fixture interface

Existing flat `python/qualification/{fast,uci,sleet}/test_*.py` ownership remains.
A module may add `fixtures/<module-stem>/manifest.json` with exactly `version`,
`test`, sorted `files`, and sorted complete `tests` (module.Class.test_method IDs).
Only flat `.json`, `.txt`, `.xsd`, `.yaml` data are allowed. AST inventory is
reconciled with unittest discovery and actual starts/passes; fixture registrations
cannot use `load_tests` to omit source tests. Skips, errors, expected failures,
suppressed execution and duplicate identities cannot produce accepted receipts.
The checked-in examples generate actual Python code and exercise Fast, pinned UCI,
and an inherited real transport probe. The Python UCI example extracts the exact
authenticated DateTimeType declaration from each release (as PY-009 does); it is
**not full real-UCI service interoperability**. The attempted full DLZ example
failed at the existing unsupported Python primitive Time boundary. That production
boundary and all historical NOT READY expectations remain unchanged.
Both real Python 3.11/3.13 interpreters run
normal and `-O` in their existing owners. Existing CI-SELECT-003 UCI receipt
publication/invalidation remains in place; new source-bound receipts are additive.

## Ownership and evidence

Validated Ada Fast selects Rust + GNAT14/16 runtime Fast. Ada UCI/Sleet selects
the same Fast owner plus GNAT14/16 existing pinned DLZ owner. Python Fast selects
Rust + Python3.11/3.13; UCI adds real-uci-regressions, Sleet adds real-sleet.
Mixed changes form the union plus inherited dependency closure. Registration
ownership is independently discovered from current checked-out source, not a
filename glob. Unknown/unsafe/unregistered/incomplete changes fail broad; shared
generator/backend/IR/schema paths keep their inherited conservative policy.
Workflow, security, shared harness and selector changes require full CI, including
this infrastructure PR. Task folders do not grant selective production ownership.

Ada evidence is added under `ci004-ada-{fast,pinned}/` inside the existing
Task099/Task101 compiler artifacts; their historical contents and names remain
mandatory. Artifacts `ci004-python-{fast-3.11,fast-3.13,uci,sleet}` are additive
and mandatory whenever their owner is selected. The final gate downloads
digest-verified data-only archives and uses
the unchanged CI-RETRY-001 resolver to bind them to an exact successful owner and
publication step, actual producing run/attempt, source head/tree and repository.
Independent verifiers reconstruct registrations and required cells from source;
they do not execute artifact code or accept self-reported task completion alone.
Receipts include compiler/interpreter versions and binary hashes, generator hash,
source/input hashes, pinned identities where applicable, exact execution inventory
and actual logs/outcomes. Failed runs cannot retain a successful completion receipt.

## Validation and hosted acceptance

Local evidence and commands are preserved outside the repository under
`/home/zboll/ci-select004-evidence`. Synthetic receipt mutation fixtures test
verifier rejection only; they are not real compiler or transport qualification.
Real local campaigns and exact-head hosted CI must be reported separately.

Disposition: **HOSTED_CI_PENDING**. The implementation PR must receive exact-head
full CI before the nine disposable selective acceptance PRs are created. Those
must compare actual job allocations, execution/artifacts/final gate, runner-minutes
and workflow elapsed time against full-CI baselines; skipped labels alone are not
savings. Close without merging after retaining evidence. Do not claim readiness
before hosted acceptance and independent verification. Stop before merging.