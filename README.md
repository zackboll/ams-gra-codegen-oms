# ams-gra-codegen-oms

Schema-driven, multi-language code generation for OMS/UCI services that preserves the existing AMS GRA / OMS Language-Agnostic CAL (LA-CAL) runtime architecture.

> **Status:** the XSD frontend fully normalizes the authoritative UCI 2.5 and
> UCI 2.6 schema sets into semantic IR. This is not a claim of general-purpose
> complete XML Schema support. Ada, Rust, and C++ generation remains partial;
> see [backend compatibility](docs/backend-compatibility.md) for measured
> coverage and current limitations.

## Quick start

Validate a standalone XSD or the root of a local include/import graph:

```bash
cargo run -p ams-gra-codegen-oms -- \
  validate \
  --schema tests/fixtures/codegen-order/root.xsd
```

Generate Ada, Rust, or C++ source under an output directory:

```bash
cargo run -p ams-gra-codegen-oms -- \
  generate \
  --schema tests/fixtures/codegen-order/root.xsd \
  --language ada \
  --output generated/ada \
  --world closed-schema
```

Browse the normalized schema offline, without a language or world selection:

```bash
cargo run -p ams-gra-codegen-oms -- docs \
  --schema tests/fixtures/service-plan/root.xsd \
  --overlay tests/fixtures/service-plan/private-overlay.xsd \
  --output generated/uci-docs
```

Open `generated/uci-docs/index.html` locally; navigation and client-side search
work from `file://` with no server. See [Task 043](docs/task-043-uci-html-type-browser.md).

**UCI HTML Reference:** https://zackboll.github.io/ams-gra-codegen-oms/

The hosted reference offers both UCI 2.5 and UCI 2.6, built from exact pinned
authoritative revisions. No UCI schema source is vendored here. Build the same
Task 043 browser locally with the `docs` command above, or assemble the complete
site with `scripts/build-uci-pages.sh <empty-output-directory>`. See
[Task 045](docs/task-045-uci-pages.md).

