# Service Contract Integration

This document records how `ams-gra-codegen-oms` consumes a **portable Service
Contract** as a first-class code-generation input, and where the authority
boundaries sit.

## Portable Service Contract compatibility baseline

```text
Portable Service Contract compatibility baseline:
zackboll/ams-gra-service-contract
4ea5be8dd36e9695bd58f2c36c6b3dd8075de249

contract_version: 0.1
```

The two projects are **not commit-lockstep**. Compatibility is defined by
`contract_version`, not by repository SHA equality. The SHA above records which
revision of the portable format this implementation was written and tested
against; a later Service Contract commit that still publishes
`contract_version: "0.1"` remains compatible, and a contract declaring any
other version is rejected outright rather than parsed on a best-effort basis.

This repository is the first independent, non-Python consumer of the portable
format. Parsing is Rust-native (`serde` / `yaml_serde` / `serde_json`; the
YAML dependency is Cargo-aliased as `serde_yaml`). At no point does the build
or the runtime shell out to Python or invoke the Service Contract repository's
tooling.

## Pipeline

```text
                         Service Contract
                         "what is used"
                               |
                               v
                         Contract IR
                               |
                               |
 UCI baseline XSD ------------+
 UCI overlay XSDs ------------+
                               |
                               v
                         Schema IR
                               |
                               v
                     Resolved Service Plan
                               |
                future contract-driven codegen
                    /          |          \
                  Ada         Rust         C++
```

Task 030 stops at the Resolved Service Plan. No Ada, Rust, or C++ service
wrappers are generated, and backend generation behavior is unchanged.

## Authority boundaries

| The Service Contract owns | The UCI XSD / Schema IR owns |
|---|---|
| service identity | whether a UCI message exists |
| component kind | its qualified name |
| Capability inventory | its payload type |
| function existence/grouping | the payload's transitive type graph |
| function category | records |
| required group | choices |
| Capability ownership | inheritance |
| standard role | cardinality |
| applicability | enums |
| exchange existence | constraints |
| exchange kind, direction, mandate | schema documentation |
| topic and timing | |
| Data Transfer / Special Signal / Security / Non-OMS metadata | |

The Resolved Service Plan owns **only the join**. Service Contract semantics
are never moved into `SchemaIr`, and XSD/type semantics are never moved into
the contract parser.

## Crate layout

| Crate | Responsibility | Forbidden dependencies |
|---|---|---|
| `ams-gra-oms-service-contract` | portable v0.1 parsing, typed Contract IR, portable semantic validation | `xsd-frontend`, `codegen-core`, backends, CLI, runtime |
| `ams-gra-oms-codegen-core` | the language-neutral Resolved Service Plan and the contract/UCI join | backends, CLI |
| `ams-gra-codegen-oms` (CLI) | the `service-plan` command and extension-identifier mapping | -- |

Resolution deliberately does **not** live in `backend-ada`, `backend-rust`,
`backend-cpp`, or the CLI: the plan is language-neutral, and all three future
service backends must consume the same one. No language backend parses YAML or
JSON.

## What was NOT ported

`profiles/oms/2.5/profile.yaml` and the completion-assistant machinery stay
owned by the Service Contract project. This repository consumes an
**already-authored** portable contract. It contains:

* no OMS profile conformance engine;
* no contract completion logic, worksheets, decisions, or mapping consumption;
* no automatic Capability inference;
* no automatic function grouping;
* no automatic topic generation;
* no Section 3.3 topology regeneration.

Validation in this repository only ever **rejects**; it never fills anything
in. The final portable contract is authoritative for codegen.

## Contract IR

The typed Contract IR represents every current v0.1 portable field:
`Contract`, `Service`, `Standards`, `Source`, `TraceRef`, `Capability`,
`Function`, `Exchange`, and `Timing`. Typed enums are used for service kind,
function category, required group, standard role, applicability, exchange
kind, direction, mandate, and timing kind.

Exchange kinds are **enum variants**, not one struct with a `kind` tag plus a
bag of optionals. Each kind's required fields are therefore non-optional in
the IR, and unrepresentable states (a Data Transfer carrying a `message`, an
OMS Message carrying a `protocol`) cannot be constructed.

Unknown YAML/JSON properties are rejected via `#[serde(deny_unknown_fields)]`
throughout. A future portable field fails loudly rather than being silently
dropped. (`Timing::Asynchronous` is spelled as an empty struct variant rather
than a unit variant, because serde's internally tagged unit variants would
otherwise accept and discard stray sibling keys.)

