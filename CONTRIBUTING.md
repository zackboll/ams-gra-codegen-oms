# Contributing

## Architectural rules

1. UCI/OMS XSD remains authoritative; do not invent a parallel public IDL.
2. Raw XSD/XML concepts stop at the frontend/IR boundary.
3. Language backends consume semantic IR only.
4. Unsupported schema constructs fail explicitly; never silently discard semantics.
5. Generated clients must preserve the existing LA-CAL WebSocket/OWP/JSON behavior unless an explicit ADR changes that decision.
6. Keep handwritten protocol runtimes separate from schema-generated source.
7. Every IR feature needs cross-language fixture coverage.
8. Prefer deterministic output and stable ordering.
9. Preserve source provenance in diagnostics and generated artifacts.
10. Do not claim OMS/AMS-GRA compliance solely from successful generation.

## Pull-request expectations

A change to the XSD frontend or IR should normally include:

- a focused XSD fixture;
- expected normalized IR behavior;
- negative/error coverage;
- at least one affected backend test or an explicit reason no backend change is needed.

## Continuous integration: Fast CI and Deep CI

Two workflows split the checks by *when* they run. See
[Task 055](docs/task-055-ci-fast-deep-split.md).

**Fast CI** (`.github/workflows/ci.yml`, workflow `CI`): the required
development feedback for every PR.

- Runs once per PR head (`pull_request`), once per commit on `main`, or
  manually. Pushing to a feature branch does not start a second `push` run.
- A newer commit on the same PR cancels the older head's run.
- Deterministic and local: fmt, check, clippy, GNAT-backed Ada, strict
  generated C++, generated Rust facades and codecs through mock OWP, the
  runtime crates at the Rust 1.95.0 floor, and
  `AMS_GRA_REQUIRE_GNAT=1 cargo test --workspace`.
- Never downloads the real UCI releases or builds Sleet
  (`scripts/check-ci-split.sh` enforces this).

**Deep CI** (`.github/workflows/deep-ci.yml`, workflow `Deep CI`): real
external/reference evidence.

- Runs on every push to `main`, nightly at 06:00 UTC, manually, and on PRs that
  change CI workflows or the deep-only scripts.
- Pinned real UCI 2.5/2.6 inventories (Tasks 052/053/054), the real-UCI
  generated-code compile at Rust 1.95.0, and the runtime against unmodified
  pinned Sleet.

Moving a check to Deep CI does not weaken or delete it. A red Deep CI on `main`
is a real failure. If your PR changes real-UCI or Sleet behaviour, run the
matching Deep CI step locally (for example `scripts/run-real-sleet-test.sh` with
`AMS_GRA_UCI_2_5_ROOT` set) or dispatch Deep CI on your branch before merge.
