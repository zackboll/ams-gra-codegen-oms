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

### Scalar range restrictions

**Complete (Task 020):** named integral declarations and direct integral field
constraints lower to checked ranges in all three backends, for the inclusive
subset.

**Complete (Task 033):** named `Float32`/`Float64` numeric range restrictions
lower in all three backends — lower-only, upper-only, two-sided, inclusive,
exclusive, and mixed, including effective bounds inherited through Task 015
named restriction chains. Widths are preserved (binary32 stays binary32), and
IEEE semantics for NaN, one-sided infinities, and signed zero follow from the
emitted comparisons rather than from special cases. This covers the entire
authoritative floating tranche: both pinned releases contain zero lexical and
zero length facets on floating types.

**Still open:**

- [ ] field-local floating constraints on a *direct* primitive field (no
      checked wrapper exists for that storage, so it stays fail-closed);
- [ ] floating lexical facets (none exist in the authoritative releases);
- [ ] constrained String and constrained Binary;
- [ ] the excluded integral exclusive/lexical shapes.

### Occurrence / cardinality representation

**Complete (Task 034), for the exact subset below:** Ada general optional named
Record values. A Record field lowers to a generated per-field discriminated
wrapper `Owner_Field_Optional` iff cardinality is `0..1`, the field is not
nillable, the target is a **named** type, field-local constraints are default,
and the target is otherwise renderable by existing Ada rules. Rust and C++ were
already `Option<T>` / `std::optional<T>` here and are unchanged.

This is an **occurrence** change only. It does not make an unsupported target
kind supported: an optional `DateTimeType` still fails, attributed to the
temporal target rather than to optionality. Helper names belong to the emitted
owner, so an inherited optional field on a non-emitted abstract ancestor
renders as `Derived_Maybe_Optional`, and the wrapper participates in the shared
generated-name preflight. See `docs/backend-compatibility.md`.

**Complete (Task 035), for the exact subset below:** Ada optional **direct
primitive** Record values, reusing the identical Task 034 wrapper. A `0..1`
non-nillable direct primitive field lowers to `Owner_Field_Optional` iff the Ada
backend already implements that primitive's *value* representation:

| Direct primitive | Task 035 | Wrapped value type |
| --- | --- | --- |
| `Boolean` | wrapper | `Boolean` |
| `SignedInteger` | wrapper | `Long_Long_Integer`, with any Task 020 range |
| `UnsignedInteger` | wrapper | `Interfaces.Unsigned_64`, with any Task 020 range |
| `Float32` | wrapper | `Interfaces.IEEE_Float_32` |
| `Float64` | wrapper | `Interfaces.IEEE_Float_64` |
| `Binary` | wrapper | `Binary_Vectors.Vector` |
| `String` | **unchanged** | shared `Optional_String`, no per-field wrapper |
| `DateTime`/`Time`/`Duration`/`Decimal` | unsupported | no *direct* primitive value representation |

Task 036 does **not** change that last row. It adds a **named** `DateTime`
declaration carrying the UCI Zulu profile, which is a different question: a
named declaration gets its own generated wrapper type, and an optional field
referencing it composes through the existing Task 034 named-target path. A
direct `<xs:element type="xs:dateTime"/>` field still has no representation.


`ConstraintSet::default()` is deliberately **not** the integral gate. The
frontend normalizes a built-in integer type's domain into semantic IR bounds, so
a bare `<xs:element type="xs:unsignedInt" minOccurs="0"/>` — which carries no
author-written facet — still arrives with `minInclusive = 0`,
`maxInclusive = 4294967295`. Integral fields are therefore gated on Task 020's
`inclusive_integral_domain()`, the same lowering required direct integral fields
already use, keeping one integral-domain policy and no IR constraint provenance.
Non-integral direct primitives have no field-local facet lowering at all, so
they do still require default constraints.

**Still open:**

- [x] **named `DateTime` carrying the UCI Zulu profile** — delivered by Task
      036 as a validated lexical carrier in all three backends. This removed
      `DateTimeType` as the first selected `PositionReport` blocker. See
      `docs/backend-compatibility.md` for the exact supported profile;