### Omitted vs explicit-empty Capabilities

`capabilities` is `Option<Vec<Capability>>`, and all three author states stay
distinguishable end to end, including in the Resolved Service Plan:

| Contract text | Contract IR | Service Plan | Reported as |
|---|---|---|---|
| (absent) | `None` | `None` | `capabilities: (omitted by contract)` |
| `capabilities: []` | `Some(vec![])` | `Some(vec![])` | `capabilities: (explicitly empty)` |
| `capabilities: [...]` | `Some(non-empty)` | `Some(non-empty)` | `capabilities: N` |

Omitted is never normalized to empty: that would convert "unstated" into an
assertion the author never made.

## Portable validation

Parsing fails closed. Implemented v0.1 invariants:

* `contract_version == "0.1"` (checked before semantics, so a future contract
  reports an unsupported version rather than a pile of v0.1 rule violations);
* the identifier grammar `^[a-z][a-z0-9]*(?:[-_][a-z0-9]+)*$`;
* duplicate source IDs, Capability IDs, function IDs;
* duplicate exchange IDs *within* a function (exchange IDs are scoped per
  function, so two functions may each have a `status-output`);
* unknown function Capability references, including when the inventory is
  omitted entirely -- absence of an inventory is not permission to invent one;
* unknown traceability source references;
* `category: specific` must not declare `required_group`;
* `not_applicable` requires `not_applicable_reason` and `exchanges: []`;
* Capability objects require `id`, `name`, and
  `requires_position_information`;
* exchange-kind required fields, and present-but-empty required text;
* timing-kind structure;
* strictly positive numeric timing values (zero, negative, and NaN rejected;
  *absent* values stay absent and are never defaulted).

The first violation in contract order is reported, with logical
function/exchange/Capability identity rather than byte offsets, so diagnostics
are stable across runs and platforms.

## Message resolution

Contract OMS Message names are resolved against `SchemaIr.messages` by **exact
local-name equality**:

```text
0 matches  -> fail
1 match    -> resolve
>1 matches -> fail ambiguous, reporting the candidate qualified names
```

There is no case folding, prefix matching, suffix matching, substring
matching, fuzzy matching, or first-match-wins. A near-miss is a
contract/schema mismatch, not a hint. Errors carry function and exchange
identity:

```text
function 'mission-data' exchange 'position-input':
OMS message 'PositionReport' was not found in the supplied schema set
```

## Repeated uses of one message

This stays legal:

```text
Function A / Exchange X -> PositionReport / topic A
Function B / Exchange Y -> PositionReport / topic B
```

Both exchange occurrences are retained in the plan with their own topics,
directions, mandates, and timing. `ServicePlan::selected_messages()`
separately exposes the deduplicated selection, ordered by **first occurrence
in contract function/exchange order** rather than by a set ordering that would
lose author order.

## Non-UCI exchanges

`data_transfer`, `special_signal`, `security_exchange`, and `non_oms_message`
are preserved verbatim in the plan, are never resolved against `SchemaIr`, are
never dropped, and never cause a failure merely because they are not UCI
messages. They are genuine parts of the service interface.

## Contract-selected type closure

`ServicePlan::selected_type_closure(&schema)` starts from each selected
message's `MessageDecl.payload_type` and follows the same semantic dependency
categories used by the rest of codegen planning: named base types, aliases,
record fields, choice alternatives, and list item types. Those categories live
in exactly one place (`codegen_core::direct_named_dependencies`), which both
coverage closure analysis and declaration ordering now also call, so no second
inconsistent dependency model exists.

The result is returned in schema declaration order, which is deterministic and
independent of contract ordering. It answers:

> Which normalized UCI type declarations does this Service Contract require?

It does **not** yet answer whether Ada, Rust, or C++ can render all of them.

## GenerationWorld separation

`service-plan` requires no `--world`. Contract parsing and message/type
resolution are independent of `closed-schema` vs `open-extensions`: which
messages and types a contract selects is a fact about the contract and the
schema, while renderability is a backend policy question. Task 031 can combine
the selected closure with backend/world renderability.

## Extension identifiers are not paths

A portable contract declares:

```yaml
standards:
  uci_extension_schemas:
    - ext-a-1.0
    - ext-b-2.0
```

Those are **logical extension identifiers**. Task 029 overlays are filesystem
paths. The two are never conflated, and an extension ID is never guessed at as
a path or derived from a filename.

The `service-plan` command therefore takes a repeatable explicit pair:

