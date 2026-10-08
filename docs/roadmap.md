# Roadmap

Task065 implements **generic semantic Ada artifact naming** for a concrete
Choice's literal in an abstract closed sum, without renaming established
companion types or unrelated successful outputs. Both pinned full-schema
Ada naming preflights pass; historical isolated closed coverage equals Rust/C++
(674/679). Current integration includes Task063 via a normal main merge.
Measured integrated closed totals are 676/679 on every backend, with zero
peer-minus-Ada gap; open totals are 571/574.
The separately authorized pre-existing component/type hiding defect uses
conditional named-subtype qualification without public field renaming; strict
synthetic and Authorization probes pass locally. Final delivery gates remain
explicit and are not implied by those focused results.
Three Unicode profiles and other semantic/topology blockers remain deferred.
See [Task065 evidence](task-065-ada-companion-naming.md) for immutable BEFORE,
full versus projected distinctions and explicit delivery-gate status.

Task066 resumes under the authorized **Unicode 3.1.0 Nd baseline** and supports
the three exact formerly deferred String profiles across Ada/Rust/C++. UTF-8
scalar length, checked lifecycle and JSON/mock-OWP lexical preservation are
compiler-backed. Eleven closed projected services per release/backend become
READY; full-schema message closures do not change. See [Task066 evidence](task-066-unicode-string-profiles.md)
for historical STOP provenance and explicit final delivery status. Historical
Task060 measurements are unchanged.

Task 063 implements the exact UCI 2.5 named patterned integral value-space
profile without broadening direct integers. Its isolated historical evidence
remains frozen; current integration includes the independently merged Task 062.
Numeric carriers and the OMS JSON integer mapping are retained. General numeric
patterns, lexical integer storage and exclusive-bound support remain deferred.
Evidence and outstanding review gates: [Task 063](task-063-patterned-integral-profile.md).

Task 062 adds **exact pinned IPv6 constrained-String support** across Ada,
Rust and C++, preserving lexical spelling without networking parsers or a
general regex engine. All strict compiler corpora and Rust checked JSON/mock-OWP
gates pass locally. Both releases gain five closed / four open projected READY
services per backend; the smallest, `RDMA_InitializeSetup`, passes the real
three-language model/API and Rust codec vertical. Three Unicode profiles and
the known Ada Query companion naming collision remain deferred. See
[Task 062](task-062-ipv6-address-profile.md) for frozen parent and actual delivery
gate status; local evidence does not imply hosted validation or merge approval.

Task 061 implements evidence-bounded **named TimeZulu** checked carriers in
Ada/Rust/C++, plus the existing Rust OMS JSON codec path. Exact XML Schema `.+Z`
facets only; name-free admission, XML-collapse spelling preservation and no
temporal comparison. Direct Time, general temporal support, other Time facets,
Ada/C++ codecs and naming remediation remain deferred. Compiler-backed synthetic
and pinned evidence is recorded in [Task 061](task-061-time-zulu-support.md);
hosted final-head and final-base review gates remain explicit, not implied by
local capability tests.

Task 043: [offline searchable HTML browser](task-043-uci-html-type-browser.md)
is implemented as a `SchemaIr` consumer, independent of source backends.
Task 045: [UCI Pages publication](task-045-uci-pages.md) assembles pinned 2.5
and 2.6 browsers through the production CLI; it adds no documentation model.
Publication runs on relevant `main` changes (including the Task 045 merge) or
manual dispatch, not on every PR; normal CI covers PR correctness.

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
- [ ] constrained String outside the supported profiles (named
      length-only constrained Binary is done: Task 053; the pinned
      single-class bounded-ASCII String family is done: Task 058);
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

(Later: Task 046 gives unconstrained direct `DateTime` and Task 057 gives
unconstrained direct `Duration` a validated carrier that composes with this
wrapper. `Time` and `Decimal` remain unsupported.)

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
- [x] **direct** unconstrained `xs:dateTime` fields — Task 046 uses one
      validated lexical carrier per generated unit and shares the Task 036
      calendar parser, admitting absent, Z and bounded numeric timezones.
      Named unconstrained DateTime declarations remain unsupported;
- [x] **named zero-facet `xs:duration`** (UCI `DurationType`) and **direct
      unconstrained `xs:duration`** fields / Choice alternatives -- Task 057,
      one checked lexical carrier and one shared parser per unit in all three
      backends, plus the Rust OMS JSON codec (JSON string = stored lexical).
      No arithmetic, ordering or value-space equality is claimed. See
      [Task 057](task-057-duration-support.md);
