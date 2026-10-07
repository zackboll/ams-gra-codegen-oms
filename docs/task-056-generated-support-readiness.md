# Task 056 — readiness accounts for generated-support types

Status: implemented. PRODUCT-CORRECTNESS task. Base: `main` @
`6cc1118ac993631052bbc4347f1b5a1d9e40688d`.

No backend capability was added. `xs:duration`, the `AircraftIdentifierType`
String facet profile, `EphemerisOrbitalModelType`, `EmptyType`, the Ada
`QueryType_Kind` collision, `SourceCommandEXT`, and open-world
`CapabilityCommandBaseType` all remain unimplemented. Task 056 makes the
verdict about them accurate; it does not solve them.

## Superseding integrity note (Task 064)

The measurements below remain historical Task 056 evidence. Task 064 adds
closed-world plan/schema binding for the same production generated-support
surface, without changing any selected/support counters or backend capability.
The previously documented selected-only binding limitation is closed; open-world
abstract-value errors remain authoritative. See
[Task 064](task-064-generated-support-plan-binding.md).

## 1. The defect

Task 032 keeps two type sets apart, on purpose:

| Set | Source | Meaning |
| --- | --- | --- |
| contract-selected semantic closure | `ServicePlan::selected_type_closure` | what the contract selects |
| generated-support closure | `ServiceGenerationProjection::generated_support_type_names` | what generated representation also needs (Task 024 closed-sum descendants and their dependencies) |

```text
abstract Base ; ConcreteA : Base
Holder { Value : Base } ; HolderReport -> Holder

contract-selected : Holder, Base
generated support : ConcreteA
```

`analyze_service_readiness` already projected once, built one
`CoverageAnalysis` and one baseline renderability snapshot over the PROJECTED
schema, and ran backend preflight on it. But it checked declaration
renderability only for `plan.selected_type_closure(schema)`. The projection's
support names were never consulted:

```text
unrenderable support declaration + every selected declaration renderable
    = false READY
```

`service-generate` then handed the projected schema to the backend, which
failed.

## 2. Reproduction before any production change

Single-message contract selecting `OrderOfBattle`, closed-schema, pinned UCI
2.5 (`093610b7`, root `ac943049...`) and 2.6 (`78eb61b6`, root
`af54ce72...`), release binary built from `6cc1118`:

| Release | Backend | service-check | selected types | service-generate | Backend failure | Output dir |
| --- | --- | --- | --- | --- | --- | --- |
| 2.5 | Ada  | READY (exit 0) | 55 / 55 | exit 1 | `unsupported Ada IR construct: type reference Primitive(Duration)` | absent |
| 2.5 | Rust | READY (exit 0) | 55 / 55 | exit 1 | `... unsupported facet profile on AircraftIdentifierType` | absent |
| 2.5 | C++  | READY (exit 0) | 55 / 55 | exit 1 | `... unsupported facet profile on AircraftIdentifierType` | absent |
| 2.6 | Ada  | READY (exit 0) | 56 / 56 | exit 1 | `... unsupported facet profile on AircraftIdentifierType` | absent |
| 2.6 | Rust | READY (exit 0) | 56 / 56 | exit 1 | `... unsupported facet profile on AircraftIdentifierType` | absent |
| 2.6 | C++  | READY (exit 0) | 56 / 56 | exit 1 | `... unsupported facet profile on AircraftIdentifierType` | absent |

The backends render in memory before writing, so no directory was created
even before Task 056; the defect was the verdict, not a partial write. The
measured 2.6 Ada failure is `AircraftIdentifierType`, not a duration.

The projection (identical in all three backends): 55 (2.5) / 56 (2.6) selected
types and **442** generated-support types in both releases.

The synthetic `support-readiness.xsd` + `support-blocked.yaml` fixture showed
the same defect with the pre-Task-056 binary in all three backends: READY with
2 / 2 selected types, then
`unsupported temporal declaration: Duration on BadDuration` from the backend.

## 3. The change

`ServiceBackendReadiness` gains three explicit, separate fields:

```rust
pub generated_support_types_total: usize,
pub generated_support_types_renderable: usize,
pub unsupported_generated_support_types: Vec<QualifiedName>,
```

* **total**: `projection.generated_support_type_names().len()`.
* **renderable**: support declarations the SAME baseline snapshot marks
  renderable.
