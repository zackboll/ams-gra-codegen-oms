# CI-RESET-001: backend-specific PR smoke, complete main qualification

This is the active CI architecture. CI-SELECT-001/002/003/004 and the PR
artifact reconciliation gate are retired. Their design/evidence documents are
historical, not current PR requirements.

## Pull requests to main

`.github/workflows/pr-smoke.yml` runs on every PR, without path filters. Its
single stable job/check is **PR smoke**, on one GitHub-hosted Ubuntu runner.
It checks out `github.sha` (the proposed merge), has read-only contents access,
does not persist checkout credentials, and cancels superseded runs of that PR.
There is no matrix, private executor, secret, or Deep qualification dispatch.

`scripts/pr_smoke.py` uses `git diff --no-renames --name-only -z BASE MERGE`.
Disabling rename detection deliberately classifies both the deleted source and
added destination. New and deleted files do not need to exist on disk to route.
Invalid paths, undecodable diffs, empty diffs and Git errors select all three
lightweight suites (and syntax validation), never Deep CI.

Rules are applied per path and backend sets are unioned:

| Path | Execution |
| --- | --- |
| `.github/workflows/*.yml`, `*.yaml` | Workflow syntax validation |
| `docs/**` with `.md`, `.rst`, `.txt`; README/CONTRIBUTING/CHANGELOG; LICENSE, NOTICE | No backend compilation |
| Any `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml` | Python + Rust + Ada |
| `crates/backend-python/**`, `python/**`, `tests/fixtures/python-*`, `tests/python-*`, `crates/cli/tests/python_*`, `scripts/check-py*`, `scripts/test-py*`, `scripts/measure-py*` | Python |
| `crates/backend-rust/**`, `crates/runtime-rust/**`, `crates/runtime-api-rust/**`, `crates/runtime-rust-facade-tests/**`, `tests/rust-*`, `tests/fixtures/rust-*` | Rust |
| `crates/backend-ada/**`, `ada/**`, `qualification/ada/**`, `tests/ada-*`, `tests/fixtures/ada-*`, `tests/fixtures/service-generate/codec-ada-*`, `crates/cli/tests/ada_*` | Ada |
| Shared IR/frontend/core/CLI, infrastructure scripts, shared fixtures, unknown or ambiguous implementation | Python + Rust + Ada |
| Mixed backend changes | Union; workflow validation also runs if workflows change |

The infrastructure cutover PR therefore runs all three suites **and** actionlint.
Documentation-only and workflow-only runs do not install Rust or Ada compilers.
Router/execution unit tests run in all cases.

## Exact lightweight suites

Run from the repository root (absolute paths are printed in execution logs):

```sh
python3.11 scripts/pr_smoke.py run python
python3 scripts/pr_smoke.py run rust
python3 scripts/pr_smoke.py run ada
```

Every suite builds the debug production CLI with
`cargo build --locked -p ams-gra-codegen-oms --bin ams-gra-codegen-oms`.

* **Python:** verifies Python 3.11; runs
  `cargo test --locked -p ams-gra-oms-backend-python --test models --test codecs -- --nocapture`;
  generates the `python-codec/oam.xsd` model and codec via `service-generate`,
  `python-codec/service.yaml`, `--language python --world closed-schema --with-codec`;
  executes the existing nine independent/adversarial codec tests in
  `crates/backend-python/tests/codec_runtime.py`. Package tests also execute
  model/alias/scalar probes. No pip runtime is required.
* **Rust:** runs exactly `tests::track_matches_golden_and_is_deterministic` and
  `tests::schema_set_matches_dependency_order_golden` with
  `cargo test --locked -p ams-gra-oms-backend-rust --lib NAME -- --exact --nocapture`;
  generates the track fixture with the production `generate` command;
  compiles `tests/pr-smoke/rust_consumer.rs` against that output using
  `rustc --edition=2024 main.rs -o consumer`; executes positive and negative
  bounded-ID checks in the consumer.