- [ ] constrained `Duration` (range, pattern, length facets) -- fail closed
      (none exists in pinned UCI);
- [ ] `Time`, and any *named* `DateTime` outside the Zulu profile
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

As of Task 032, generation was **types only**. A ready selection produced the
UCI type model and nothing else: no CAL façade, publisher/subscriber API,
service wrapper, codec, or runtime source. (Task 047 later added the typed
service API wrapper; the other items remain open.)

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

> **Task 044 follow-up:** Enum-only remapping now supports both leading-digit
> wire values and otherwise-valid target-reserved enum literals (the pinned
> schemas exercise the reserved-word path only in Ada).
> Rechecking selected UCI 2.5 exposed a separate blocker: the
> `SecurityInformationType` Record contains direct primitive `xs:dateTime`
> fields, outside this enum task. See
> [Task 044 evidence](task-044-enum-identifier-remapping.md) for the measured
> post-remapping result; the figures above remain the historical Task 041 result.

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
`ReleasableToEnum`), which was deliberately not done in Task 042. Task 044
subsequently delivered narrow plain-enum remapping, but its recheck exposed
direct primitive `xs:dateTime` fields in `SecurityInformationType` as the next
independent blocker. See [Task 044](task-044-enum-identifier-remapping.md).

Coverage gained exactly **+1** in all twelve pinned cells: the inventory found
exactly one declaration with this effective profile per release and no
derivations, and the four consuming choice types were already counted
renderable. In Rust and C++ those choice types' complete closures are now
renderable. See `docs/task-042-nato-special-words.md`.

**Complete (Task 047):**

- [x] service-specific generated wrapper APIs — `codegen-core` lowers the
      `ServicePlan` **once** into a small language-neutral `ServiceApiModel`
      (functions and every exchange occurrence, in contract order, all five
      exchange kinds, and a resolved payload binding for OMS Message
      exchanges only). A new `Backend::generate_service_api` renders it as
      `service_api.rs`, `service_api.hpp`, or `service_api.ads`, beside the
      unchanged selected type model. Backend crates still never see a
      contract. Scope names come from contract IDs behind fixed
      `function_`/`exchange_` prefixes; IDs that normalize to one identifier
      fail closed, and that wrapper preflight is part of readiness, so READY
      means the model **and** the wrapper can be generated. The real UCI 2.5
      `PositionReport` selection stays 60/60 READY in every backend, its model
      files are byte-identical to Task 046, and its wrapper compiles in all
      three languages.

The wrapper is compile-time endpoint metadata only: it sends, receives,
encodes, decodes, subscribes, publishes, dispatches, and connects to nothing.
A zero-OMS service now produces exactly one file, its wrapper. See
[Task 047](task-047-service-api-wrappers.md).

- [x] generated service publish/subscribe façade (Task 048). Every OMS
      Message exchange gains exactly one typed operation, decided once in
      codegen-core from its `Direction` (`output` -> Publish, `input` ->
      Subscribe; mandate and timing never change it). Publish takes exactly
      that endpoint's `Payload`; Subscribe registers a handler for exactly
      that `Payload`. The application supplies no topic, message name,
      namespace, or subscription group: the operation forwards them, with the
      resolved message `QualifiedName` still structured and the authored
      group verbatim, to an injected adapter whose result/error/token type the
      runtime chooses (Rust generic traits, C++ duck-typed templates, Ada
      generic packages; the Ada spec stays bodyless). The wrong operation is a
      compile error in every language. Non-OMS kinds and zero-OMS wrappers are
      unchanged; the real UCI 2.5 `PositionReport` input gets Subscribe only,
      with 60/60 READY and byte-identical model files.

```text
Task 047:  What endpoints exist?
Task 048:  Which typed operation may the application perform at each OMS endpoint?
Task 049:  How are those operations executed through LA-CAL/Sleet?
```

The façade still performs no communication: no WebSocket, OWP, subscription
IDs, codec, thread, or dispatcher. See
[Task 048](task-048-publish-subscribe-facade.md).

**Still open** (Phase 3 progress is tracked below):