- [ ] **direct** temporal primitive fields (`<xs:element type="xs:dateTime"/>`)
      — deliberately still unsupported. Task 036 is a *named declaration*
      slice; the reusable direct-primitive temporal representation waits for a
      later temporal-generalization task, once this validator architecture has
      proven itself;
- [ ] `Time` and `Duration`, and any `DateTime` outside the Zulu profile
      (unconstrained, a different pattern, multiple alternatives or groups, or
      an unsupported neighbouring facet) — all fail closed. `TimeType` carries
      the *same* `.+Z` text as the supported DateTime profile and was
      deliberately not admitted alongside it: `time` has its own lexical
      grammar and needs its own validator and conformance corpus;
- [ ] `Decimal` — unchanged, no value representation;
- [ ] temporal **arithmetic, ordering, value-space equality, timezone
      conversion, canonicalization, and codecs** — explicit Task 036 non-goals.
      The generated carriers deliberately expose no comparison;
- [ ] integral shapes Task 020 rejects (exclusive bounds, half-open ranges,
      length/lexical facets) and field-local facets on optional
      Boolean/float/Binary — all still fail-closed, with no silent facet loss;
- [ ] nillability in any backend — no optional Record field in either pinned
      UCI release is nillable, so this stays fail-closed on evidence;
- [ ] optional **Choice alternatives**, named or direct primitive —
      deliberately not enabled by Task 034 or Task 035, whose scope is Record
      fields only.

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

**Measured progress (Task 033):**

Task 033 added no Phase 2.5 code. It raised backend capability, and the
contract-selected readiness improved automatically through the existing shared
capability model. Re-measured against authoritative UCI 2.5, the upstream
`PositionReport` contract, closed world:

| Backend | Task 031 | Task 033 | First blocker now |
| --- | --- | --- | --- |
| Rust | 47/60, `AltitudeType` | 52/60 | `DateTimeType` |
| Ada | 32/60, `Acceleration3D_Type` | 37/60 | `Acceleration3D_Type` |

Those are the Task 033 figures. After generated-name preflight the current
authoritative readiness is **Rust 51/60** and **Ada 32/60**, with the same
first blockers; see `docs/backend-compatibility.md`.

**Measured progress (Task 034):**

Task 034 again added no Phase 2.5 code. It removed Ada's optional named-value
occupancy boundary, and contract-selected readiness improved automatically
through the same shared capability model. Same authoritative inputs:

| Backend | Before | After | First blocker now |
| --- | ---: | ---: | --- |
| Ada | 32/60, `Acceleration3D_Type` | **44/60** | `MissionID_Type` |
| Rust | 51/60, `DateTimeType` | 51/60 | `DateTimeType` (unchanged) |

`Acceleration3D_Type` is no longer a blocker. Ada's new one, `MissionID_Type`,
inherits `Version : xs:unsignedInt 0..1` — an optional **direct non-String
primitive**, which Task 034 deliberately does not cover.

`PositionReport` is **not** ready in any backend: both probes still report NOT
READY. The remaining selected blockers are temporal primitives, constrained
String, and Ada optional direct non-String primitive fields — all open.

**Measured progress (Task 036):**

Task 036 again added no Phase 2.5 code. It made the named UCI Zulu `DateTime`
profile renderable, and contract-selected readiness improved automatically
through the same shared capability model. Same authoritative inputs — UCI 2.5,
the upstream `PositionReport` contract, closed world:

| Backend | Before | After | First blocker now |
| --- | ---: | ---: | --- |
| Ada | 47/60, `DateTimeType` | **48/60** | `UCI_SchemaVersionStringType` |
| Rust | 51/60, `DateTimeType` | **52/60** | `UCI_SchemaVersionStringType` |
| C++ | 51/60, `DateTimeType` | **52/60** | `UCI_SchemaVersionStringType` |

`DateTimeType` is no longer a blocker in any backend. The measured next blocker
is `UCI_SchemaVersionStringType`, a **constrained String** — which matches the
prior expectation, but is reported here because it was measured, not assumed.
Task 036 deliberately does not implement it.

`PositionReport` remains NOT READY in every backend.

**Measured progress (Task 037):**