```bash
ams-gra-codegen-oms service-plan \
  --schema public.xsd \
  --contract service.yaml \
  --extension ext-a-1.0=/path/private-a.xsd \
  --extension ext-b-2.0=/path/private-b.xsd
```

The ordinary Task-029 `--overlay PATH` option is **not** overloaded with
guessed identifiers, and `service-plan` does not accept `--overlay` at all.

### Exact extension-set matching

The supplied set must match the declared set exactly. All of these fail:

| Situation | Result |
|---|---|
| declared extension with no mapping | usage error naming the missing ID |
| mapping for an undeclared extension | usage error naming the extra ID |
| duplicate `--extension` identifier | usage error |
| any `--extension` when the contract declares none | usage error |

There are no silent extras. A missing mapping means the operator did not
supply schema content the contract says it needs; an extra mapping means the
operator supplied content the contract never asked for. Both silently change
the resolved type universe, so both are rejected.

If the contract omits `uci_extension_schemas` or declares an empty list, no
`--extension` options may be supplied.

### Overlay load order

Task 029 gives overlay caller order semantic significance. For contract-driven
planning, overlays are ordered by `standards.uci_extension_schemas`, **not**
by CLI option order. Given the contract above, this command line:

```bash
--extension ext-b-2.0=/b.xsd --extension ext-a-1.0=/a.xsd
```

still calls:

```rust
load_schema_set_with_overlays(root, [a_path, b_path])
```

because the contract -- not the shell history that produced the command -- is
the reproducible authority for how a service composes its extensions. Task
029's existing loader is reused unchanged; no second overlay loader exists.

Task 029's rule that ordering never implies duplicate-declaration precedence
is untouched: overlays remain purely additive, and duplicate qualified names
still fail in the frontend regardless of which order they were composed in.

## UCI root version evidence

A contract declares a **logical** UCI schema version:

```yaml
uci_schema_version: "2.5"
```

The supplied XSD root separately carries an `xs:schema @version` release
string. The authoritative roots were inspected directly:

| Release | Pinned commit | Root document | `xs:schema @version` |
|---|---|---|---|
| UCI 2.5 | `093610b7753944059360d3236770ab446d039556` | `UCI_MessageDefinitions_v2_5_0.xsd` | `002.5.0` |
| UCI 2.6 | `78eb61b6112c8bffa40820c33124b57787fc5bd9` | `UCI_MessageDefinitions_v2_6_0.xsd` | `002.6.0` |

No documented, normative mapping from the contract's logical version to the
XSD root release value was found in either release or in the portable-contract
project. This implementation therefore **preserves both values and defers
strict comparison**:

* `ServiceStandards::uci_schema_version` holds the contract's logical value;
* `ServiceStandards::schema_root_version` holds `SchemaIr::schema_version`,
  the root's declared release string;
* the two are reported side by side and are never compared.

No string surgery is performed. Zeros are not stripped, dots are not split,
and it is not assumed that `002.5.0 == 2.5`. That equivalence is plausible,
but plausible is not documented, and encoding a guess as a validation rule
would make a mismatch that matters indistinguishable from a formatting
difference that does not. If a stable mapping is later documented, it can be
implemented explicitly and tested at that point.

## Fixtures

| Fixture | Purpose |
|---|---|
| `tests/fixtures/service-plan/root.xsd` | controlled non-UCI schema set: `PositionReport` and `ObservationMeasurementReport` plus dependent types, and an unselected `UnusedReport`/`UnusedType` pair that proves the closure is a genuine selection |
| `tests/fixtures/service-plan/service.yaml` | **synthetic** contract, clearly labelled as such |
| `tests/fixtures/service-plan/service-extension.yaml` | two declared logical extension IDs |
| `tests/fixtures/service-plan/private-overlay.xsd` | declares `PrivateReport`, which exists in no other document |
| `crates/service-contract/tests/fixtures/upstream-*.yaml` | verbatim upstream contracts with recorded provenance |

Ordinary CI requires **no external UCI schemas**. The controlled fixture
exercises Contract IR + Schema IR = Service Plan, not full UCI fidelity; UCI
fidelity evidence lives in `docs/uci-schema-compatibility.md`.

The synthetic fixture's function grouping is invented for testing. In
particular, pairing a position input with an observation-measurement output in
one function does **not** come from IR Search & Track evidence or any other
upstream source.

The upstream fixtures are authoring **examples** copied verbatim (with only a
provenance header prepended) from the Service Contract project at the baseline
commit. No completion-tooling file -- no profile, worksheet, decisions,
mapping, or scaffold -- is consumed.