- [ ] typed LA-CAL integration (Rust slice done in Task 049; Ada/C++ open);
- [ ] codec and runtime integration (Rust generated payload codec done in
      Task 050; Ada/C++ codecs and runtimes open).

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

Progress (typed LA-CAL integration as a whole is **not** complete):

- [x] Rust reference LA-CAL runtime adapter (Task 049). The Task 048 adapter
      traits moved verbatim into the tiny, dependency-free
      `ams-gra-oms-runtime-api` crate, and generated Rust wrappers re-export
      them (application imports and calls unchanged). `ams-gra-oms-runtime-rust`
      implements them over the pinned public `sleet-client`
      (`e38f61d8`): connect + `INIT`/`INFO`, typed Publish/Subscribe,
      runtime-owned subscription IDs, `MSG` dispatch by SID, explicit
      unsubscribe, close, the §6.1.1 global-element envelope and LA-CAL
      message-name formatting, and runtime errors/events. Proven through the
      generated façade against a mock OWP peer in CI and against the
      unmodified pinned Sleet.
- [ ] Ada LA-CAL runtime.
- [ ] C++ LA-CAL runtime.
- [x] generated **Rust** OMS JSON payload codecs (Task 050). Opt-in
      `service-check`/`service-generate --with-codec` emits `service_codec.rs`
      with one `OmsJsonCodec<P>` impl per unique payload, following
      OMSC-SPC-013 Rev B §6.1 from the projected Schema IR (never Serde
      derives, never the Rust layout). Every decode is built through the
      generated model's checked constructors. The real UCI 2.5
      `PositionReport` round-trips through unmodified pinned Sleet as a typed
      `PositionReportMT` in CI. Binary failed closed until Task 052.
- [x] qualified single-namespace extension member QName support (Task 051).
      Schema IR preserves each local element's effective target namespace
      (`FieldDecl.wire_namespace_uri`, from `elementFormDefault` and local
      `form`); Record/Choice member keys and `$type` values use the element or
      concrete-type QName: bare for OAM, `{namespace}local` otherwise. A
      qualified non-OAM single-namespace service is codec READY. Model,
      wrapper, and OAM codec output is byte-identical.
- [ ] Ada and C++ generated OMS JSON codecs (`--with-codec` reports them NOT
      READY at the `service codec boundary`).
- [ ] multi-namespace backend generation (backends remain single-namespace).
- [ ] unqualified member OMS JSON semantics: OMSC-SPC-013 Rev B gives no
      spelling for an absent element namespace, so it fails closed.
- [ ] `xs:element ref` support.
- [x] `xs:hexBinary` lexical provenance in Schema IR (Task 052):
      `TypeRef.binary_encoding` on the primitive ancestry, resolved through
      named restrictions by one shared query; unknown provenance stays legal
      IR but codec NOT READY.
- [x] Rust hexBinary OMS JSON codec (Task 052): canonical uppercase encode,
      XSD collapse + case-insensitive decode, one private generated helper
      pair, no new dependency. Real UCI 2.5 `SubsystemStream` round-trips
      through unmodified pinned Sleet.
- [ ] `xs:base64Binary` support, if a real schema needs it (none of the
      pinned UCI 2.5/2.6 documents use it; it stays an explicit unsupported
      frontend construct and a fail-closed codec provenance).
- [x] constrained named Binary carriers (Task 053): `length`/`minLength`/
      `maxLength` in OCTETS, classified by one shared
      `codegen-core::binary_length_domain`; checked carriers in Ada (private,
      failing default), Rust (`new -> Option`) and C++ (`create -> optional`,
      copy-only lifecycle); Rust codec decodes through the generated checked
      constructor. Every real UCI 2.5/2.6 constrained Binary (`AA_CodeType`,
      `BDS_AddressType`, `IFF_RegisterType`, `SHA_2_256_HashType`) is model
      renderable in all three backends.
- [ ] direct field-local Binary constraints (no per-field carrier; fail closed).
- [ ] Binary lexical patterns (a lexical constraint stored octets cannot enforce).
- [x] reserved Record-field / Choice-alternative identifier remapping
      (Task 054): fixed `Field_` / `Alternative_` escape from shared
      `codegen-core` helpers, final-name preflight, Rust codec keeps source
      wire names. Every reserved UCI 2.5/2.6 structural member is renderable.
- [ ] Ada top-level companion collision (`QueryType` vs `QueryType companion`
      -> `QueryType_Kind`), now the closed-schema Ada full-schema first blocker.