Task 037 again added no Phase 2.5 code. It made the named UCI schema-version
**String** profile renderable, and contract-selected readiness improved
automatically through the same shared capability model. Same authoritative
inputs — UCI 2.5, the upstream `PositionReport` contract, closed world:

| Backend | Before | After | First blocker now |
| --- | ---: | ---: | --- |
| Ada | 48/60, `UCI_SchemaVersionStringType` | **49/60** | `UniversallyUniqueIdentifierType` |
| Rust | 52/60, `UCI_SchemaVersionStringType` | **53/60** | `UniversallyUniqueIdentifierType` |
| C++ | 52/60, `UCI_SchemaVersionStringType` | **53/60** | `UniversallyUniqueIdentifierType` |

`UCI_SchemaVersionStringType` is no longer a blocker in any backend. The
measured next blocker is `UniversallyUniqueIdentifierType`, a **different**
constrained-String family (`length` + a pattern). It is
reported here because it was measured, not assumed, and Task 037 deliberately
does not implement it.

Task 037's evidence gate also corrected a widely assumed fact: the UCI schema
version is **not** a four-group `NNN.NNN.NNN.NNN` string. The authoritative
pattern admits three dot-separated numeric groups plus optional suffixes, and
rejects the four-group form that appears as the type's own `uci:version`
attribute. See `docs/backend-compatibility.md`.

`PositionReport` remains NOT READY in every backend.

**Measured progress (Task 038):**

Task 038 again added no Phase 2.5 code. It made the named UCI **UUID** String
profile renderable, extending Task 037's shared `StringProfile` classifier with
a second variant rather than adding a parallel mechanism, and contract-selected
readiness improved automatically through the same shared capability model. Same
authoritative inputs — UCI 2.5, the upstream `PositionReport` contract, closed
world:

| Backend | Before | After | First blocker now |
| --- | ---: | ---: | --- |
| Ada | 49/60, `UniversallyUniqueIdentifierType` | **50/60** | `VisibleString256Type` |
| Rust | 53/60, `UniversallyUniqueIdentifierType` | **54/60** | `VisibleString256Type` |
| C++ | 53/60, `UniversallyUniqueIdentifierType` | **54/60** | `VisibleString256Type` |

`UniversallyUniqueIdentifierType` is no longer a blocker in any backend. The
measured next blocker is `VisibleString256Type`, a **third** constrained-String
family (`minLength`/`maxLength` plus a printable-range pattern). It is reported
here because it was measured, not assumed, and Task 038 deliberately does not
implement it.

Task 038's evidence gate corrected the prior abbreviated record of the UUID
profile on three counts. The declaration carries **one** `xs:pattern` facet, not
two alternatives, so the `|` is internal to a single IR expression. The nil
branch is **not** redundant, because the general branch constrains the version
nibble to `[1-5]` and the variant nibble to `[89abAB]` and the nil UUID fails
both. And those two classes — previously hidden behind an ellipsis — mean an
all-`f` UUID is **invalid**, which a general 8-4-4-4-12 hexadecimal reading
would have wrongly accepted. They are enforced because the schema contains them,
not because RFC 4122 does. See `docs/backend-compatibility.md`.

`PositionReport` remains NOT READY in every backend.

**Measured progress (Task 039):**

Task 039 again added no Phase 2.5 code. It made the named UCI **visible-ASCII**
String family renderable, extending the same shared `StringProfile` classifier
with a third variant rather than adding a parallel mechanism, and
contract-selected readiness improved automatically through the same shared
capability model. Same authoritative inputs — UCI 2.5, the upstream
`PositionReport` contract, closed world:

| Backend | Before | After | First blocker now |
| --- | ---: | ---: | --- |
| Ada | 50/60, `VisibleString256Type` | **51/60** | `SecurityInformationType` |
| Rust | 54/60, `VisibleString256Type` | **55/60** | `SecurityInformationType` |
| C++ | 54/60, `VisibleString256Type` | **55/60** | `SecurityInformationType` |