* **unsupported**: the rest, in original `SchemaIr` declaration order (the
  projection's support order; no sort, no hash order, no fixed-point order).

They are computed from the projection readiness already built, with the
analysis and snapshot it already built: one projection, one
`CoverageAnalysis`, one snapshot, plus one pass over the support-name list. No
new dependency walker, no descendant recomputation, and no `Backend::generate`
call from `codegen-core`. A unit test instruments `CoverageAnalysis::new` and
`project_service_generation_schema` (`#[cfg(test)]` thread-local counters,
compiled out of every non-test build) and asserts exactly one of each per
readiness call.

`is_ready()` now also requires `unsupported_generated_support_types` to be
empty. READY means the contract-selected semantic model renders AND every
generated-support declaration the projection requires renders AND backend
global preflight passes AND service API preflight passes.

Service API preflight also requires the support list to be empty before it
runs, so a support-blocked model reports one cause rather than an extra
wrapper diagnostic. Codec readiness already skips when `!is_ready()`, so
`--with-codec` reports `codec: not measured (selected model is NOT READY)`.

### What did NOT change

* `ServicePlan::selected_type_closure`, `SchemaBinding.types`, and projection
  membership (asserted exactly for the Task 032 abstract fixture).
* `selected_types_total`, `selected_types_renderable`, `unsupported_types`,
  `selected_messages_total`, `selected_messages_renderable`,
  `blocked_messages`: they still describe only the contract-selected closure.
  A support-blocked model reports all its selected messages renderable; no
  message blocker is fabricated.
* `backend_preflight` stays a separate category (`backend_blocker`).
* Abstract support is judged by the same baseline rules as before; the Task
  032 `AbstractMiddle` intermediate stays renderable and READY.
* No backend renderer, no XSD frontend semantics, and no generated byte for
  any READY service.

### Open-extensions

Under `OpenExtensions` a selected abstract structural VALUE has no projection
(Task 028 fail-closed). No support surface exists to measure: the three fields
are `0` / `0` / empty, the existing single per-declaration and per-message
blocker is unchanged, and the report is byte-identical to before.

## 4. CLI report

The four selected lines are unchanged. When the projection has generated
support (`total > 0`) two lines follow them; when it is support-blocked, a
separate section lists the blockers, never labelled "selected":

```text
selected oms messages: 1
renderable selected oms messages: 1
selected type closure: 2
renderable selected types: 2
generated support types: 3
renderable generated support types: 2
status: NOT READY

unsupported generated support types:
  {urn:test}BadDuration
```

A projection with zero generated support prints exactly the pre-Task-056
report (conditional output, chosen to keep concrete-only services
byte-stable). `service-generate` prints the same report, exits 1, invokes no
backend, and creates no output directory.

## 5. After: OrderOfBattle

| Release | Backend | selected | support | renderable support | first unsupported support | service-generate |
| --- | --- | --- | --- | --- | --- | --- |
| 2.5 | Ada / Rust / C++ | 55 / 55 | 442 | 400 | `EphemerisOrbitalModelType` | exit 1 before backend, no dir |
| 2.6 | Ada / Rust / C++ | 56 / 56 | 442 | 402 | `AircraftIdentifierType` | exit 1 before backend, no dir |

No selected type is unsupported and no backend boundary applies in any cell.
The unsupported support lists are identical across the three backends within
a release (2.5: 42 entries; 2.6: 40, the same minus
`EphemerisOrbitalModelType` and `OrbitalEphemerisParametersReferenceType`).
`EphemerisOrbitalModelType.IntegratorStepSize` is a direct `xs:duration`
member: it is the declaration behind the historical 2.5 Ada
`type reference Primitive(Duration)` failure. `AircraftIdentifierType` and
`DurationType` are in the support set of every cell, so every historical
backend failure is now reported by readiness first.

## 6. Tests

Fast CI (synthetic, no network):

* `crates/codegen-core/src/service_readiness.rs` unit test: one projection
  and one `CoverageAnalysis` per readiness call.
* `crates/codegen-core/tests/service_readiness.rs` (+5): support-only NOT
  READY with unchanged selected counts; schema-order support blockers;
  abstract intermediate support stays READY; an unrelated unsupported
  declaration is not support; open-extensions reports no support surface.
* `crates/cli/tests/generated_support_readiness.rs` (+9): the parity matrix,
  all three backends:

  | Case | Boundary | Result |
  | --- | --- | --- |
  | A | selected unsupported type | NOT READY, no generation |
  | **B** | **selected supported, support unsupported** | **NOT READY, exact report, backend never reached, no directory** |
  | C | selected + support all supported | READY, generates, compiles, omits unselected unsupported declarations |
  | D | unrelated unsupported type only | READY, generates, no support lines |
  | E | global backend preflight (two namespaces) | NOT READY, separate `backend boundary:` |
  | F | service API wrapper collision | NOT READY, separate `service api boundary:` |

  plus: codec not measured for a support-blocked model; the Task 032
  abstract fixture READY with 2 selected + 8 support and compiling (rustc
  `-D warnings`, strict C++17, GNAT `-gnatc`); its projection membership
  exact.

New fixtures: `tests/fixtures/service-generate/support-readiness.xsd`,
`support-blocked.yaml`, `support-selected-unsupported.yaml`,
`support-ready.yaml`.

Deep CI `real-uci` (env-gated, pinned roots),
`crates/cli/tests/uci_generated_support.rs`:

* `task056_real_uci_order_of_battle_support_parity`: per release and
  backend, plan and projection succeed, counts as in section 5, NOT READY on
  support only, the backend really rejects the projected schema with a
  declaration in the reported support set, and the CLI stops before writing.
  Markers `UCI 2.5 ORDEROFBATTLE GENERATED SUPPORT PARITY: PASSED` and
  `UCI 2.6 ORDEROFBATTLE GENERATED SUPPORT PARITY: PASSED`.
* `task056_real_uci_category_a_readiness_matches_generation`: the 35 Task 054
  category-A messages through the library (no per-message subprocess), per
  backend: `is_ready() == Backend::generate(projection).is_ok()` for all 105
  cells; 34 READY and generated, `OrderOfBattle` alone NOT READY and only on
  support. Marker `UCI 2.5 CATEGORY-A GENERATED SUPPORT PARITY: PASSED`.
  Local run: 2 passed in 96 s (release).

`scripts/check-ci-split.sh` requires the target and all three markers in
Deep CI and forbids the target in Fast CI; `scripts/test-check-ci-split.sh`
drops each one, injects the target into Fast CI, and checks the marker
function on good, missing-marker, and libtest-prefixed output. The Deep CI
step uses `set -euo pipefail`, here-strings and whole-line `grep -Fqx`. Each
test prints a detail line before its marker, so a marker never shares
libtest's `test <name> ... ` line.

## 7. Regression evidence

* **Task 054 category A**, per backend, before -> after: 35 READY of which
  34 generate -> 34 READY, all 34 generate; `OrderOfBattle` READY-but-fails
  -> NOT READY before generation; zero previously generable message blocked.
* **Pinned 2.5 PositionReport** (Ada/Rust/C++, closed-schema): selected
  closure 60, generated support 0; `service-check` output, `service-generate`
  output tree and log, and Rust `--with-codec` check and generated files all
  byte-identical before/after. Real Sleet was not rerun because no
  PositionReport model or codec byte changed.
* **Every synthetic `service-generate` fixture contract** (52 contracts x 3
  backends x 2 worlds = 312 runs, before vs after binaries): every generated
  tree and generate log byte-identical; `service-check` output differs only
  in the 9 closed-schema cells whose projection has generated support
  (`abstract`, `codec-oam`, `codec-shape`), each by exactly the two added
  support-count lines. Open-extensions output is byte-identical everywhere.
* Before any new test was added, the existing workspace suite (1128) passed
  unchanged with the production change.

## 8. Performance

Release binary, one run each, wall time (no assertion added):

| Case | before | after |
| --- | --- | --- |
| 2.5 OrderOfBattle service-check (Ada / Rust / C++) | 6.17 / 6.18 / 6.15 s | 6.18 / 6.15 / 6.20 s |
| 2.6 OrderOfBattle service-check (Ada / Rust / C++) | 6.14 / 6.09 / 6.08 s | 6.11 / 6.08 / 6.06 s |
| 2.5 PositionReport service-check (Ada / Rust / C++) | 5.86 / 5.86 / 5.86 s | 5.89 / 5.90 / 5.86 s |

Within noise: no second multi-second analysis, only one pass over at most 442
names.

## 9. Out of scope, recorded separately

`SchemaBinding` fingerprints the contract-selected closure only, so changing
only a generated-support descendant between plan resolution and reuse is not
reported as a binding mismatch. Readiness and generation still agree in that
situation, because both re-project the schema they are given, so Task 056
parity does not depend on it. Recorded as its own roadmap item.

## 10. Task 057 follow-up: fixture adaptation and new measured figures

[Task 057](task-057-duration-support.md) makes zero-facet `xs:duration`
(named and direct) renderable in all three backends. Two things follow for
this task.

**Fixture adaptation, not a semantic change.** `support-readiness.xsd` used an
unconstrained `xs:duration` (`BadDuration`, `UnrelatedDuration`) as its
intentionally unsupported generated-support primitive. Left as is, case B
would silently turn READY and stop testing Task 056. It now uses `xs:time`
(`BadTime`, `UnrelatedTime`), which is still unsupported, and every case keeps
its verdict: A and B NOT READY, C and D READY, and B still exits 1 before any
backend call and writes nothing. The unit-level Task 056 schema in
`service_readiness.rs` was adapted the same way.

**OrderOfBattle after Task 057** (closed-schema, single-message contract,
identical in Ada / Rust / C++):

| Release | Support total | Renderable before → after | Unsupported before → after | First unsupported support before → after | Status |
| --- | --- | --- | --- | --- | --- |
| 2.5 | 442 | 400 → **403** | 42 → **39** | `EphemerisOrbitalModelType` → **`AircraftIdentifierType`** | NOT READY |
| 2.6 | 442 | 402 → **403** | 40 → **39** | `AircraftIdentifierType` → `AircraftIdentifierType` | NOT READY |

2.5 regained `EphemerisOrbitalModelType` (direct `IntegratorStepSize`),
`OrbitalEphemerisParametersReferenceType` (direct `EphemerisResultsStepSize`)
and `DurationType`. 2.6 regained `DurationType`. The two releases now have the
same 39-name unsupported support list, and the backend of every cell fails
first on `AircraftIdentifierType`, which is in that list. Selected counts are
unchanged (55 / 56, all renderable). `OrderOfBattle` stays NOT READY on
generated support, which is the Task 056 behaviour.
