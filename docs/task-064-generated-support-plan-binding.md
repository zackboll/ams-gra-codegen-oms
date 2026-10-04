# Task 064 — bind ServicePlan integrity to generated-support semantics

Immutable BEFORE / starting main: `cf9ebc518f49e58b1731417186c836f3a41e0e74`
(Task 061 merge, containing `add348ce867daf4253a88510a65a48a3ed23f218`).
Branch: `feature/064-generated-support-plan-binding`, independently based on
main, not on Task 062 or Task 063. No model capability is added.

## Frozen BEFORE reproduction

Before production edits, a standalone executable called the real
`resolve_service_plan`, `selected_type_closure`, `verify_schema_binding`, and
`project_service_generation_schema` APIs. The probe source, full output and
SHA-256 manifest are retained in `/tmp/task064/probes/before`,
`/tmp/task064/logs/before.log`, and `/tmp/task064/evidence/before.sha256`.
It printed `TASK064 BEFORE DEFECT: CONFIRMED` at the immutable benchmark.
This broken behavior is not a permanent passing regression.

```text
abstract Base
ConcreteA : Base { Code : String }
Holder { Value : Base }
SelectedReport -> Holder

selected: Base, Holder (schema order)
support: ConcreteA
```

For each mutation, selected closure and selected declarations were identical;
the old `verify_schema_binding(B)` returned `Ok`:

| Mutation in B only | Production observation |
| --- | --- |
| ConcreteA.Code String -> Boolean | projected support member changed |
| add ConcreteB : Base | projected support grew from one to two descendants |
| ConcreteA references SupportLeaf; change only SupportLeaf | projected transitive support body changed |
| optional empty abstract Base gains Concrete | support grew from empty to inhabited; Task 026 topology changed |
| effective member wire namespace urn:test -> urn:other | actual generated Rust codec wire spelling changed |

The namespace probe used the production API model and codec renderer with a
valid full IR. Its first attempt through service projection exposed an existing
namespace-narrowing boundary (member-only namespaces are not retained). The
permanent codec regression keeps both namespaces through a selected named
dependency, so it also exercises production service projection successfully.
No namespace projection capability was added in this task.

## One support model, world-independent resolution

The pre-existing fixed-point expansion was extracted, not duplicated, into
internal `expand_service_support` in `service_generation.rs`. Both resolution
and projection use it. It preserves ordinary dependencies, abstract structural
VALUE detection, concrete transitive descendants, nested support, world policy,
and schema-order filtering. There is no support walker in `service_plan.rs`.

Resolution still takes no world argument and selects exactly the old semantic
closure. It captures an internal closed-support semantic snapshot using the
shared expansion, without validating/rendering a projected schema. An empty
snapshot is retained: zero support does not exempt future topology changes.
If expansion is already unrepresentable, resolution stores that state rather
than introducing a world-specific resolution error. Projection still reports
the authoritative expansion error; a previously unexpandable model becoming
expandable cannot silently become a compatible pairing.

Projection checks selected semantics first. Under ClosedSchemaSet it checks
expected support declarations before expansion (so a missing transitive
dependency gets a binding diagnosis), expands once, verifies membership, and
filters that SAME set into the generated schema. No second projection or
independent descendant scan is introduced in a readiness call.

OpenExtensions does not check the closed snapshot. A demanded abstract value
continues to report `NotClosedUnderOpenExtensions`; readiness retains its
existing ordinary capability/blocker attribution. Selected binding remains
active in both worlds. Public `verify_schema_binding` remains explicitly
selected-only; world-aware callers must use production projection/readiness.

## Semantic snapshot audit

The snapshot uses explicit exact semantic equality, not pointer identity,
whole-schema equality, hashes, filenames, or serializer output.

| Captured surface | Existing production consumers |
| --- | --- |
| message QName and complete payload TypeRef | service API binding, payload readiness, selected projection |
| type QName, abstractness, base TypeRef | backend naming, inheritance projection, abstract-value topology, codec type discriminators |
| Primitive / Alias / enum wire values / Record / Choice / List body | emission planning, renderers, coverage, codec emission |
| list item reference and cardinality | dependency planning and storage capability |
| type/member constraints | integral/floating/string/binary/temporal lowering, readiness and codec validation |
| member name, TypeRef, cardinality, nillability | structural projection, member naming/storage, coverage and codec preflight |
| **member wire_namespace_uri (new)** | `service_codec::member_support` calls `FieldDecl::wire_name`; Rust codec `member_key` formats that QName |