`VisibleString256Type` is no longer a blocker in any backend. The measured next
blocker is `SecurityInformationType`, a *record* whose own remaining blockers
are `NATO_SpecialWordsType`, `WhitespaceVisibleString1024Type` /
`WhitespaceVisibleString4096Type`, and several enumerations. It is reported here
because it was measured, not assumed, and Task 039 deliberately does not
implement it.

This is the first task whose coverage gain was **larger than one**, and
legitimately so. The evidence gate found that the blocker is not a lone profile
but one member of a family of **thirteen** declarations — identical in both
pinned releases — differing in nothing but their `minLength`/`maxLength` pair.
The profile was therefore parameterized rather than fixed, and every cell of the
coverage matrix gained exactly +13 in all three backends and both releases,
matching the inventory exactly.

Two findings from that gate are worth recording. First, the pinned XSD spells
the character class with XML **character references**, `[&#x20;-&#x7E;]`, which
the parser expands, so the raw bytes and the normalized IR read differently
while meaning the same thing. Second, and more consequentially, U+0020 SPACE is
*inside* the class and `xs:string`'s intrinsic `whiteSpace = preserve` is not
overridden, so leading, trailing, and all-space values are **valid** and must be
stored untrimmed — the opposite of the Task 037 and 038 profiles, whose
alphabets excluded whitespace entirely. TAB, LF, CR, DEL, and every non-ASCII
character remain invalid. See `docs/backend-compatibility.md`.

`PositionReport` remains NOT READY in every backend.

**Corrective progress (Task 040):**

Task 040 changed generated **lifecycle behavior**, not capability. Coverage is
unchanged in every measured cell, no new profile or `FeatureFamily` was added,
and Rust output is byte-identical. It closed two gaps in the *existing*
validated lexical carriers — the schema-version, UUID, visible-ASCII, and Zulu
DateTime families:

- **Ada** default initialization created a usable, unchecked carrier whose
  `Value` returned the empty string, which every one of these profiles rejects.
  The private component now carries an explicitly failing `raise` default, so a
  carrier must come from `Create` or from an already valid carrier. Enforcement
  is a language initialization effect, proven without `-gnata` and under
  `Assertion_Policy (Ignore)`;
- **C++** implicit move construction and move assignment left the still-live
  source holding a representation its own `create` rejects, which could then be
  copied. The carriers now declare their copy operations, which suppresses the
  implicit move operations so rvalue operations fall back to copying. The
  tradeoff — a `std::move` may copy and allocate, and these operations are
  correctly not `noexcept` — is accepted in favour of the validated-value
  invariant.

This also corrects an overly broad earlier claim: privacy alone does **not**
prove there is no unchecked construction path. Privacy stops a client from
naming the representation; it did not stop the language from
default-initializing it or from synthesizing destructive moves. See
`docs/task-040-validated-carrier-lifecycle.md`.

**Corrective pass (Task 040, repeated storage):**

The Ada rejecting default was correct for scalar carriers but conflicted with
**repeated storage**, which default-initializes physical capacity before any
live element is assigned. A second over-broad claim is corrected here: not all
legitimate Ada construction paths were unaffected. Appending valid carriers to
an unbounded field raised `Program_Error` from inside the container, and a
valid *empty* `0..N` bounded field raised on declaration. Unchanged coverage
counts could not have caught either, because capability analysis never executes
a container operation.

The governing invariant is now *unused capacity is not a live validated value*:
unbounded fields use `Ada.Containers.Indefinite_Vectors` behind an opaque
sequence type, and bounded fields keep their bounded array but give each
physical slot a discriminated record whose unused variant has no payload. A
separate **pre-existing** defect — the visible-part instantiation failing to
compile over any generated `private` element type — is fixed by the same
change and reproduces on original `main`.

This is an explicit generated-API change (documented in
`docs/task-040-validated-carrier-lifecycle.md` §10.5) and an explicit
allocation-policy tradeoff: indefinite vectors heap-allocate per element.
Cardinality semantics, coverage in all twelve pinned cells, and Rust/C++ output
are unchanged. The pinned UCI 2.5 inputs, previously unobtainable, were
obtained and freshly measured on this pass.

`PositionReport` remains NOT READY in every backend.

**Measured progress (Task 041):**

