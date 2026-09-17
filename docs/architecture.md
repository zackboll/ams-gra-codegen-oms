# Architecture

## 1. Architectural intent

`ams-gra-codegen-oms` is a compiler/tooling project that sits **above** the OMS LA-CAL runtime boundary. Its purpose is to remove repetitive, error-prone schema and protocol glue from application code without changing the externally visible OMS behavior of the mission system.

The repository deliberately separates four concerns:

1. **Authoritative schema input** — UCI/OMS XSD documents.
2. **Semantic normalization** — one language-neutral schema IR.
3. **Language generation** — Ada/SPARK, Rust, C++, and later Python backends.
4. **Runtime transport/protocol** — small handwritten language runtimes that speak the existing OWP/WebSocket/JSON interface to Sleet.

## 2. Before this project

The public AMS GRA Starter Kit presents LA-CAL as a language-agnostic interface. A service can open a WebSocket to Sleet, send an OWP `INIT`, send `SUB`/`PUB` operations, and encode/decode UCI JSON itself.

Conceptually:

```text
Application
   |
   | manually constructs/parses
   v
OWP strings + UCI JSON
   |
WebSocket
   |
Sleet / LA-CAL
   |
ASB
```

This is portable, but each language has to recreate:

- UCI type models;
- schema constraints;
- JSON field mapping;
- message metadata;
- OWP frame construction/parsing;
- publish/subscribe wrappers;
- schema-version handling.

The public Starter Kit documentation does not currently present a common multi-language generator that owns those tasks.

## 3. After this project

```mermaid
flowchart TD
    XSD[Authoritative UCI / OMS XSD] --> FE[XSD frontend]
    FE --> IR[Language-neutral schema IR]
    IR --> ADA[Ada/SPARK backend]
    IR --> RUST[Rust backend]
    IR --> CPP[C++ backend]
    IR -. planned .-> PY[Python backend]

    ADA --> AG[Generated Ada/SPARK schema bindings]
    RUST --> RG[Generated Rust schema bindings]
    CPP --> CG[Generated C++ schema bindings]

    AG --> AR[Ada LA-CAL runtime]
    RG --> RR[Rust LA-CAL runtime]
    CG --> CR[C++ LA-CAL runtime]

    AR --> OWP[OWP + JSON over WebSocket]
    RR --> OWP
    CR --> OWP
    OWP --> SLEET[Sleet LA-CAL]
    SLEET --> ASB[Abstract Service Bus]
```

## 4. Runtime compatibility boundary

The code generator must preserve the existing observable LA-CAL contract.

### Unchanged

- Sleet remains the LA-CAL implementation used by the Starter Kit.
- WebSocket remains the language-agnostic transport exposed to clients.
- OWP remains the framing/operation protocol.
- JSON remains the UCI representation used on this interface.
- UCI remains the authoritative message model.
- CAL remains the application-facing abstraction boundary above the ASB.
- Service registration and allowed topic/message policy remain deployment concerns.
- Existing handwritten OWP clients remain valid.

### Added

- build-time schema parsing and normalization;
- generated language-native types;
- generated validation;
- generated JSON codecs/mappings;
- generated message descriptors/qualified names;
- typed publish/subscribe facades;
- reproducible generation manifests;
- optional schema compatibility/diff tooling.

No server-side Sleet change is required for the initial architecture.

## 5. Compiler pipeline

```text
          XSD documents
               |
               v
       +----------------+
       | loader         |
       | includes       |
       | imports        |
       +-------+--------+
               |
               v
       +----------------+
       | XSD parser     |
       | syntax model   |
       +-------+--------+
               |
               v
       +----------------+
       | name resolver  |
       | QName/ns graph |
       +-------+--------+
               |
               v
       +----------------+
       | normalizer     |
       | restrictions   |
       | inheritance    |
       | cardinality    |
       | choice/groups  |
       +-------+--------+
               |
               v
       +----------------+
       | OMS/UCI        |
       | classifier     |
       +-------+--------+
               |
               v
       +----------------+
       | IR validator   |
       +-------+--------+
               |
               v
       +----------------+
       | Schema IR      |
       +-------+--------+
               |
       +-------+---------+---------+
       |                 |         |
       v                 v         v
      Ada               Rust      C++
```