## Error model

Deterministic errors exist for every failure this slice can produce:

| Failure | Type |
|---|---|
| contract read / unsupported extension | `ContractError::Io` |
| contract parse failure, unknown property | `ContractError::Parse` |
| unsupported contract version | `ContractError::UnsupportedVersion` |
| contract semantic failure | `ContractError::Semantic(SemanticError::*)` |
| missing contract extension mapping | CLI usage error (exit 2) |
| extra extension mapping | CLI usage error (exit 2) |
| duplicate extension mapping | CLI usage error (exit 2) |
| unknown OMS message | `ServicePlanError::UnknownOmsMessage` |
| ambiguous OMS message | `ServicePlanError::AmbiguousOmsMessage` |
| unresolved payload type | `ServicePlanError::UnresolvedPayloadType` |

Message-resolution errors always carry logical function and exchange context;
ambiguity errors additionally list the candidate qualified names.

## CLI example

```bash
ams-gra-codegen-oms service-plan \
  --schema tests/fixtures/service-plan/root.xsd \
  --contract tests/fixtures/service-plan/service.yaml
```

```text
service contract valid
contract version: 0.1
service: Contract Codegen Test
kind: service
contract uci schema version: 2.5
schema root version: 000.1.0
capabilities: (omitted by contract)

functions: 1
exchange occurrences: 2
oms message exchanges: 2
unique uci messages: 2
selected type closure: 5

mission-data / position-input
  input PositionReport
  topic: mission.position-report
  resolved: {urn:test}PositionReport

mission-data / observation-output
  output ObservationMeasurementReport
  topic: mission.observation-measurement-report
  resolved: {urn:test}ObservationMeasurementReport
```

The command performs no filesystem writes, and there is no `--language`,
`--output`, or `--world`.

## Backend readiness

The Resolved Service Plan answers *what does this contract select?*. It does
not answer *can we render it?*, because that depends on which backend and which
generation world, and neither belongs in a plan. Task 031 adds a separate
analysis for the second question:

```text
ServicePlan + SchemaIr + BackendLanguage + GenerationWorld
    -> ServiceBackendReadiness
```

```rust
pub fn analyze_service_readiness(
    plan: &ServicePlan,
    schema: &SchemaIr,
    language: BackendLanguage,
    world: GenerationWorld,
) -> Result<ServiceBackendReadiness, ServiceReadinessError>
```

`ServicePlan` is unchanged: it still has no `BackendLanguage` and no
`GenerationWorld` field. The same plan can be measured against Ada and Rust, or
against a closed and an open world, without being re-resolved, because the
selection is not a function of who compiles it.

The result is:

```rust
pub struct ServiceBackendReadiness {
    pub language: BackendLanguage,
    pub world: GenerationWorld,
    pub selected_types_total: usize,
    pub selected_types_renderable: usize,
    pub selected_messages_total: usize,
    pub selected_messages_renderable: usize,
    pub unsupported_types: Vec<QualifiedName>,
    pub blocked_messages: Vec<BlockedMessage>,
}
```

`is_ready()` is true exactly when every selected OMS Message closure is
renderable.

### What is measured, and what is not

Only the transitive type closures of contract-selected OMS Messages are
measured. Three consequences follow, all deliberate:

- **Non-UCI exchanges do not affect UCI type readiness.** Data Transfer,
  Special Signal, Security Exchange, and non-OMS Message exchanges are real
  parts of a service interface that simply require no UCI type model. They are
  preserved in the plan and ignored here. A contract with zero OMS Message
  exchanges is therefore *vacuously* ready: `selected_messages_total` and
  `selected_types_total` are both `0` and `is_ready()` is `true`, for every
  backend and both worlds.
- **Unselected unsupported UCI declarations do not block readiness.** This is
  the point of the whole layer; see the evidence below.
- **READY does not mean service source has been generated.** Nothing is
  written. Task 031 delivers the capability boundary; Task 032 can consume it
  to actually emit contract-selected source.

Readiness uses **actual** backend capability: no hypothetical `FeatureFamily`
is enabled, so there is no "ready if X were implemented" verdict.

### Ordering

The two lists are ordered differently on purpose, because they answer different
questions:

| List | Order | Why |
| --- | --- | --- |
| `unsupported_types` | schema declaration order | matches `selected_type_closure`, which is contract-order-independent |
| `blocked_messages` | contract first-occurrence order | matches `selected_messages`, which preserves the author's presentation |

