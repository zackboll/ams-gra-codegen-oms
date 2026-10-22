# PERF-001-R2 — Reconcile PR #105 with Python-enabled main

## Integration identity and scope

- Existing PR: <https://github.com/zackboll/ams-gra-codegen-oms/pull/105>.
- Existing branch: `feature/perf-001-service-check-batching`.
- Starting remote PR HEAD: `1d8ec85954e4195fbfc15b4bdef6c95629809e99`;
  tree `ce7eb8432eef2d626a036f70d8280f2043646160`.
- Starting main: `0e3a89967cbb2c087eba78fae8ad3ca7cc4da3d2`.
- Original merge base: `23c8b2e4f7ebd600b8856b0e0e85d1c6eca25c39`.
- The clean local checkout initially lagged the remote PR by its Task 060
  wrapper-test correction. It was fast-forwarded without rewriting history.

The normal merge incorporates main's Python model CLI, service API, OMS JSON
codec, render-backed selected-service readiness, regression tests and Python
3.11/3.13 CI matrix. Main's existing Ada hexBinary timeout correction is also
inherited unchanged; R2 does not change workflows, gates, assertions or timeouts.

## Conflict resolution

Only `crates/cli/src/lib.rs` conflicted in the initial merge. Its resolution:

1. Loads and resolves standalone service inputs with the existing validation.
2. Dispatches Python to main's unchanged `python_service`, preserving actual
   render-backed model/API/codec readiness and fail-before-write generation.
3. Retains PERF-001's shared `evaluate_service_check` for Ada/Rust/C++ standalone
   and batch checks, including the authoritative readiness, codec and report
   implementations. There is no second batch readiness algorithm.
4. Preserves the fallible `backend_language()` interface and propagates errors
   with `?`, including codec preflight. Python is not added to
   `BackendLanguage::ALL`.

The only additional production change is the explicit Ada/Rust/C++ batch
allow-list in `crates/cli/src/service_check_batch.rs`. Python batch parity is
**not qualified**. An unsupported Python request rejects the entire manifest
with usage exit status 2 and a deterministic diagnostic, before schema loading
or request evaluation; stdout is empty, not a partial completion report.
Standalone Python `service-check` and `service-generate` remain supported.

`crates/cli/tests/service_check_batch.rs` covers Python-only and mixed manifests,
both request orders, both generation worlds, and codec on/off. Its existing
production-output equivalence matrix still covers Ada/Rust/C++, both worlds,
READY/NOT READY, codec on/off, missing contracts and unmapped extensions, comparing
complete stdout, stderr and per-request exit status. Schema/manifest identity
and malformed-input controls remain intact.

## Evidence boundaries

The original evidence in `docs/perf-001/` is unchanged. The historical baseline
was **1494.738841 seconds**, versus **134.177588 seconds** for batching, with
**11.14x** measured local speedup, **91.84%** CPU reduction and **609/609** exactly
equivalent selections against that original baseline. These are not measurements
of the reconciled main, and this reconciliation makes no identical-speedup claim.

R2 qualification logs and task-owned scratch are recorded separately under
`/home/zboll/perf-001-r2-evidence`. Shared `/tmp` had exhausted its inode capacity.
The first full CLI/workspace attempts encountered ENOSPC in the inherited test
that hard-codes `/tmp/task085`. The unchanged suites are qualified using a private
mount namespace binding task-owned scratch to `/tmp`; no shared files are deleted,
tests changed, or assertions skipped to work around the environment.

The final PR qualification comment records the merge commit/tree/parents,
completed local results, exact-head hosted run URLs and conclusions, and any
remaining blockers. Old-head CI never qualifies the new merge commit.

### Local qualification of the initial integration

Formatting, locked workspace/all-target check and Clippy with warnings denied,
locked CLI/all-target tests, and the locked Rust 1.95.0 CLI/all-target check pass.
The full CLI suite passes 353 tests (six pre-existing ignored tests). The
GNAT-required workspace suite passes 1,462 tests (eight pre-existing ignored
tests). Python backend tests pass 11 tests and the model/service/codec CLI tests
pass 18 tests on each of Python 3.11.17 and 3.13.16, with no ignored Python tests.

The five production batch integration tests, Task 061 Fast compiler/codec gates,
Task 060 wrapper controls (96 checks), Task 061 wrapper and transport controls,
Task 077B layout and mutation controls, and CI split validation/adversarial
self-tests (214 checks) pass. Python controls were exercised on both interpreters.
The pinned Task 061 wrapper passes both release inventories/impact checks and
both release DLZ compiler/codec verticals with GNAT required.

These results do not replace exact-final-head hosted qualification.

The full pinned-UCI `--mode equivalence` campaign also passes 609/609 complete
standalone/batch comparisons (444 release/backend/message tuples). Its frozen
production executable SHA-256 is
`4bdde41124bffc0e3ef4142eb6c64db7c451a8279a7686414a651ffbc4ab54a9`;
ordered results SHA-256 is
`85321ed8b6534196a944966dd727c141797beb5a7af86faf8ad27bf50f26ca33`.
New raw timing/accounting is retained in the separate task-owned evidence;
this shared-host qualification is not a replacement for the historical benchmark.

### Main advanced during qualification

PR #103 merged Python LA-CAL runtime support into main as
`07384e5407bd0d048c18fe45ba41aeca2f89ee33` while R2 was running. After qualifying
the initial integration, a second normal merge incorporates that newer tip before
the single push. No Rust source, Cargo input, or qualified generator code changes
between those main tips. The second merge's sole conflict is adjacent additions
in `scripts/check-ci-split.sh`: both PERF-001's mandatory batch checks and PY-005's
mandatory runtime checks are retained verbatim. Automatically merged workflow
and adversarial-control additions retain both sets of requirements.

The final integration is requalified before committing and pushing. PY-005's
mock and existing pinned-Sleet campaigns use the actual generated Python APIs
and codecs on Python 3.11 and 3.13, without rebuilding the existing clean pinned
broker unnecessarily. Runtime source is inherited unchanged from current main.

The final-tree focused qualification passes the same formatting, locked check,
Clippy, CLI/all-target, Rust 1.95.0 and Python 3.11/3.13 targets listed above.
Both sets of CI requirements pass together, including 218 CI split adversarial
checks. Each Python interpreter passes all 19 actual-network mock tests in both
normal and optimized modes, and the pinned-Sleet receipt/typed-delivery campaign
in both modes. The final release binary remains byte-identical to the executable
that passed the full 609-selection equivalence campaign above.
The repeated GNAT-required full workspace regression on the completed integration
also passes: 1,462 tests, zero failures, eight pre-existing ignored tests.
