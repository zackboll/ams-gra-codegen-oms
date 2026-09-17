# ams-gra-codegen-oms

Schema-driven, multi-language code generation for OMS/UCI services that preserves the existing AMS GRA / OMS Language-Agnostic CAL (LA-CAL) runtime architecture.

> **Status:** architecture/bootstrap phase. The repository intentionally starts with the schema IR, generator boundaries, and backend contracts before implementing the complete UCI XSD frontend.

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
    +----> schema-diff / docs / lint tools (planned)
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
│   ├── codegen-core/      # Backend contract and generated-file model
│   ├── backend-ada/       # Ada/SPARK generator
│   ├── backend-rust/      # Rust generator
│   ├── backend-cpp/       # C++ generator
│   └── cli/               # ams-gra-codegen-oms command-line driver
├── docs/
│   ├── architecture.md
│   ├── ir.md
│   ├── compatibility.md
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

## Initial command-line direction

The CLI is scaffolded but intentionally not feature-complete. The intended interface is:

```text
ams-gra-codegen-oms parse \
  --schema-root /path/to/uci/xsd \
  --emit-ir build/uci-ir.json

ams-gra-codegen-oms generate \
  --schema-root /path/to/uci/xsd \
  --language ada \
  --out generated/ada

ams-gra-codegen-oms generate \
  --schema-root /path/to/uci/xsd \
  --language rust \
  --out generated/rust

ams-gra-codegen-oms diff \
  --old-schema /path/to/uci-2.5 \
  --new-schema /path/to/uci-next
```

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
- vendor the authoritative UCI/OMS standards into this repository.

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