Each blocked message carries exactly one deterministic first blocker, typed
rather than prose:

```rust
pub enum ServiceMessageBlocker {
    Declaration(QualifiedName),      // earliest non-renderable member, schema order
    Primitive(PrimitiveKind),        // unsupported primitive payload
    PayloadReference(QualifiedName), // unsupported payload *position*
}
```

For a named payload, the first blocker is the earliest non-renderable
declaration in that message's dependency closure in schema order -- never an
arbitrary map member. A message selected by several exchanges is analyzed once
and reported once.

### One capability model

Readiness does not re-implement any renderability rule. It delegates entirely
to `CoverageAnalysis`, which already owns the primitive, cardinality, Choice,
inheritance, abstract-value, and world rules from Tasks 017-029. The
per-declaration renderability vector that `backend_coverage` computed
internally was extracted into one reusable snapshot that both consumers now
share, so a rule cannot be true for coverage and false for readiness.
Full-schema coverage results are byte-for-byte unchanged by that refactor.

Per readiness request there is exactly one `CoverageAnalysis`, one baseline
renderability snapshot, and then index lookups. No whole-schema scan happens
per selected type or per selected message, and the 31 hypothetical feature
combinations that a full `coverage` report evaluates are **not** run.

### Authoritative UCI 2.5 evidence

Against authoritative UCI 2.5 (`UCI_MessageDefinitions_v2_5_0.xsd`, 5,557 types
and 722 messages) with the copied upstream portable contract
`crates/service-contract/tests/fixtures/upstream-minimal.yaml` (provenance
`zackboll/ams-gra-service-contract` @
`4ea5be8dd36e9695bd58f2c36c6b3dd8075de249`):

`PositionReport` resolves by exact local-name equality to
`{https://www.vdl.afrl.af.mil/programs/oam}PositionReport`. It selects **1**
unique OMS message and a **60**-declaration type closure.

Measured, not predicted:

| language | world | messages renderable | types renderable | status | first blocker | elapsed |
| --- | --- | --- | --- | --- | --- | --- |
| ada | closed-schema | 0/1 | 32/60 | NOT READY | `{…}Acceleration3D_Type` | 255.0 s |
| ada | open-extensions | 0/1 | 32/60 | NOT READY | `{…}Acceleration3D_Type` | 256.4 s |
| rust | closed-schema | 0/1 | 47/60 | NOT READY | `{…}AltitudeType` | 255.9 s |
| rust | open-extensions | 0/1 | 47/60 | NOT READY | `{…}AltitudeType` | 257.7 s |
| cpp | closed-schema | 0/1 | 47/60 | NOT READY | `{…}AltitudeType` | 255.8 s |
| cpp | open-extensions | 0/1 | 47/60 | NOT READY | `{…}AltitudeType` | 255.8 s |

Ada genuinely differs from Rust and C++ here — fewer renderable selected types
and an earlier first blocker in schema order — which is exactly the
per-backend discrimination this analysis exists to provide. Closed and open
worlds agree on this particular closure because nothing in it is an abstract
structural value position; the difference between worlds is visible in the
synthetic regressions above, not here.

Task 031 reports these blockers. It deliberately does **not** implement them:
`AltitudeType` and `Acceleration3D_Type` support is not in scope.

The contract declares logical `uci_schema_version: "2.5"` while the XSD root
declares `002.5.0`. Both are retained and never compared; no normalization was
introduced. This is a UCI 2.5 probe only and supports no UCI 2.6 claim.

### Where the authoritative time goes

The ~255 s is worth attributing precisely, because a readiness query must not
secretly behave like a full `coverage` report. Phase timings for the same
authoritative inputs:

| phase | elapsed |
| --- | --- |
| `load_schema_set_with_overlays` (XSD parsing) | 5.81 s |
| `SchemaIr::validate` | 10.3 ms |
| `resolve_service_plan` | 13 µs |
| `CoverageAnalysis::new` | **244.4 s** |
| one full renderability snapshot + all 722 message closures | 46.0 ms |

Essentially all of it is the one-time `CoverageAnalysis::new` construction of
abstract-value topologies and closed-world elision indexes over the whole
5,557-type schema — pre-existing shared infrastructure that Task 031 did not
change. The selected-closure work Task 031 added is ~46 ms at most.