`--world closed-schema` here is **your assertion** that the supplied schema set
is the complete value-type universe — it is not an inferred default. `generate`
and `coverage` have no default world and fail as a usage error without one; see
[`--world` is required for `generate` and `coverage`](#--world-is-required-for-generate-and-coverage).

Add known private derived types without editing the authoritative root document
by supplying them as repeatable additive overlays:

```bash
cargo run -p ams-gra-codegen-oms -- \
  generate \
  --schema public.xsd \
  --overlay private-extension.xsd \
  --language rust \
  --output generated \
  --world closed-schema
```

See [Additive schema overlays](#additive-schema-overlays).

The accepted language values are `ada`, `rust`, and `cpp`. Run
`ams-gra-codegen-oms --help` or a command followed by `--help` for complete
usage. Exit status `0` means success, `2` means invalid command-line usage, and
`1` means schema, code-generation, or filesystem execution failed.

## Why this repository exists

The current public AMS GRA Hello World Starter Kit demonstrates OMS Service integration by connecting directly to Sleet's LA-CAL endpoint with WebSockets, sending OMS WebSocket Protocol (OWP) operations such as `INIT`, `SUB`, and `PUB`, and manually constructing/parsing UCI JSON payloads. That is an excellent interoperability baseline because any language that can speak WebSocket + JSON can participate, but it leaves each language ecosystem to recreate a large amount of repetitive plumbing.

As of **2026-09-16**, we did **not** find a shared, general-purpose code generator in the public AMS GRA Hello World Starter Kit that accepts the UCI XSD schema set and emits strongly typed LA-CAL bindings for Ada/SPARK, Rust, C++, and Python. The Starter Kit documentation instead shows language-agnostic pseudocode that manually constructs OWP frames and JSON. This project is intended to fill that tooling gap.

That statement is deliberately narrow:

- it does **not** mean OMS is hostile to code generation;
- it does **not** mean no OMS implementation generates code;
- the current Rust CAL ecosystem (`rcal`) already demonstrates that UCI types can be generated from XSD;
- OMS also has language-specific CAL interface generation specifications.

The missing capability this project targets is a **shared, language-neutral generation pipeline** suitable for multiple AMS GRA language ecosystems.

## Core architectural decision

We keep the OMS runtime architecture intact and add a build-time generation layer:

```text
                         UCI / OMS XSD
                              |
                              v
                    +-------------------+
                    |   XSD frontend    |
                    +---------+---------+
                              |
                              v
                    +-------------------+
                    | language-neutral  |
                    |    schema IR      |
                    +---------+---------+
                              |
                              v
                    +-------------------+
                    | shared structural |
                    | projection/analysis|
                    +---------+---------+
                              |
          +-------------------+-------------------+
          |                   |                   |
          v                   v                   v
     Ada/SPARK             Rust                 C++
      backend              backend              backend
          |                   |                   |
          v                   v                   v
     generated            generated            generated
     types/codecs         types/codecs         types/codecs
     typed CAL API        typed CAL API        typed CAL API
          |                   |                   |
          +-------------------+-------------------+
                              |
                 small handwritten runtimes
                              |
                       OWP / WebSocket
                       JSON on the wire
                              |
                              v
                            Sleet
                              |
                           LA-CAL
                              |
                             ASB
```

The generator is therefore **not a replacement for Sleet, LA-CAL, OWP, WebSockets, JSON, UCI, or the OMS CAL boundary**. It is a developer-tooling layer above them.

## What stays exactly the same

### Sleet remains the LA-CAL server

Services still connect to the Sleet LA-CAL endpoint. Existing Sleet deployments do not need to understand whether a client was handwritten or generated.

### WebSocket remains the transport exposed to LA-CAL clients

The generated client APIs ultimately use the same WebSocket endpoint and lifecycle as a manually written LA-CAL client.

### OWP remains the protocol

Operations such as `INIT`, `SUB`, `PUB`, message delivery, unsubscribe, errors, and connection lifecycle remain OWP concerns. The generated API hides string construction from application code; it does not invent a replacement protocol.

### JSON remains the LA-CAL payload representation

Generated codecs produce and consume the same OMS/UCI JSON expected by the existing LA-CAL implementation. There is no new binary wire format and no DDS dependency in the runtime path.

### UCI XSD remains authoritative

This project does not create a competing IDL. The UCI/OMS schema set remains the source of truth. The IR is a compiler-internal normalized representation derived from that authoritative schema.

### The CAL abstraction boundary remains intact

OMS Services continue to depend on the CAL abstraction rather than directly depending on an ASB transport implementation. This project improves how application code reaches that boundary; it does not move the boundary.

### Existing manual clients remain compatible

Generated and handwritten clients can coexist. No Sleet-side migration is required.

### MEL and high-rate sensor-data paths are unaffected

This repository targets OMS/UCI CAL messaging. It does not replace AMS GRA MEL interfaces or attempt to move raw/high-bandwidth sensor traffic into LA-CAL.

## What changes

The change is in **developer experience and build-time assurance**.

Instead of application code doing this conceptually:

```text
open websocket
format INIT string
format SUB string
construct JSON object with schema field names
serialize JSON
format PUB string
parse MSG string
parse JSON
manually validate expected fields
```

application code should eventually look like this:

```ada
Publisher.Publish
  ((Latitude  => ...,
    Longitude => ...,
    Altitude  => ...));
```

or:

```rust
publisher.publish(PositionReport { /* typed fields */ }).await?;
```

The generator is responsible for producing the repetitive schema-specific pieces:

- language-native UCI data types;
- enumerations and constrained scalar wrappers;
- optional/repeated-field representations;
- schema-derived validation;
- JSON serialization/deserialization glue;
- UCI qualified-name and message metadata;
- typed publish/subscribe wrappers;
- deterministic generated-file manifests;
- source/schema version provenance;
- eventually service-contract helpers and compatibility reports.

A small handwritten runtime per language remains responsible for the stable protocol machinery:

- WebSocket connection management;
- OWP framing and state machine;
- initialization/handshake;
- subscription lifecycle;
- publish/receive dispatch;
- error mapping;
- reconnect/backoff policy where appropriate;
- TLS/authentication integration where required.

This separation keeps schema churn out of the runtime and keeps transport/protocol code out of generated files.

## Why a language-neutral schema IR

A direct `XSD -> Ada`, `XSD -> Rust`, and `XSD -> C++` design would create three independent schema interpreters. They would inevitably disagree about edge cases such as namespace resolution, inheritance, anonymous types, `choice`, cardinality, restrictions, and schema evolution.

Instead, this repository introduces one semantic normalization pipeline:

```text
XSD documents
    |
    v
parse XML/XSD syntax
    |
    v
resolve imports/includes/QNames
    |
    v
normalize XSD semantics
    |
    v
classify OMS/UCI message roles
    |
    v
validate invariants
    |
    v
Schema IR
    |
    +----> Ada/SPARK backend
    +----> Rust backend
    +----> C++ backend
    +----> Python backend (planned)
    +----> offline HTML type browser (docs)
    +----> schema-diff / lint tools (planned)
```

The IR is **not another public interface standard** and is **not serialized onto the mission network**. It is an internal compiler representation.

The important rule is:

> XSD-specific complexity is resolved once, before any language backend runs.

Backends should not need to understand XML Schema syntax. They should consume normalized semantic concepts such as:

- qualified names and namespaces;
- scalar aliases;
- enumerations;
- records/complex types;
- inheritance/extension;
- choices/unions;
- field cardinality;
- optionality and nillability;
- numeric/string/list restrictions;
- publishable-message classification;
- documentation and source provenance.

See [docs/ir.md](docs/ir.md) for the initial IR model.

## Why not DDS IDL?

DDS inspired the developer experience we want: define types once, generate strongly typed language APIs, and keep serialization details out of application code.

But replacing the OMS/UCI model with DDS IDL would introduce a second authoritative type system and could bypass the CAL/ASB abstraction that AMS GRA/OMS intentionally preserves.

The goal here is therefore:

```text
DDS lesson:       schema -> generated typed APIs
OMS implementation: UCI XSD -> generated typed LA-CAL APIs
```

not:

```text
UCI XSD -> convert to DDS IDL -> make DDS the application API
```

A DDS-backed ASB could still be explored independently in the future without changing generated application APIs, because the CAL boundary remains preserved.

## Repository layout

```text
ams-gra-codegen-oms/
├── crates/
│   ├── ir/                # Language-neutral semantic schema model
│   ├── xsd-frontend/      # XSD loading, resolution, normalization into IR
│   ├── codegen-core/      # Backend contract, generated-file model, Service Plan
│   ├── service-contract/  # Portable Service Contract v0.1 parsing and IR
│   ├── backend-ada/       # Ada/SPARK generator
│   ├── backend-rust/      # Rust generator
│   ├── backend-cpp/       # C++ generator
│   └── cli/               # ams-gra-codegen-oms command-line driver
├── docs/
│   ├── architecture.md
│   ├── ir.md
│   ├── compatibility.md
│   ├── service-contract-integration.md
│   ├── roadmap.md
│   ├── references.md
│   └── adr/
├── examples/
│   └── minimal/
└── .github/workflows/
```

## Proposed generated/runtime split

The long-term product should have two kinds of artifacts.

### Generated, schema-version-specific artifacts

Regenerated whenever the authoritative UCI schema changes:

```text
UCI 2.5 schema
   |
   +--> generated Ada/SPARK UCI model
   +--> generated Rust UCI model
   +--> generated C++ UCI model
```

### Small, stable handwritten runtimes

Changed only when OWP/LA-CAL behavior changes:

```text
oms_ada runtime
oms_rust runtime
oms_cpp runtime
```

The generator backends target the public contracts of those runtimes. This avoids regenerating or duplicating WebSocket/OWP machinery for every UCI message type.

## SPARK-specific objective

The Ada backend should be designed so that as much generated schema logic as practical can be used in SPARK:

```text
mission algorithm
      |
 generated SPARK-friendly UCI types
      |
 generated validation/contracts
      |
 ================================
 trusted / non-SPARK boundary
 ================================
      |
 Ada LA-CAL runtime
 WebSocket / TLS / JSON machinery
      |
    Sleet
```

The goal is not to pretend the network stack is formally verified. The goal is to make the mission-domain data model, constraints, and application logic easier to reason about while keeping the unavoidable runtime boundary explicit.

## Command-line interface

The CLI exposes schema-set validation and source generation:

```text
ams-gra-codegen-oms validate \
  --schema /path/to/root.xsd

ams-gra-codegen-oms generate \
  --schema /path/to/root.xsd \
  --language ada \
  --output generated/ada \
  --world closed-schema

ams-gra-codegen-oms generate \
  --schema /path/to/root.xsd \
  --language rust \
  --output generated/rust \
  --world open-extensions

ams-gra-codegen-oms coverage \
  --schema /path/to/root.xsd \
  --world closed-schema

ams-gra-codegen-oms service-plan \
  --schema /path/to/root.xsd \
  --contract /path/to/service.yaml

ams-gra-codegen-oms service-check \
  --schema /path/to/root.xsd \
  --contract /path/to/service.yaml \
  --language rust \
  --world closed-schema

ams-gra-codegen-oms service-generate \
  --schema /path/to/root.xsd \
  --contract /path/to/service.yaml \
  --language rust \
  --world closed-schema \
  --output generated/service

ams-gra-codegen-oms generate \
  --schema /path/to/public-root.xsd \
  --overlay /path/to/private-extension.xsd \
  --language cpp \
  --output generated/cpp \
  --world closed-schema
```

Generation completes schema loading, validation, backend generation, and
generated-path validation before writing any file. Existing generated files may
be overwritten; unrelated files are retained. Files written before a later I/O
failure are not rolled back.

### `--world` is required for `generate` and `coverage`

A complete schema *file* set does not prove a complete *type* universe: XSD
derivation is open by default, so an instance may select an externally declared
derived type. The generator cannot decide that from schema content, and it
refuses to guess — so `generate` and `coverage` require an explicit `--world`,
with **no default**. Omitting it is a usage error (exit code 2).

- `--world closed-schema` — you **assert** that the supplied schema set contains
  every concrete type that may legally inhabit an abstract value. This is a
  claim about your deployment, not something schema loading can verify. Under
  it, known concrete descendants of an abstract value are treated as exhaustive
  and lowered to a closed sum, and an abstract value with zero known descendants
  is treated as genuinely uninhabited.
- `--world open-extensions` — external or private derived types may exist
  outside the supplied schema set. Known descendant sets are never assumed
  exhaustive, so every abstract structural **value** position fails closed with
  a diagnostic instead of being represented by a sum that might be incomplete.
  No placeholder value is invented. Abstract types used only as inheritance
  ancestry are unaffected, and ordinary concrete values generate identically.

If you need open-extension types to actually generate, the supported route today
is to add the private derived-type schema to the generation schema set (same
target namespace) and use `--world closed-schema`. Since Task 029 that no longer
requires editing the root document: see
[Additive schema overlays](#additive-schema-overlays) and
`docs/adr/0004-open-extension-points.md`. Runtime-polymorphic open extensions
are not implemented.

`validate` takes no `--world`: schema validity is independent of generation
policy.

### Additive schema overlays

`validate`, `coverage`, and `generate` accept a repeatable `--overlay PATH`:
an additional top-level schema document loaded into the same normalized schema
set as `--schema`.

```text
ams-gra-codegen-oms generate \
  --schema public.xsd \
  --overlay private-extension.xsd \
  --language rust \
  --output generated \
  --world closed-schema
```

- **Repeatable.** Pass `--overlay` as many times as needed. Overlays are applied
  in command-line order, which is the deterministic composition input; they are
  never sorted by filesystem path, because absolute paths vary by machine.
- **Additive only.** An overlay may add declarations and may derive from types
  declared in the root or in an earlier overlay. It may not replace, override,
  mutate, or remove an existing declaration.
- **Same target namespace.** Every top-level overlay must declare the same
  `targetNamespace` as the root, or loading fails with an explicit overlay
  namespace-mismatch diagnostic. An accepted overlay's own `xs:include` and
  `xs:import` dependencies then follow the ordinary loader rules.
- **Duplicates are errors.** A qualified name declared twice — root versus
  overlay, or overlay versus overlay — fails existing duplicate validation.
  There is no precedence and no "last overlay wins".
- **The root stays authoritative** for schema version, namespace presentation
  metadata, and initial declaration order. Files reachable through more than one
  route are loaded once, by canonical path.
- **Overlays never choose a world.** Supplying a private descendant does not
  select `closed-schema` or relax `open-extensions`; under `open-extensions`,
  known descendants still are not treated as exhaustive.

This is the current preferred mechanism for supplying known private extension
types against a pinned public root such as the authoritative UCI schema. It is
build-time schema composition, **not** a runtime extension registry: no runtime
type registration, unknown-subtype representation, or `xsi:type` dispatch is
implemented.

### Service Contract planning

A portable AMS GRA Service Contract is a first-class generator input. The
read-only `service-plan` command joins one to a normalized schema set:

```text
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

- **The contract owns interface semantics; the XSD owns type identity.** The
  contract states which functions and exchanges exist, with direction,
  mandate, topic, and timing. The schema states which messages exist, their
  qualified names, payload types, and transitive type graphs. The Resolved
  Service Plan owns only the join.
- **Exact message resolution.** Contract message names resolve by exact
  local-name equality. Zero matches fail; more than one fails as ambiguous
  with the candidate qualified names listed. There is no case folding, affix
  matching, fuzzy matching, or first-match-wins.
- **Non-UCI exchanges are preserved.** `data_transfer`, `special_signal`,
  `security_exchange`, and `non_oms_message` are kept verbatim, never resolved
  against the schema, and never a cause of failure.
- **Contract order is preserved.** Functions and exchanges keep author order.
  `ServicePlan::selected_messages()` separately exposes the deduplicated
  message selection in first-occurrence order.
- **Writes nothing, and takes no `--world`.** Contract validity and
  message/type resolution are independent of the `closed-schema` /
  `open-extensions` policy.

Contract `standards.uci_extension_schemas` entries are logical **identifiers**,
not paths, so each is mapped explicitly:

```text
ams-gra-codegen-oms service-plan \
  --schema public.xsd \
  --contract service.yaml \
  --extension ext-a-1.0=/path/private-a.xsd \
  --extension ext-b-2.0=/path/private-b.xsd
```

The supplied set must match the declared set exactly — a missing mapping, an
undeclared mapping, and a duplicate identifier all fail — and the Task 029
overlay loader receives the paths in **contract** declaration order, not
command-line order.

### Service Contract backend readiness

`service-plan` answers **what** a contract selects. `service-check` answers
**whether** one backend, under one generation world, can render that selected
UCI type model:

```text
ams-gra-codegen-oms service-check \
  --schema root.xsd \
  --contract service.yaml \
  --language rust \
  --world closed-schema
```

```text
service contract valid
service: Readiness Ready
language: rust
generation world: closed-schema

selected oms messages: 1
renderable selected oms messages: 1
selected type closure: 1
renderable selected types: 1
status: READY
```

- **Requires `--language` and `--world`, with no default.** Readiness is
  backend- and world-specific, so `service-check` requires both — exactly the
  options `service-plan` refuses, because planning is neither.
- **Only the selected closure is measured.** An unrenderable UCI declaration
  the contract does not select does not make the service unready. A contract
  can be READY against a schema set whose full-schema `generate` fails; that
  asymmetry is the reason contract-selected generation is worth building.
- **Non-UCI exchanges require no UCI type model.** A contract with zero OMS
  Message exchanges is vacuously ready.
- **Deterministic blockers.** Unsupported selected types are listed in schema
  declaration order with qualified names; blocked selected messages are listed
  in contract first-occurrence order, each with one first blocker. A message
  selected by several exchanges is reported once.
- **Current capability only.** No hypothetical feature family is enabled, so
  there is no "ready if X were implemented" verdict.
- **Same extension mapping.** `--extension ID=PATH` behaves exactly as in
  `service-plan`; there is no raw `--overlay`.
- **Exit status.** `0` READY, `1` NOT READY (the full report is still written
  to stdout), `2` usage error — usable directly as a CI gate.

Neither command writes generated code. READY means a backend *could* render the
selection, not that any service source has been generated yet.

### Contract-selected type generation

The three contract commands answer three separate questions:

| Command | Question |
| --- | --- |
| `service-plan` | **what** the contract selects |
| `service-check` | **whether** that selection is renderable today |
| `service-generate` | **emit** the selected UCI type model, after readiness succeeds |

```text
ams-gra-codegen-oms service-generate \
  --schema root.xsd \
  --contract service.yaml \
  --language rust \
  --world closed-schema \
  --output generated/service
```

```text
service contract valid
service: Selected Generation Both
language: rust
generation world: closed-schema

selected oms messages: 2
contract-selected types: 5
generated support types: 0
projected schema types: 5
generated 1 file(s)
output: generated/service
```

- **Readiness is the gate.** Task 031 readiness runs first. If the selection is
  NOT READY the same report `service-check` prints goes to stdout, no backend
  is invoked, no output directory is created, no file is written, and the
  command exits 1.
- **Only the selection is emitted.** Unrelated schema declarations are absent,
  so a schema set whose full-schema `generate` fails can still generate a
  contract's selected model. Helper emission follows the projected model too:
  an unselected unbounded field contributes no unbounded sequence helper.
- **Contract-selected types and generated support types are reported
  separately.** The contract selects the former. The latter exist only because
  generated representation needs them — most importantly the Task 024
  closed-sum concrete descendants of a selected abstract structural value,
  which the contract never named. Support types are never presented as
  contract selections.
- **Type order follows the schema, not the contract.** Reordering a contract's
  exchanges while selecting the same messages produces byte-identical output.
- **Same extension mapping.** `--extension ID=PATH` and contract-ordered
  overlay composition behave exactly as in `service-plan`; there is no raw
  `--overlay`.
- **Exit status.** `0` generated, `1` NOT READY / projection / filesystem
  error, `2` usage error.

**No CAL or service wrapper is emitted yet.** `service-generate` produces the
selected UCI *type model* only: no publisher/subscriber façade, typed CAL API,
codec, or runtime source.

Compatibility is defined by `contract_version`, not by repository SHA. The
baseline is `zackboll/ams-gra-service-contract`
`4ea5be8dd36e9695bd58f2c36c6b3dd8075de249`, `contract_version: 0.1`. OMS
profile conformance and contract completion remain owned by that project: no
profile engine was ported, and nothing here infers Capabilities, groups
functions, generates topics, or regenerates Section 3.3 topology.

See `docs/service-contract-integration.md` and
`docs/adr/0005-service-contract-codegen-plan.md`.

## First implementation milestone

The first meaningful vertical slice should be deliberately small:

1. load a controlled XSD fixture;
2. resolve namespaces and named types;
3. normalize enums, scalar restrictions, records, optional fields, and bounded sequences into IR;
4. validate IR invariants;
5. generate one equivalent message model in Ada, Rust, and C++;
6. round-trip the same JSON fixture through each generated language implementation;
7. verify that a generated client still interoperates with the unmodified Sleet LA-CAL endpoint.

Only after that slice is stable should support expand to the full UCI schema surface.

## Non-goals

This project does **not** initially aim to:

- replace Sleet;
- replace LA-CAL;
- define a new ASB;
- replace OWP with a new protocol;
- replace JSON with a binary serialization;
- replace UCI XSD with a new IDL;
- generate AMS GRA MEL interfaces;
- make a claim of OMS compliance solely because code was generated;
- vendor the authoritative UCI/OMS standards into this repository;
- reimplement the Service Contract project's OMS profile engine or
  completion-assistant tooling. A portable contract is consumed here, never
  authored, completed, or inferred.

## Source provenance and standards handling

The generator should consume an externally supplied, versioned UCI/OMS schema tree rather than silently embedding a private copy. Every generated output should record:

- generator version/commit;
- schema version;
- schema-set digest;
- backend version;
- generation options.

This makes generated code auditable and reproducible.

## References

See [docs/references.md](docs/references.md). The key public references used for this initial architecture are:

- AMS GRA Hello World Starter Kit — Overview and Terminology
- AMS GRA Hello World Starter Kit — How to Build an OMS Service
- UCI Standard public repository
- OMS Standard public repository
- Rust `rcal` documentation showing XSD-generated UCI types

## License

Apache-2.0. See [LICENSE](LICENSE).