The crucial dependency direction is:

```text
backend -> IR
```

and never:

```text
backend -> raw XML/XSD DOM
```

If a backend needs to understand `xs:extension`, `xs:choice`, namespace prefixes, import paths, or anonymous XSD naming rules, the frontend/IR boundary is too weak.

## 6. Why normalize before generation

XSD is a serialization/schema language, not an ideal code-generation IR. The same semantic type can be expressed in multiple XSD forms. Language backends should not each re-interpret those forms.

Normalization provides one answer for:

- QName resolution;
- include/import closure;
- anonymous type identity;
- extension/restriction relationships;
- inherited fields;
- `minOccurs`/`maxOccurs`;
- `nillable` versus absent;
- enum and scalar restrictions;
- pattern/length/range facets;
- sequence/choice composition;
- top-level publishable message identification;
- stable ordering and identifiers.

This substantially reduces backend complexity and makes cross-language equivalence testable.

## 7. Generated code versus runtime code

### Generated code

Schema-specific and regenerated when UCI changes:

- types;
- enums;
- containers;
- validation;
- schema metadata;
- JSON mappings;
- typed topic/publish/subscribe helpers.

### Handwritten runtime

Protocol-specific and relatively stable:

- WebSocket lifecycle;
- OWP state machine;
- connection/init;
- subscription handles;
- message dispatch;
- error and reconnect behavior;
- TLS/authentication hooks.

This line is important because it keeps generated output deterministic and reviewable while allowing runtime code to be tested like normal infrastructure software.

## 8. Ada/SPARK architecture

The Ada backend should distinguish pure/schema-domain code from the external runtime boundary.

```text
+------------------------------------------+
| SPARK-friendly generated domain         |
|                                          |
| UCI types                                |
| constraints / predicates                 |
| deterministic validation                 |
| message metadata                         |
+----------------------+-------------------+
                       |
=======================|==================== trusted boundary
                       |
+----------------------v-------------------+
| Ada runtime                              |
| JSON codec glue where needed             |
| WebSocket / TLS                          |
| OWP connection state                     |
+----------------------+-------------------+
                       |
                     Sleet
```

Not every generated representation must be provable from day one. The backend should, however, avoid choices that unnecessarily prevent later SPARK use.

## 9. Cross-language equivalence

A schema generator is useful only if its language outputs agree semantically.

The test strategy should therefore include a canonical set of schema fixtures and JSON vectors:

```text
fixture.xsd
   |
   v
same Schema IR
   |
   +--> Ada generated code ----+
   +--> Rust generated code ---+--> identical accepted/rejected vectors
   +--> C++ generated code ----+
```

For every supported schema feature, test:

- valid minimum representation;
- valid maximum/boundary representation;
- missing required fields;
- out-of-range scalar values;
- invalid enum values;
- empty/nonempty bounded sequences;
- each `choice` branch;
- inherited/extended types;
- unknown fields according to the selected JSON policy.

## 10. Versioning model

The generator, IR, runtime, and UCI schema version independently.

Generated artifacts should record all four dimensions:

```text
generator_commit = ...
ir_version        = ...
backend_version   = ...
uci_schema        = 002.5.0
schema_digest     = sha256:...
runtime_api       = 1
```

This avoids ambiguous generated sources and makes CI reproduction possible.

## 11. Why the IR should not become a new standard

The IR is intentionally internal. Publishing it as a competing mission-system schema would create another compatibility surface and undermine the goal of keeping UCI authoritative.

It may be serialized for:

- debugging;
- golden tests;
- schema diffs;
- cache artifacts;
- generator reproducibility.

But it is not a wire format and is not an application-level IDL.