This confirms the intended shape: one `CoverageAnalysis`, one baseline
snapshot, then indexed lookups. Readiness never calls `CoverageAnalysis::report`
or `impact`, and evaluates none of the 31 hypothetical feature combinations —
doing so would multiply the 46 ms figure by 31 per language, not the 244 s one.
On synthetic fixtures the whole command is effectively instantaneous.

### World semantics are inherited, not redefined

Under `ClosedSchemaSet`, Task 024 closed sums and Task 026 absent-only elision
apply, so an abstract structural value with known concrete descendants, and a
`0..1` field typed as a zero-descendant abstract target, can both be ready.
Under `OpenExtensions`, abstract structural *value* positions fail closed
exactly as Task 028 defines, so the same selections are not ready. Abstract
declarations used only as inheritance ancestry are unaffected by the world in
either direction. Contract-selected types are not special-cased anywhere.

## Contract-selected type generation

Task 032 turns a READY selection into source. The pipeline is:

```text
Service Contract (portable YAML/JSON)
  -> Contract IR              (ams-gra-oms-service-contract)
  -> ServicePlan              (codegen-core: what does the contract select?)
  -> ServiceBackendReadiness  (codegen-core: can backend X render it in world Y?)
  -> ServiceGenerationProjection
                              (codegen-core: narrow SchemaIr to that model)
  -> existing Ada/Rust/C++ backend, unchanged
  -> selected UCI type source
```

The projection step exists so the backends never learn what a Service Contract
is. `project_service_generation_schema` returns an **owned, projected
`SchemaIr`**, and the CLI hands that to the ordinary `Backend::generate`. From
the backend's point of view selected generation is indistinguishable from
ordinary generation, so validation, emission planning, helper detection, and
rendering are all reused rather than duplicated. No backend crate changed.

### Semantic closure versus generated support closure

`ServicePlan::selected_type_closure` is a **semantic** closure: base types,
aliases, Record fields, Choice alternatives, and List item types. It answers
"which declarations does this contract require?" and Task 032 does **not**
change it.

Generated code needs slightly more, because Task 024 lowers an abstract
structural value into a closed sum over its concrete transitive descendants,
and those descendants are not ordinary named dependencies of anything selected:

```text
abstract Base
ConcreteA extends Base
ConcreteB extends Base
Holder { Value : Base }
message HolderReport -> Holder

contract-selected types : Holder, Base
generated support types : ConcreteA, ConcreteB
```

Folding `ConcreteA`/`ConcreteB` into `selected_type_closure` would corrupt its
meaning — the contract selected neither — so Task 032 computes a **separate**
fixed-point support closure instead. Iteration to a fixed point is required,
not cosmetic: an admitted descendant may introduce new named dependencies and
new nested abstract values, each with descendants of their own.

Classification is reported explicitly, and support types are never described as
contract selections:

* a projected declaration is **selected** if and only if it is in the raw
  semantic closure;
* everything else is **support** — concrete closed-sum descendants, their
  dependencies, and abstract intermediates retained because a descendant's
  ancestry needs them (an abstract intermediate is *not* a wrapper variant).

Both lists use original `SchemaIr` declaration order, as does
`projected.types`. Contract presentation order lives in `ServicePlan` and
nowhere else, so reversing a contract's exchange order while selecting the same
messages produces byte-identical output.

### Readiness is the gate, and support closure is not an escape hatch

`service-generate` calls `analyze_service_readiness` first. A NOT READY
selection prints the same report `service-check` prints (from the same shared
helper), invokes no backend, creates no output directory, writes no file, and
exits 1. Support expansion only supplies representation dependencies for
already-ready selected values; it can never make an unsupported selected type
generate.

Two invariants are then asserted on the projection itself: the projected schema
must pass `SchemaIr::validate`, and it must pass the shared
`plan_type_emissions` planner. A subset missing one required named dependency
is a projection defect, and neither check is relaxed to make a subset "work".

### The demonstration

`tests/fixtures/service-generate/root.xsd` contains a supported selected
closure alongside unselected declarations that are unrenderable (`xs:duration`)
or helper-provoking (the set's only unbounded occurrence, binary value, and
floating value). For a contract selecting only the supported messages:

| command | result |
| --- | --- |
| `generate` (full schema) | **fails** on the unselected `xs:duration` |
| `service-check` | **READY** |
| `service-generate` | **succeeds**, and the output compiles |

The generated source contains the selected closure and none of the unrelated
declarations — including none of their helpers, because the backend only ever
saw the projected schema.

### World behavior