* **Ada:** runs the same two exact renderer tests in
  `ams-gra-oms-backend-ada`; generates `service-generate/codec-ada-minimal`
  with `service-generate --language ada --world closed-schema --with-codec`;
  compiles `tests/pr-smoke/ada_consumer.adb` against the generated model/codec
  and existing `ada/oms-json-support`. Alire 2.1.1 selects exactly
  `gnat_native=16.1.0` and `gprbuild=26.0.1`; `alr exec -- gnat --version`
  must report **GNAT 16.1.0**, and gprbuild must report 26.0.0, before
  `alr -n build`. One consumer checks decode, encode/decode round-trip,
  optional absence, service metadata, and rejection of missing fields.

Missing tools, zero-test Rust runs, ignored Rust tests, unexpected skips,
missing completion markers, compilation and execution failures are errors.
There are no successful fallbacks or failure-masking steps. The 30-minute
outer timeout accommodates cold dependency/toolchain provisioning; **5–10
minutes is a goal, not an unmeasured performance guarantee**. No smoke suite
fetches UCI, builds the Sleet server, or invokes whole-workspace tests.

## Main, nightly, manual

`ci.yml` runs on every push to main and `workflow_dispatch` (including a
one-time pre-merge dispatch against this implementation branch). It always
calls complete `deep-ci.yml`, also available nightly/manual. All product
qualification jobs and their commands, both GNAT versions, both Python
versions, pinned real UCI/Sleet, historical regressions and the five
`oms-main-slot-1` through `oms-main-slot-5` lanes remain.

`Main qualification outcome` inspects only `needs.*.result`. A failed,
cancelled or skipped mandatory job makes it fail; it downloads no artifacts
and validates no historical receipts. Product qualification jobs still
publish and check their own substantive evidence.

`CI002 Security Contracts` runs on main, not routine PRs. `scripts/ci002/`,
its authorization tests, the trusted PR signal, and the private-dispatch
authorization workflow are unchanged. The repository activation variable
`OMS_TRUSTED_PR_DISPATCH_ENABLED` must remain false/unset: the signal alone
must not start qualification. Do not enable private PR qualification as part
of this architecture.

## Retired versus preserved

Removed: selector CLI/modules, policy projection modules, CI split-policy
shell entrypoints, PR artifact gate/retry replay, selector-only tests, and
their workflow invocations. Product qualification discovery, adversarial
runner/no-skip controls, owned evidence verifiers, Task 100/101 evidence and
Git trust-boundary tests, real-UCI disjoint-shard checks, and manual benchmark
authorization/reconciliation tests were retained or moved out of selector
test files into discoverable product test files. GNAT compiler placement
guards now reject conditional main qualification rather than require a PR
selector. The generic block reader is `scripts/workflow_text.py`.

`scripts/ci-qualification-inventory.json` is retained **unchanged as historical
data only** because the immutable real-UCI evidence contract lists its bytes
as an input, and manual benchmark trust tests consume its historical record.
It is not loaded by PR routing or main orchestration. Historical hashes,
test-result contracts, product sources and qualification command bodies were
not rewritten to make an obsolete policy pass.

## Required check and next merges

Required status after cutover: **PR smoke**, not `pr-ci-required`, old CI
matrix cells or CI002. Inspect branch protection **and** repository rulesets
using owner-authenticated access; an old required check is a merge blocker,
not an assumed successful check. Hosted validation and the configuration
change are reported in the delivery record.

**Tasks 102–106 (#134, #142, #144, #145, #147) must be reconciled against this
architecture before merging.** Their stacked historical workflow edits must
not restore `ci.yml`'s `pull_request` trigger, selectors, policy projection or
mandatory PR artifact gate. Preserve their product qualifications as main
qualification. This task does not modify those PRs or PERF-009.

## Validation records

Local/hosted observations, exact source identities, elapsed time and actual
runner-minutes are recorded separately at delivery, not inferred from the
initial performance goal. The stopped CI-SELECT-004-A1 controller and its
existing evidence are preserved outside the repository at
`/home/zboll/ci-reset-001-evidence/acceptance-snapshot`; disposable PRs #151
and #152 were closed unmerged. Unrelated campaigns/task processes were not
stopped.