- [ ] abstract value targets with no concrete descendant (`SourceCommandEXT`)
      and open-extensions `CapabilityCommandBaseType`.
- [x] readiness vs generated-support parity (Task 056): readiness measures the
      projection's generated-support declarations with the same single
      projection / `CoverageAnalysis` / baseline snapshot, in separate
      `generated_support_*` fields; READY requires them to render.
      `OrderOfBattle` is now NOT READY before any backend call.
- [x] `xs:duration` (`DurationType`, direct `xs:duration` members such as
      `EphemerisOrbitalModelType.IntegratorStepSize`) -- Task 057 model support
      in Ada/Rust/C++ and the Rust Duration codec. `EphemerisOrbitalModelType`
      is now renderable.
- [x] `AircraftIdentifierType`, `EmptyType` and the sibling single-class
      bounded ASCII String profiles -- Task 058: 61 declarations / 19
      alphabets / 60 pinned rows, all backends plus the Rust codec; 131 real
      messages per release became READY. See
      [Task 058](task-058-bounded-ascii-string-profiles.md).
- [x] 27 pinned deterministic structured ASCII String declarations (Task 059):
      literal prefixes/separators, position-specific classes, optional and
      one-or-more repetitions, maxLength-only and length-1 follow-ups. Checked
      carriers in all three backends and Rust JSON codec. `OrderOfBattle`
      support now has only three alternation blockers. See
      [Task 059](task-059-structured-ascii-string-profiles.md).
- [x] 15 alternating ASCII constrained String declarations per release
      (Task 060, 16 exact profiles): includes `NotationType`, `MilitaryGridType`
      and `RecordOriginatorType`; no general regex engine. OrderOfBattle
      generated support is 442/442, with six-way model/API compiler evidence.
      See [Task 060](task-060-alternating-ascii-string-profiles.md).
- [x] exact pinned IPv6_AddressType lexical profile (Task 062; name-free).
- [ ] three deferred Unicode profiles.
- [ ] `xs:time` (`TimeType`, now the first blocker of 28 messages per
      release).
- [x] plan binding scope (Task 064): selected semantic binding is retained;
      closed-world projection also binds actual generated-support semantics
      and membership through the one production expansion model. Open-world
      abstract-value attribution remains unchanged. See
      [Task 064](task-064-generated-support-plan-binding.md).
- [ ] reconnect/backoff, TLS/auth policy, timers, service lifecycle.

```text
Task 048:  generated type-safe operation
Task 049:  first reusable runtime executing that operation through real LA-CAL
Task 050:  generated Rust payload-body OMS JSON codecs
Task 051:  local element wire QNames in Schema IR; qualified non-OAM codecs
Task 052:  xs:hexBinary lexical provenance in Schema IR; Rust hex codecs
Task 053:  constrained named Binary carriers (octet length domain), all backends
Task 054:  reserved Record/Choice member identifier remapping, all backends
Task 056:  readiness accounts for generated-support declarations
Task 057:  XML Schema duration checked lexical carriers, all backends; Rust codec
Task 058:  bounded-ASCII single-class String carriers, all backends; Rust codec
```

See [Task 049](task-049-rust-la-cal-runtime.md),
[Task 050](task-050-rust-oms-json-codecs.md),
[Task 051](task-051-member-qname-provenance.md),
[Task 052](task-052-hexbinary-provenance-codec.md), and
[Task 053](task-053-constrained-binary-carriers.md). Generic Binary codec
completeness is NOT claimed: only hexBinary, only Rust, and only named
length-only constraints (no field-local facets, no patterns, no base64).

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

- [x] Fast / Deep CI split (Task 055): one blocking deterministic Fast CI run
      per PR head with same-PR cancellation; real pinned UCI / Sleet / real-UCI
      MSRV evidence in Deep CI on every merge to `main`, nightly and on demand.
      CI infrastructure only. (The `OrderOfBattle` readiness vs generation
      item it left open in Phase 3 was closed by Task 056.)
- [ ] possible follow-ups after measuring the split: consolidate the per-test
      `require_one_test` invocations, cache builds, and decide on branch
      protection once the check names are stable;
- deterministic generation;
- schema digest verification;
- generated-file headers with source provenance;
- SBOM for generator releases;
- signed release artifacts;
- hermetic/air-gapped generation workflow.