Task 041 added no Phase 2.5 code either. It made the named UCI
**whitespace-visible** String family renderable in both pinned releases,
extending the same shared `StringProfile` classifier with a fourth variant.
Same authoritative inputs — UCI 2.5, the upstream `PositionReport` contract,
closed world:

| Backend | Before | After | First blocker now |
| --- | ---: | ---: | --- |
| Ada | 51/60 | **53/60** | `SecurityInformationType` (unchanged) |
| Rust | 55/60 | **57/60** | `SecurityInformationType` (unchanged) |
| C++ | 55/60 | **57/60** | `SecurityInformationType` (unchanged) |

`WhitespaceVisibleString1024Type` and `WhitespaceVisibleString4096Type` are no
longer blockers. `SecurityInformationType` nevertheless remains the first
blocker, because its *other* remaining dependencies are untouched here:
`NATO_SpecialWordsType` and several enumerations whose members — for example
`25X1` — cannot form a legal Ada identifier without enum identifier remapping.
Two dependencies improving does **not** make `PositionReport` ready, and it is
still NOT READY in every backend.

Coverage gained exactly **+3** in all twelve pinned cells, not +2. The third is
`QueryString4096Type`, a genuine **semantic alias**: in UCI 2.6 its facets are
byte-for-byte the 4096 profile's, so a name-free classifier supports it
automatically. There were no transitive gains.

This is the first task where the two pinned releases carry materially different
*value spaces* for the same declaration name. UCI 2.5 restricts with
`whiteSpace = collapse` and `minLength = 0`; UCI 2.6 drops the facet entirely and
raises the minimum to one. Both are supported, as separate profile triples over
whole observed tuples rather than independent axes — the eight combinations of the
separately observed policies, minima and maxima include three that no release
contains, and those fail closed. The collapse half genuinely **normalizes** its
constructor input before storing it, so an input longer than `maxLength` is
accepted when its normalized form fits, and `Create ("")` succeeds where the
schema permits it. That last point corrected a Task 040 comment which had claimed
default construction was prohibited *because* the empty string is invalid; the
prohibition is an API policy, and for these profiles the empty string is a valid
value. See `docs/task-041-whitespace-visible-string.md`.

**Measured progress (Task 042):**

Task 042 added no Phase 2.5 code either. It made the constrained-String profile
of `NATO_SpecialWordsType` — `minLength 6`, `maxLength 261`, pattern
`NATO:[a-zA-Z\-_]{1,256}`, identical in both pinned releases — renderable,
extending the shared `StringProfile` classifier with a fifth, fixed variant.
Generated carriers check the total length, the exact case-sensitive `NATO:`
prefix, and the suffix class, and store the text unchanged. This is lexical
validation only. Same authoritative inputs — UCI 2.5, the upstream
`PositionReport` contract, closed world:

| Backend | Before | After | First blocker now |
| --- | ---: | ---: | --- |
| Ada | 53/60 | **54/60** | `SecurityInformationType` (unchanged) |
| Rust | 57/60 | **58/60** | `SecurityInformationType` (unchanged) |
| C++ | 57/60 | **58/60** | `SecurityInformationType` (unchanged) |

`NATO_SpecialWordsType` is no longer a blocker. `PositionReport` is still NOT
READY in every backend: every remaining selected blocker is an enumeration whose
members need identifier remapping (`DeclassExceptionEnum` everywhere; in Ada also
`FGI_SourceOpenEnum`, `FGI_SourceProtectedEnum`, `OwnerProducerEnum`,
`ReleasableToEnum`), which is the obvious next task and is deliberately not done
here.

Coverage gained exactly **+1** in all twelve pinned cells: the inventory found
exactly one declaration with this effective profile per release and no
derivations, and the four consuming choice types were already counted
renderable. In Rust and C++ those choice types' complete closures are now
renderable. See `docs/task-042-nato-special-words.md`.

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

Task 034's optional representation was chosen to be compatible with this phase
without anticipating it: the discriminant alone controls whether `Value`
exists, and there is no access type, no heap allocation from the wrapper
itself, no unchecked conversion, and no sentinel. No proof annotation and no
GNATprove CI was added there; that work belongs here.

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
