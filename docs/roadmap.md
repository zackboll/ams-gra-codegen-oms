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

Partially complete. Task 027 established from the pinned UCI 2.5/2.6 schemas that the
13 zero-known-descendant abstract types (`SourceCommandEXT`, `ConstraintEXT`,
`OpNotificationEXT`, and the ten `CommSupport*EXT` types) are documented **open
extension points**, not uninhabited values: their concrete descendants are
supplied by schemas outside the open, unclassified UCI schema set. See
`docs/task-027-open-extension-points.md`.

**Complete (Task 028):**

- the generator's world model is explicit: a language-neutral
  `codegen-core::GenerationWorld` policy with `ClosedSchemaSet` and
  `OpenExtensions`, surfaced as a **required** `--world` option on `generate`
  and `coverage` with no implicit default. Task 024 closed sums and Task 026
  optional elision are now attributable to a declared caller assumption rather
  than a hidden one, and open mode fails closed on every abstract structural
  value. ADR-0004 moved Proposed → Accepted;
- supplying private derived-type schemas in the generation schema set is
  validated end to end for the **same target namespace**: a private-extension
  overlay fixture closes the extension point and lowers through ordinary
  Task 024 closed sums with no new code.

**Complete (Task 029):**

- **explicit root-plus-private-schema overlay composition.** One frontend API,
  `load_schema_set_with_overlays(root, overlays)`, composes a primary root and
  additional top-level same-namespace schema documents into one normalized
  `SchemaIr`. `load_schema_set` is now its empty-overlay case. Overlays are
  additive only: the root stays authoritative for schema version, namespace
  presentation, and initial declaration order; duplicate qualified names remain
  errors with no precedence; documents are deduplicated by canonical path;
- **repeatable same-namespace CLI overlay input.** `--overlay PATH` on
  `validate`, `coverage`, and `generate`, accumulated in command-line order
  (never sorted) and accepted even when repeated. A private derived type can now
  be supplied against a *pinned* authoritative root with no edit to that root
  and no manufactured wrapper document, which makes ADR-0004 option 4
  operational. Overlays never infer a world.

**Still open:**

- representation for extension values whose derived types are *not* known at
  generation time (arbitrary external derived types); this depends on Phase 3
  runtime/codec work and must not be decided before it;
- an extension registry / runtime, including `xsi:type` handling and runtime
  codec dispatch;
- multi-namespace generation, if a private derived type must live in a different
  target namespace than its base — top-level overlays are same-target-namespace
  only, and current backends still require a single namespace;
- CAL codec and runtime polymorphism.

`SourceCommandEXT`'s `0..unbounded` occurrence stays fail-closed in **both**
worlds: Task 028 deliberately did not adopt an always-empty repeated-value rule.
No name-based (`EXT` suffix) heuristic may be used; the schema contains no
reliable machine-readable discriminator for extension points.

## Phase 2.5 — Contract-driven generation

A portable AMS GRA Service Contract is now a first-class generator input, not a
distant Phase 6 helper. See `docs/service-contract-integration.md` and
`docs/adr/0005-service-contract-codegen-plan.md`.

**Complete (Task 030):**

- [x] portable Contract IR — a dedicated `ams-gra-oms-service-contract` crate
      parsing v0.1 YAML/JSON into a typed IR, with fail-closed portable
      validation and `deny_unknown_fields`. It depends on no frontend, backend,
      CLI, or runtime code;
- [x] UCI message resolution — exact local-name equality against
      `SchemaIr.messages`; zero matches fail, more than one fails as ambiguous
      with candidates listed, and no fuzzy or first-match behavior exists;
- [x] contract-selected type closure — the transitive named closure of the
      selected messages' payload types, computed with the single shared
      dependency model that declaration ordering and coverage analysis also use;
- [x] Service Plan — a language-neutral `ServicePlan` in `codegen-core`,
      preserving contract function/exchange order, all four non-UCI exchange
      kinds, Capability ownership and standard roles, and the distinction
      between omitted and explicitly empty Capabilities;
- [x] a read-only `service-plan` CLI command with explicit
      `--extension ID=PATH` mappings, exact extension-set matching, and
      contract-ordered Task 029 overlay composition.

**Still open:**

- [ ] selected-closure backend generation;
- [ ] generated service publish/subscribe façade;
- [ ] LA-CAL runtime integration.

Nothing in this phase copies the OMS profile engine or the completion
assistant: profile conformance and contract completion remain owned by
`zackboll/ams-gra-service-contract`. There is no Capability inference, no
function grouping, no topic generation, and no Section 3.3 regeneration here.

### Why this precedes full-UCI generation

> Useful contract-driven generation does not require all 722 UCI 2.5 message
> closures to be renderable.

The code generator may generate only the type/message closure **selected by a
Service Contract**. A service that uses a handful of messages needs only those
messages' transitive type closures to render, not the whole 5,557-type UCI
universe. Full-UCI generation (Phase 5) remains an eventual capability and a
useful coverage metric, but it is no longer a precondition for delivering
value to a real service.

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
