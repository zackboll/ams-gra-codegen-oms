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

**Complete (Task 031):**

- [x] backend/world readiness — `analyze_service_readiness` in `codegen-core`
      takes a `ServicePlan`, its `SchemaIr`, a `BackendLanguage`, and a
      `GenerationWorld`, and reports how much of the contract-selected UCI type
      model that backend can render today. The plan itself stays
      world-independent: readiness is a separate analysis, not a plan field;
- [x] deterministic blocker reporting — unsupported selected types in schema
      declaration order, blocked selected messages in contract first-occurrence
      order, each with one typed first blocker. Repeated message selections are
      deduplicated, and unselected unrenderable declarations are never reported;
- [x] one capability model — the per-declaration renderability computation was
      extracted out of `BackendCoverage` into a single snapshot that both
      full-schema coverage and selected-service readiness consume. Full-schema
      coverage results are unchanged, and readiness enables no hypothetical
      feature family;
- [x] a read-only `service-check` CLI command requiring `--language` and
      `--world`, reusing the same exact extension mapping as `service-plan`,
      writing no files, and exiting 0 READY / 1 NOT READY / 2 usage error.

Readiness is **analysis, not generation**: a READY verdict means a backend
could render the selected closure, not that any service source exists yet.

**Complete (Task 032):**

- [x] contract-selected type generation — `project_service_generation_schema`
      in `codegen-core` narrows a full `SchemaIr` to the model one contract
      selects, and the existing Ada/Rust/C++ backends generate from that
      projected schema unchanged. No backend crate learned what a contract is;
- [x] semantic closure versus generated support closure — `ServicePlan`'s raw
      selected closure stays exactly what the contract selects, while a
      separate fixed-point support closure supplies the Task 024 closed-sum
      concrete descendants (and their dependencies) that generated
      representation needs. The two are reported separately and never conflated;
- [x] a `service-generate` CLI command requiring `--language`, `--world`, and
      `--output`, gated on Task 031 readiness: a NOT READY selection prints the
      same report `service-check` prints, invokes no backend, and writes no
      file — not even the output directory.

Generation is still **types only**. A ready selection produces the UCI type
model and nothing else: no CAL façade, publisher/subscriber API, service
wrapper, codec, or runtime source is emitted yet.

**Still open:**

- [ ] service-specific generated wrapper APIs;
- [ ] generated service publish/subscribe façade;
- [ ] typed LA-CAL integration;
- [ ] codec and runtime integration.

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