Complete TypeRefs already include Binary lexical encoding, preserved by Task
052 binding tests. Member and enum order remain significant. No other missing
declaration semantic field was found. SourceRef.document, SourceRef.line, type/
member/message/enum documentation are deliberately excluded. Schema-wide version
metadata and namespace preferred prefixes are not declaration snapshots; the
backend layout audit found no generation dependency on their provenance.
This is not an IR redesign or a global schema fingerprint.

## Typed diagnostics and deterministic precedence

The only added `PlanBindingMismatch` variant is:

```rust
GeneratedSupport { name: QualifiedName, change: GeneratedSupportChange }
```

`GeneratedSupportChange` distinguishes `Missing`, `Changed`, `ClosureChanged`.
`MismatchRole` is unchanged; support is never labelled “selected type”.
Existing selected `Missing`, `Changed`, and `ClosureChanged` retain their exact
meaning. **Public source compatibility:** downstream exhaustive matches on
`PlanBindingMismatch` must add the new arm. Function signatures, selected
semantics, existing variants and fields are unchanged.

Precedence: selected messages in contract first-occurrence order, selected
types in captured schema order, support bodies in captured schema order,
then membership removals in captured order and additions in supplied schema
order. Traversal stack and map ordering cannot select a mismatch.

## Synthetic matrix and parity

`codegen-core/tests/generated_support_binding.rs` uses production APIs for:
concrete selected-only acceptance; unchanged support; changed, added and removed
descendants; transitive dependencies (including missing-dependency precedence);
nested abstract support changes and growth; Task 026 empty-to-inhabited;
unrelated changed/added/removed declarations; independently rebuilt schemas with
different paths, lines and docs; open-world error preservation; selected/support
wire namespaces; and exact deterministic first mismatches with simultaneous
changes. Changed support is rejected by readiness in every backend with the
same `PlanBindingMismatch` as projection, before backend consumption.

Task 056's unit instrumentation now also counts shared expansion calls:
one readiness call builds one production projection, one support expansion,
and one CoverageAnalysis. Plan resolution's initial snapshot is outside that
measured readiness call. Existing selected-binding regressions remain intact.

The CLI integration crate's two synthetic controls compare readiness, actual
generated Ada/Rust/C++ source, and generated Rust codec source across compatible
provenance-only reloads, and show actual codec key changes across a namespace
mutation with freshly resolved plans. Old-plan reuse rejects that mutation.
No renderer or codec mapping rules changed; identical codec source means no
codec execution/output semantics delta. No new compiler campaign is required.

Ordinary CLI usage resolves and consumes one schema in one invocation and
cannot naturally produce stale-plan reuse. No serialized plan format was
invented. Projection/readiness perform no filesystem writes and CLI generation
calls them before file writing; existing no-output-on-failure controls remain.

## Compact pinned real-UCI proof

One exact test adjacent to Task 056 evidence,
`task064_real_uci_generated_support_binding`, covers both pinned releases.
It checks pins, resolves OrderOfBattle normally, projects, chooses the first
support Record with a member in production order (`Acceleration3D_Type`),
clones in-memory IR and changes only that member's wire namespace. The clone
passes IR validation and selected closures are exactly equal. Both readiness
and projection reject the original plan with typed Changed support.
Pinned bytes on disk are untouched.

Local release test: **1 passed**, both release completion markers, **11.84 s**.
Counts remain **55 selected / 442 support** for 2.5 and **56 / 442** for 2.6.
Selected/support readiness metrics retain Task 056 definitions; no message's
capability changes for same-semantics pairing.

## CI and local controls

Fast runs synthetic binding, codec/source controls and single-pass instrumentation.
Deep adds only the exact compact pinned binding test; the historical Task 056
step is filtered to its original two tests to avoid executing the new test twice.
New exact steps verify registered names, require one nonzero test result,
print diagnostics before propagating original status, and require completion
markers. The real-UCI timeout remains 240 minutes.