Under `ClosedSchemaSet` a selected abstract structural value generates as a
Task 024 closed sum over every concrete transitive descendant, and a Task 026
optional zero-known-descendant target keeps its declaration while its storage
is elided; no concrete support type is invented for it. Under `OpenExtensions`
a selected abstract structural *value* has no closed representation, so
readiness reports NOT READY and the projection independently fails closed —
it never emits a partial sum or silently behaves as if the world were closed.
A selection containing no abstract structural value projects and generates
identically in both worlds.

### Authoritative UCI 2.5 remains out of scope

A representative `service-generate` probe was run against the copied upstream
contract `crates/service-contract/tests/fixtures/upstream-minimal.yaml` and
`UCI_MessageDefinitions_v2_5_0.xsd`, for Rust under `closed-schema`. Measured:

| property | measured |
| --- | --- |
| selected OMS messages | 1 (`PositionReport`) |
| selected type closure | 60 |
| renderable selected types | 47 |
| status | **NOT READY** |
| first blocker | `{...oam}AltitudeType` |
| exit code | 1 |
| generated files | 0 |
| output directory | **not created** |
| elapsed | 254 s |

This reproduces Task 031's committed readiness evidence exactly, which is the
point: Task 032 changed no readiness semantics. The readiness section of the
report is byte-equivalent to what `service-check` prints, because both commands
render it from the same helper.

The no-write result is the important Task 032 property. Because readiness is
consulted before anything else, the projection and the backend never ran at
all, so there was no opportunity to leave a partial output tree — the output
directory does not exist.

Task 032 implements none of those blockers. `AltitudeType` and
`Acceleration3D_Type` support remains future work, and selected generation
deliberately does not route around the readiness verdict.

### Where the Task 032 time goes

The 254 s above is the pre-existing Task 031 readiness cost, dominated by
`CoverageAnalysis::new` (~244 s) over the whole 5,557-type schema. None of it
is selected projection or backend generation: in a NOT READY probe neither
executes. On the synthetic `service-generate` fixture — load, readiness,
projection, backend generation, and writing — the whole command completes in
about 1 ms, so the projection and generation stages Task 032 adds are not a
measurable cost at that scale.

## Non-goals for this slice

Not implemented, deliberately: Ada/Rust/C++ service wrappers, generated
publish/subscribe façades, a typed CAL API, OWP, WebSocket, JSON codec generation,
OMS profile validation, completion-assistant parsing, contract completion
logic, automatic Capability inference, automatic function grouping, automatic
topic generation, contract message-name qualification syntax, full-UCI
generation, and the schema-source manifest verification port. Task 029
overlays already provide the XSD bytes/topology path.

## Task 033 follow-up — constrained floating ranges

The Task 031 measurements above are preserved as the historical record of that
task. This section records the *re-measured* result after Task 033 added backend
lowering for named `Float32`/`Float64` numeric range restrictions.

Same authoritative inputs: UCI 2.5 root, contract
`crates/service-contract/tests/fixtures/upstream-minimal.yaml`, one selected
message (`PositionReport`), 60-declaration selected closure, closed world.

| Backend | World | Messages | Selected types | Status | First blocker | Elapsed |
| --- | --- | ---: | ---: | --- | --- | ---: |
| rust | closed-schema | 0/1 | **52/60** (was 47/60) | NOT READY | `{…}DateTimeType` (was `{…}AltitudeType`) | 258 s |
| ada | closed-schema | 0/1 | **37/60** (was 32/60) | NOT READY | `{…}Acceleration3D_Type` (unchanged) | 260 s |

C++ tracked Rust exactly in Task 031 and shares the identical shared-capability
model, the identical post-change full-schema coverage numbers, and passes the
same synthetic strict-C++17 generation and runtime tests. No separate
authoritative C++ count is asserted here beyond what was actually measured.

`AltitudeType` — the Task 031 Rust/C++ selected-service blocker — is renderable
in every backend now, and no other constrained floating type replaced it. The
new Rust first blocker is a temporal primitive, a different feature family that
Task 033 deliberately does not implement. Ada's first blocker is unchanged
because it was never a floating problem: `Acceleration3D_Type` carries an
ordinary optional named field that Ada's current occurrence model cannot
represent.

### No architectural change

This improvement required **no** change to `service_plan.rs`,
`service_readiness.rs`, or `service_generation.rs`. Contract selection is
unchanged, the selected closure is unchanged at 60 declarations, and the
selected-generation architecture is unchanged. Readiness improved purely because
the single shared renderability snapshot it already consults now answers
differently for bound-only floating declarations, and selected generation
improved because the projected schema already hands named declarations to the
ordinary backends.

