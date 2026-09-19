# Roadmap

## Phase 0 — Repository bootstrap

- [x] Document architecture and boundaries.
- [x] Define initial language-neutral IR crate.
- [x] Define frontend/backend crate boundaries.
- [x] Add Ada, Rust, and C++ backend stubs.
- [x] Add CI scaffolding.
- [x] Select/validate XSD parsing strategy against the real UCI schema set.

## Phase 1 — Minimal vertical slice

Target only a controlled schema fixture containing:

- [x] namespace/import resolution for controlled local schema sets;
- scalar aliases;
- enumerations;
- simple restrictions;
- record/complex types;
- required/optional fields;
- bounded repeated fields.

Deliver:

- XSD -> IR;
- [x] IR validator;
- [x] validation and Ada/Rust/C++ generation CLI;
- Ada output;
- Rust output;
- C++ output;
- canonical JSON vectors;
- deterministic golden tests.

## Phase 2 — UCI-relevant XSD coverage

Add, driven by real schema evidence rather than speculative completeness:

- extension/restriction chains;
- anonymous complex/simple types;
- groups and attribute groups if required;
- `xs:choice`;
- abstract types;
- substitution/polymorphism patterns if present;
- list/union constructs if present;
- nillability distinctions;
- pattern/range/length facets;
- documentation annotations;
- publishable-message classification.

Every newly supported feature receives an IR fixture and equivalent backend tests.

### Open extension-point representation / externally supplied derived types

Not started. Task 027 established from the pinned UCI 2.5/2.6 schemas that the
13 zero-known-descendant abstract types (`SourceCommandEXT`, `ConstraintEXT`,
`OpNotificationEXT`, and the ten `CommSupport*EXT` types) are documented **open
extension points**, not uninhabited values: their concrete descendants are
supplied by schemas outside the open, unclassified UCI schema set. See
`docs/task-027-open-extension-points.md`.

Scope of the eventual work:

- make the generator's world model explicit (closed schema set versus open
  extensions) as a language-neutral `codegen-core` policy, so Task 026's
  optional-value elision is attributable to a declared assumption;
- support externally supplied derived extension types, preferentially by
  accepting private extension schemas into the schema set so existing closed-sum
  lowering applies unchanged;
- decide the representation for extension values whose derived types are *not*
  known at generation time; this depends on Phase 3 runtime/codec work and must
  not be decided before it.

Until then `SourceCommandEXT`'s `0..unbounded` occurrence stays fail-closed. No
name-based (`EXT` suffix) heuristic may be used; the schema contains no reliable
machine-readable discriminator for extension points.

## Phase 3 — Typed LA-CAL integration

Define the stable runtime contract used by generated code.

Expected conceptual operations:

```text
connect
initialize
subscribe<T>
unsubscribe
publish<T>
receive/dispatch<T>
close
```

Implement or integrate small runtimes for:

- Ada;
- Rust;
- C++.

Validate each against unmodified Sleet.

## Phase 4 — SPARK-oriented Ada backend

- SPARK-friendly generated data model where practical;
- generated schema predicates/validation;
- explicit trusted boundary around JSON/WebSocket runtime;
- bounded-container strategy;
- proof-oriented examples;
- GNATprove CI for generated fixture code.

## Phase 5 — Full UCI schema generation

- consume a pinned public UCI schema release;
- generate all supported model types;
- generate provenance manifest;
- compile all language outputs in CI;
- run shared validation vectors;
- quantify unsupported constructs explicitly.

## Phase 6 — Developer tooling

- schema diff / compatibility report;
- generated API documentation;
- service-contract helper generation;
- topic/message allow-list helpers;
- cached IR artifact;
- editor/IDE schema navigation;
- Python backend if useful.

## Phase 7 — Reproducibility and supply-chain hardening

- deterministic generation;
- schema digest verification;
- generated-file headers with source provenance;
- SBOM for generator releases;
- signed release artifacts;
- hermetic/air-gapped generation workflow.