Requested local validation logs are retained under `/tmp/task064/logs` and source
manifests under `/tmp/task064/manifests`. Task 064 uses its own target/TMPDIR.
Because the shared `/tmp` tmpfs filled, only Task 064's target and temp directories
were moved to disk-backed `/home/zboll/git/.task064-scratch` and symlinked through
the requested paths. No shared clean, Git configuration change, or sibling-task
interference was performed. Publication and fresh head/base keyed hosted results
are recorded separately from immutable BEFORE evidence.
### Recorded local results

- `cargo fmt --all -- --check`: passed.
- `cargo check --workspace --all-targets`: passed.
- `cargo clippy --workspace --all-targets -- -D warnings`: passed.
- `cargo test -p ams-gra-oms-codegen-core`: passed (including existing binding,
  projection, readiness and Task 026 controls).
- `AMS_GRA_REQUIRE_GNAT=1 cargo test --workspace`: passed.
- `cargo +1.95.0 check --locked -p ams-gra-oms-codegen-core -p
  ams-gra-codegen-oms --all-targets`: passed; no runtime API changes.
- `scripts/check-ci-split.sh`: passed; `scripts/test-check-ci-split.sh`: 128 checks.
- `git diff --check`: passed.
- Actual new Fast and Deep YAML run blocks executed locally: passed.
- Existing exact Task 056 pinned OrderOfBattle parity: passed, all six cells
  READY, selected 55/56 and support 442/442 (14.69 s).

Prepublication fetch still showed main at the immutable BEFORE commit. PR #63
remained OPEN at `8765b455235197eb533ba3d89c8a0522cf81dd60`, based on Task 061's
feature branch. Task 063 progressed independently to OPEN PR #64 against main,
head `a0b7930d24ac3a963917685a81fd1a17a42b4a89`; none of its commits are included.

## Corrective CI integration and Task 062 reconciliation

The original published head was `6981fb12739d3ced7ddb62b541e5e1e96948769b`,
based on the immutable BEFORE commit above. Deep CI `37179006472` succeeded,
including the compact binding proof. Fast CI `37179006463` failed only in the
split guard: the adversarial wrapper injected a Cargo listing failure into the
new Task 064 Deep step, and `set -e` exited before captured diagnostics printed.
This exact assertion was reproduced locally before correction.

Corrective commit `6da11f5930331213b5abed3678d9ee5dfc4a139e` captures the listing
status, prints combined stdout/stderr, and then propagates the original status.
The same pattern now protects all three Task 064 Fast listing commands. The
existing actual-workflow adversarial harness also supplies the registered test
name in its success fixture and requires both distinct generated-support steps.
It separately rejects listing/execution failures even with success markers,
missing registration, zero executed tests, and either missing release marker.
No inherited gate was removed or relaxed.

Normal merge `10b7bfc9c554ada19949e9515c8a40e64f7b08d1` has second parent
`52e22b67b94b431d96925c41ff730749c33b3c9e` (main after Task 062). Git resolved
the workflow, core export, roadmap and split-guard overlaps automatically;
there were no conflicts or manual overlap resolutions. No Task 063 commit was
merged or cherry-picked. Task 064 production binding code and regression tests
are unchanged from the original implementation head.

The immutable BEFORE source and output still pass their original SHA-256
manifest. Fresh corrective logs and extracted actual workflow blocks are kept
separately under `/tmp/task064/corrective`, using only Task 064's existing
isolated target and temporary directories. Post-merge split validation passes
132 checks; the actual-workflow adversarial suite passes 57 checks, and the
Task 061/062 wrapper suites pass 4/6 checks respectively. Fresh hosted results
must be keyed to the final published head and the reconciled main base; the
old successful Deep run does not certify this new pairing.

Before corrective publication, fmt, workspace all-target check, Clippy with
warnings denied, the inherited Task 062 exact compiler/codec gates, and the
actual Task 064 Fast workflow block all passed on the merged tree. Injected
failures in each of the three Fast listing commands also printed diagnostics
and preserved exit 53. The full GNAT-required workspace, MSRV and pinned
binding/parity campaign continues separately; its status and fresh hosted
head/base evidence must be checked before claiming final certification.