A synthetic contract whose entire selected closure is bound-only constrained
floats (`tests/fixtures/service-generate/constrained-float.{xsd,yaml}`, which
also contains an unselected unrenderable `xs:duration`) now reports READY in all
three backends and generates output that compiles under `rustc`, strict C++17,
and GNAT, with the generated bound checks verified at runtime.

## Corrective cleanup: v0.1 parity and plan/schema binding

Retrospective corrections made after Task 033. **Historical behaviour** and
**current corrected behaviour** are distinguished explicitly below; the sections
above remain an accurate record of what each task did at the time.

### Portable v0.1 validation parity

The local Rust crate claims to implement portable v0.1 schema semantics, so it
must not accept a contract the published schema rejects merely because Serde can
deserialize it. Rules were re-read from the authoritative schema at
`ams-gra-service-contract` `schema/v0.1/service-contract.schema.json`, revision
`20a3315`.

| Assertion | Historical | Current |
|---|---|---|
| `functions.minItems = 1` | `functions: []` accepted | rejected as `NoFunctions` |
| `standards.uci_extension_schemas` `uniqueItems` | duplicate reached the CLI `--extension` mapping layer | rejected as `DuplicateExtensionSchema` on the contract itself |
| optional `minLength: 1` strings | only some enforced | all enforced, enumerated from the schema |
| timing `type: number` + `exclusiveMinimum: 0` | comparison-only, so `+Infinity` passed | must be finite **and** `> 0` |

Optional `minLength: 1` coverage is systematic rather than a list of examples:
`service.description`, `standards.ams_gra_version`, optional source
`document_number`/`revision`/`note`, traceability `locator`/`note`,
`function.description`, the optional OMS metadata fields
(`operational_attribute`, `subscription_group`, `appendix_c_mapping`), and the
`details`/`reference` pair on Special Signal, Security Exchange, and non-OMS
Message exchanges. Absence stays legal and is never defaulted; a *present* value
must carry meaningful text, so whitespace-only fails exactly as it already did
for required strings.

For timing, `NonFiniteTimingValue` is kept distinct from
`NonPositiveTimingValue` so the two remain attributable. `serde_yaml` accepts
`.inf` and `.nan`, so these are reachable inputs and are tested as such.
Rejecting a value is a *validity* statement only: upstream identifies the
nominal/max timing columns as informative, and this crate still does not turn
the surviving values into deadlines.

#### Policy on `format: date` and `format: uri`

The authoritative schema annotates `source.date` with `format: date` and
`source.uri` with `format: uri`. It declares
`$schema: https://json-schema.org/draft/2020-12/schema` and does **not** opt
into the format-assertion vocabulary, so under draft 2020-12 those keywords are
*annotations*, not assertions.

This validator therefore treats them as annotations too — deliberately, not by
omission. Adding ad-hoc URI or date parsing would make this implementation
*stricter* than the authority, rejecting contracts the published schema accepts,
which is the mirror image of the parity defect being corrected. Both fields are
still checked against the assertions the schema does make. The policy is pinned
by a test, so changing it cannot pass unnoticed.

### `ServicePlan` is semantically bound to its selected closure

**Historical behaviour.** The public library API accepts a `ServicePlan` and a
`SchemaIr` separately and explicitly claims to diagnose wrong-schema reuse, but
the checks compared qualified names only. A plan resolved against schema A could
be applied to schema B whenever both declared the same message and type *names*,
even though a payload `TypeRef`, a declaration body, or a dependency's
constraints or cardinality differed.

**Current corrected behaviour.** `resolve_service_plan` captures a private
semantic snapshot of exactly the declarations the plan depends on: the selected
message declarations and the selected transitive type closure.
`ServicePlan::verify_schema_binding` compares the current schema's declaration of
each selected identity against the captured one by **exact semantic equality**.

The representation is deliberate:

* not pointer identity — a caller may legitimately rebuild an equal schema and
  must not be punished for it;
* not `std::hash` or an ad-hoc digest — a collision would silently *accept* a
  mismatch;
* not a serialization — unstable, and this crate has no serializer.

Both `analyze_service_readiness` and `project_service_generation_schema` call
this one mechanism, first, before any other lookup, so the two cannot drift into
disagreeing notions of "wrong schema". A mismatch is a typed
`PlanBindingMismatch` rather than a debug assertion or a panic on a failed index.

Scope is the selected service: changing or removing a declaration **outside** the
selected closure does not invalidate the plan.
