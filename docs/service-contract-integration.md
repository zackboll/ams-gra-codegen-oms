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

## Non-goals for this slice

Not implemented, deliberately: Ada/Rust/C++ service wrappers, contract-selected
backend generation, a typed CAL API, OWP, WebSocket, JSON codec generation,
OMS profile validation, completion-assistant parsing, contract completion
logic, automatic Capability inference, automatic function grouping, automatic
topic generation, contract message-name qualification syntax, full-UCI
generation, and the schema-source manifest verification port. Task 029
overlays already provide the XSD bytes/topology path.
