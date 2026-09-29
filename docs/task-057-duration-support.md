# Task 057 — XML Schema `duration` model support

Status: implemented. MODEL-CAPABILITY task. Base: `main` @
`b0a471299770805ef51f3f86e12606383c310b84`.

Task 057 adds XML Schema `duration` as a **checked lexical carrier** in Ada,
Rust and C++, for named zero-facet Duration declarations and for direct
unconstrained `xs:duration` Record fields / Choice alternatives. The pinned
OMS evidence supports it, so the Rust OMS JSON codec covers both too.
Nothing else is claimed:

```text
XML Schema duration  !=  Ada Duration
                     !=  std::chrono::duration
                     !=  Rust std::time::Duration / any seconds-based duration
```

The carrier stores the whitespace-collapsed, lexically valid spelling
**unchanged**. No arithmetic, ordering, value-space equality, total seconds,
calendar addition or canonicalization is implemented or implied.

Still open (see the roadmap): `xs:time`; constrained Duration facets (ranges,
patterns, lengths); `AircraftIdentifierType` and sibling constrained-String
profiles; `QueryType_Kind`; `SourceCommandEXT`; `CapabilityCommandBaseType`;
`EmptyType`; Ada/C++ codecs and runtimes; the plan-binding support-descendant
fingerprint scope.

## 1. Fresh inventory before any production change

Pinned roots, fetched fresh by `scripts/fetch-pinned-uci-2.{5,6}.sh`:
UCI 2.5 `093610b7…` (root SHA-256 `ac943049…`) and UCI 2.6 `78eb61b6…` (root
SHA-256 `af54ce72…`). The inventory is the Deep CI test
`crates/cli/tests/uci_duration.rs`. It was run against unchanged `main`
first. It reads the NORMALIZED IR, and it was cross-checked against the raw
bytes (`type="xs:duration"`, `base="xs:duration"`).

### Named Duration declarations

| Release | Declaration | Source | Ancestry | Effective `ConstraintSet` | Pattern | Range / length | Profile before → after |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 2.5 | `DurationType` | `UCI_MessageDefinitions_v2_5_0.xsd:117745` | `DurationType <- xs:duration` | `default` | none | none | `Err(DurationUnsupported)` → `Ok(Some(Duration))` |
| 2.6 | `DurationType` | `UCI_MessageDefinitions_v2_6_0.xsd:118281` | `DurationType <- xs:duration` | `default` | none | none | `Err(DurationUnsupported)` → `Ok(Some(Duration))` |

The immediate base is `xs:duration` (`<xs:restriction base="xs:duration"/>`,
no facet). The type is concrete, with no explicit `whiteSpace`. References
to `DurationType`: in 2.5, **188** declared and **232** effective field
occurrences; in 2.6, **199** and **243**. None carries a field-local facet
or `nillable`.

### Direct `xs:duration` members (UCI 2.5 only)

| # | Owner.member | Kind | Card. | Owner | Local / inherited | Emitted | Line |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | `EphemerisOrbitalModelType.IntegratorStepSize` | Record field | 0..1 | concrete | local | yes | 39334 |
| 2 | `LogMDT.ServiceUpTime` | Record field | 0..1 | concrete | local | yes | 51013 |
| 3 | `OrbitalEphemerisParametersReferenceType.EphemerisResultsStepSize` | Record field | 0..1 | concrete | local | yes | 61964 |
| 4 | `OrbitalProximityOperationsEventType.AnalysisDuration` | Record field | 0..1 | concrete | local | yes | 62464 |
| 5 | `OrbitalProximityOperationsEventType.DurationThreshold` | Record field | 0..1 | concrete | local | yes | 62469 |
| 6 | `OrbitalRendezvousEventType.AnalysisDuration` | Record field | 0..1 | concrete | local | yes | 62526 |
| 7 | `OrbitalRendezvousEventType.MinimumRangeAnalysisDuration` | Record field | 1..1 | concrete | local | yes | 62551 |
| 8 | `OrbitalVCM_RequestParametersType.StepSize` | Record field | 0..1 | concrete | local | yes | 63555 |
| 9 | `PO_ComponentSettingsFocusSweepSettingsStepTimeType.CollectionTime` | **Choice alternative** | 1..1 | concrete | local | yes | 71866 |

All nine members are in `UCI_MessageDefinitions_v2_5_0.xsd`. Each has
field-local `ConstraintSet::default()` and is not nillable. They belong to 7
distinct owners, with no inherited effective occurrence and no unused
abstract owner. **UCI 2.6 has none.**

| | UCI 2.5 | UCI 2.6 |
| --- | --- | --- |
| direct `xs:duration` members | **9** (confirmed) | **0** (confirmed) |
| named `DurationType` | **1**, zero facets (confirmed) | **1**, zero facets (confirmed) |
| message closures reaching any Duration | 269 | 272 |
| message closures reaching a direct duration | 49 | 0 |
| constrained Duration (named, direct, or facet on a reference) | **0** | **0** |

### Evidence gate (section 4)

Every authoritative shape is `PrimitiveKind::Duration` with
`ConstraintSet::default()` (named) or field-local `ConstraintSet::default()`
(direct). No UCI duration has a pattern, range, length or other lexical
facet, so the task proceeded, and constrained Duration stays fail-closed.

There are two independent guards. First, the XSD frontend rejects every
facet on an `xs:duration` restriction at load time. I verified this for
`xs:maxInclusive`, `xs:pattern`, `xs:maxLength`, `xs:whiteSpace` and
`xs:enumeration`. Second, the shared classifier rejects any constrained
Duration in IR.

## 2. Lexical grammar and representation

The authority is XML Schema 1.0 Part 2 Second Edition §3.2.6.1
(`REC-xmlschema-2-20041028/datatypes.html`, SHA-256 `980de872…`):

```text
'-'? 'P' (n 'Y')? (n 'M')? (n 'D')? ('T' (n 'H')? (n 'M')? (n ('.' n)? 'S')?)?
n := [0-9]+
at least one component; 'T' present iff at least one time component follows
```

The grammar is implemented as one small deterministic single-pass scanner,
the same in all three languages. There is no regex engine.

* The input may start with one `-`, followed by `P`. There is no `+`, and
  matching is case-sensitive.
* Date designators are ranked Y < M < D and time designators H < M < S. Each
  designator must strictly outrank the previous one, so duplicate and
  out-of-order components fail. The `T` separates date `M` from time `M`.
* Only `S` may carry `'.' [0-9]+`. `PT.5S`, `PT1.S` and `PT1.2.3S` fail.
* A `T` with no following component (`PT`, `P1DT`, `P1Y2MT`) fails.
* Anything else fails: exponent, comma, `W`, whitespace, non-ASCII digit.

**No overflow.** Component digits are only *classified*
(`is_ascii_digit` / `'0'..'9'`), never accumulated or converted. So
`P999999999999999999999999999999999999Y` and a 38-digit hour or minute are
valid.

**Negative zero.** §3.2.6.1 allows the optional `-` on every lexical form and
restricts no component value. So `-P0D` and `-PT0.0S` are legal spellings of
the zero duration. They are accepted and stored as written.

**Whitespace.** `duration` has fixed intrinsic `whiteSpace = collapse`. Every
constructor does three steps in order:

1. collapse: tab/LF/CR become spaces, runs are squeezed, ends are trimmed;
2. validate the collapsed text;
3. store the collapsed spelling.

Nothing is canonicalized: `P12M` stays `P12M` and never becomes `P1Y`. The
codec does no second normalization.

**Shared corpus.** `tests/fixtures/temporal/duration.txt` has 33 valid and
67 invalid cases:

* valid: every section-8 representative, the spec's own examples, very large
  digit strings, leading zeros, and collapsible whitespace;
* invalid: every section-8 case, plus lowercase, duplicates, misorder,
  embedded spaces, multiple signs, misplaced `T`, exponent, comma, `W`, NUL,
  a fullwidth digit, and NBSP.

The corpus was checked against an independent regex transcription of
§3.2.6.1, with 0 mismatches across 100 cases. Each generated Rust, C++ and
Ada probe was shown to run by planting a failing case and watching it fail.

## 3. Shared classification

`crates/codegen-core/src/temporal.rs`:

```rust
pub enum TemporalProfile { DateTimeZulu, Duration }
pub enum DirectTemporalProfile { DateTime, Duration }
TemporalProfileError::DurationUnsupportedConstraints        // was DurationUnsupported
DirectTemporalProfileError::DurationUnsupportedConstraints  // was DurationUnsupported
```

`Duration` classifies only when `kind == Duration` and
`constraints == ConstraintSet::default()`. Any other Duration shape is
`DurationUnsupportedConstraints`, with the message "unsupported temporal
declaration: Duration with unsupported constraints". Time is unchanged. No
check ever looks at a declaration's name.

| Shape | Before | After |
| --- | --- | --- |
| direct DateTime, no facet | `Ok(Some(DateTime))` | unchanged |
| direct Duration, no facet | `Err(DurationUnsupported)` | `Ok(Some(Duration))` |
| direct Duration + any facet | `Err(DurationUnsupported)` | `Err(DurationUnsupportedConstraints)` |
| direct Time | `Err(TimeUnsupported)` | unchanged |
| named Duration, no facet | `Err(DurationUnsupported)` | `Ok(Some(Duration))` |
| named Duration + any facet (incl. explicit `whiteSpace`) | `Err(DurationUnsupported)` | `Err(DurationUnsupportedConstraints)` |

There are two new shared predicates, `schema_emits_direct_duration` and
`emissions_emit_direct_duration`. They follow Task 046 exactly:

* inherited effective members count;
* absent-only elided members and unused abstract ancestors do not count;
* the generation world is respected.

A third new predicate, `schema_emits_named_temporal_profile(schema,
profile)`, lets each backend emit a parser only for its own profile. The
existing DateTime APIs are unchanged; `emissions_emit_direct_date_time` now
delegates to the same private rule.

Every consumer takes its decision from the classifier:

* coverage (`primitive_ref_renderable`, `field_renderable`);
* Ada optional-direct storage (`ada_optional_direct_primitive_representable`):
  a zero-facet Duration uses the wrapper, while Time and constrained Duration
  are not representable;
* name preflight;
* the three backends;
* Rust codec readiness.

## 4. Generated API

| | Named zero-facet (e.g. `DurationType`) | Direct support carrier |
| --- | --- | --- |
| Rust | `pub struct DurationType { lexical: String }`; `new(&str) -> Option<Self>`; `as_str(&self) -> &str`; `#[derive(Clone, Debug)]` only | `XmlSchemaDuration`, same text |
| C++ | `class DurationType`; `static std::optional<DurationType> create(std::string_view)`; `const std::string& value() const noexcept` | `XmlSchemaDuration`, same text |
| Ada | `type DurationType is private;` `function Create (Value : String) return DurationType;` `function Value (Item : DurationType) return String;` | `XML_Schema_Duration`, same spec/body |

Each backend uses one carrier template for both named and direct carriers.
Both call **one** shared parser per generated unit:

* Rust: `struct XmlSchemaDurationParser`, with private associated functions;
* C++: `class XmlSchemaDurationParser`;
* Ada: the body-level package `XML_Schema_Duration_Parser`.

There is no second parser. A unit without Duration never gets the Duration
parser, and the Duration and dateTime parsers never pull each other in.

Every storage position is generated and tested:

* required;
* optional (`Option`, `std::optional`, or the Ada `…_Optional` wrapper);
* bounded and unbounded repeated;
* inherited;
* Choice alternative.

**Equality and ordering** follow the DateTime policy exactly:

* Rust: the carrier derives no `PartialEq` or `Ord`. As with DateTime,
  structures that hold one lose `PartialEq`.
* C++: no comparison operator is declared.
* Ada: the predefined `"="` remains. The spec documents that it compares
  lexical spelling, NOT value-space equality.

No backend defines an ordering.

## 5. Task 040 lifecycle

**Ada.**

* The private record's only component defaults to
  `raise Program_Error with "<T> requires initialization from Create"`.
* The GNAT probe shows that default-declaring either the direct or the named
  carrier raises. It shows this twice: under the default policy, and under
  `-gnata` with `pragma Assertion_Policy (Ignore)`.
* Storage reuses the existing safe shapes: the discriminated wrapper for
  optional, slot records for bounded, and the private `Indefinite_Vectors`
  for unbounded (50 appends exercised). Spare capacity is never a live
  carrier.

**C++.**

* There is no public default constructor (checked by `static_assert`).
* The copy operations are declared, which suppresses the implicit moves. The
  probe shows that after `std::move` construction or move assignment, the
  source still holds its valid spelling.
* The carrier cannot be constructed from `std::string` or `const char*`.

**Rust.**

* The `lexical` field is private, so building the struct outside the module
  is a compile error (tested).
* There is no `Default`, and moves are ordinary Rust moves.

## 6. Generated-name preflight

Names are reserved exactly when they are emitted, using the caller's single
emission plan:

| Name | Ada | Rust | C++ | Reserved when |
| --- | --- | --- | --- | --- |
| direct carrier | `XML_Schema_Duration` | `XmlSchemaDuration` | `XmlSchemaDuration` | an emitted owner stores a zero-facet direct duration |
| shared parser | `XML_Schema_Duration_Parser` | `XmlSchemaDurationParser` | `XmlSchemaDurationParser` | direct carrier OR a named zero-facet Duration |
| `Create` / `Value` | overloadable Ada callables | — | — | same, through the existing overload-aware model |

The tests show:

* each name collides with a user declaration of the same name;
* each name stays free in a schema with no duration, and when only an
  unused abstract owner has a duration member;
* each name is reserved for a concrete owner that inherits a duration member;
* a named Duration alone reserves the parser but not the carrier;
* Duration never reserves `XmlSchemaDateTimeParser`.

In Ada, Duration's `Create` and `Value` coexist with the other carriers that
publish the same pair: String, DateTime (named and direct), constrained
floating, NATO and constrained Binary. A user declaration named `Create` or
`Value` still fails before any output and is reported as unsafe.

## 7. Rust OMS JSON codec (section 21 gate: the evidence supports it)

The evidence is the same as Task 050, re-verified: `open-arsenal/oms` @
`726272bd…`, OMSC-SPC-013 Rev B (docx SHA-256 `b1c3c078…`, Markdown
`5798da43…`).

§6.1.4 maps a simple type "based only on its {primitive type definition}".
Its cases are:

1. boolean;
2. float/double;
3. decimal;
4. the OAM `UniversallyUniqueIdentifierType`;
5. **"Otherwise string"**.

`xs:duration` is its own primitive and matches none of cases 1–4, so case 5
applies. This is the spec rule itself, not an analogy with DateTime.
§6.1.5.4 makes the string's characters (without quotes) the element's
character information items, so the string is the XML lexical value, and
§6.1.5 validates it against the XSD. The pinned Sleet validator agrees:
`SimpleValueKind::from_primitive` treats everything except boolean, decimal
and float as `String`.

What the codec does:

* Encode: `Value::String(carrier.as_str())`.
* Decode: accept a JSON string only, and pass it to the generated
  `XmlSchemaDuration::new` / `DurationType::new`. A rejection is reported at
  the member path.
* The codec contains no duration grammar of its own.

The synthetic `codec-duration` fixture (OAM namespace) covers named,
required, optional, bounded, unbounded and Choice positions. The tests cover:

* a round trip with the exact expected wire JSON;
* whitespace collapsed once, by the model constructor;
* an invalid lexical form in every position;
* rejection of non-string JSON (number, bool, null, array, object);
* a mock-OWP publish, subscribe and decode-error round trip.

## 8. Test-fixture adaptation (Task 056 and earlier fixtures)

Several synthetic fixtures used an unconstrained `xs:duration` as their
deliberately *unsupported* construct. Once Duration is supported, those
fixtures would silently stop testing what they were written to test. They
were **adapted, not re-expected**. `xs:time`, which is still unsupported, now
plays the unsupported role under renamed declarations, and every assertion
keeps its original verdict.

* Task 056: `support-readiness.xsd` (`BadDuration` / `UnrelatedDuration`
  became `BadTime` / `UnrelatedTime`), its three contracts, the unit test in
  `service_readiness.rs`, and `generated_support_readiness.rs`. A declaration
  that is unsupported and reachable only through generated support is still
  NOT READY, and still writes nothing.
* Task 031: `service-check/root.xsd`, `parity-unready.xsd` and
  `unready.yaml`.
* `service-generate/{root,optional-named,optional-primitive,constrained-float}.xsd`:
  `UnrelatedDuration` became `UnrelatedTime`.
* The `unsupported()` helper in `codegen-core/tests/service_readiness.rs`.
* In the unit tests (`coverage.rs`, `backend_names.rs`, the backend `lib.rs`
  tests, `temporal_coverage.rs`), each "unsupported" example now uses `Time`
  or a constrained Duration. Positive Duration tests were added alongside.

Task 056 semantics are unchanged.

## 9. Twelve-cell coverage (before → after)

The pinned roots were run through two release binaries: `main` @ `b0a4712`
(before) and this branch (after). The four figures reported per cell are
declarations fully renderable, field type references renderable, field
occurrences renderable, and message closures renderable.

| Release | World | Backend | Declarations | Field types | Field occurrences | Message closures |
| --- | --- | --- | --- | --- | --- | --- |
| 2.5 | closed | Ada | 5419 → **5427** /5557 | 13151 → **13160** /13160 | 13153 → **13160** | 355 → **395** /722 |
| 2.5 | closed | Rust | 5420 → **5428** | 13151 → **13160** | 13160 = | 355 → **395** |
| 2.5 | closed | C++ | 5420 → **5428** | 13151 → **13160** | 13160 = | 355 → **395** |
| 2.5 | open | Ada | 5332 → **5340** | 13151 → **13160** | 13153 → **13160** | 347 → **387** |
| 2.5 | open | Rust | 5332 → **5340** | 13151 → **13160** | 13160 = | 347 → **387** |
| 2.5 | open | C++ | 5332 → **5340** | 13151 → **13160** | 13160 = | 347 → **387** |
| 2.6 | closed | Ada | 5440 → **5441** /5570 | 13198 = /13198 | 13198 = | 355 → **395** /725 |
| 2.6 | closed | Rust | 5441 → **5442** | 13198 = | 13198 = | 355 → **395** |
| 2.6 | closed | C++ | 5441 → **5442** | 13198 = | 13198 = | 355 → **395** |
| 2.6 | open | Ada | 5353 → **5354** | 13198 = | 13198 = | 347 → **387** |
| 2.6 | open | Rust | 5353 → **5354** | 13198 = | 13198 = | 347 → **387** |
| 2.6 | open | C++ | 5353 → **5354** | 13198 = | 13198 = | 347 → **387** |

Declaration *kinds* went up by exactly one in every cell (5448 → 5449 and
5461 → 5462): that is `DurationType`'s kind. No cell went down. The library
`CoverageAnalysis` inside the Deep CI test reports the same closed-schema
numbers.

Where each delta comes from:

* **Field types, 2.5, +9.** These are exactly the nine direct `xs:duration`
  references, which are now renderable. 2.6 has none, so it has no change.
* **Field occurrences, 2.5 Ada, +7.** Seven of the nine members are optional,
  and Ada needed its optional wrapper for them. Rust and C++ had already
  counted every occurrence as renderable (occurrence is independent of the
  target), so they show no change.
* **Declarations, 2.5, +8.** `DurationType` plus the 7 direct-duration
  owners.
* **Declarations, 2.6, +1.** `DurationType` only.
* **Message closures, every cell, +40.** The same 40 messages in both
  releases and both worlds. Their closures' only non-renderable
  declaration(s) were Duration shapes (see §10).

## 10. Real message impact

For every real message whose selected **or** generated-support surface holds
a Duration, the Deep CI test builds a single-message contract, projects it,
and runs the full selected-service readiness in each backend. Each message is
then classified:

* **A**: READY now;
* **B**: NOT READY on another declaration, whether selected or generated
  support;
* **C**: a topology or projection failure.

The verdicts are identical across Ada, Rust and C++ for every message.

| | UCI 2.5 | UCI 2.6 |
| --- | --- | --- |
| messages reaching Duration | 270 | 273 |
| **A** READY solely because of Task 057 | **40** | **40** |
| **B** another declaration still blocks | 198 | 201 |
| **C** topology / projection | 32 | 32 |

**A** (the same 40 in both releases): `ActivityMetrics`,
`ActivityMetricsRequest`, `AnalysisRoute`, `AnalysisRouteRequest`,
`AnalysisRouteRequestStatus`, `DLZ_Request`, `DMPI_Status`,
`EntityOrbitalCSO`, `EntityOrbitalEphemeris`, `EntityOrbitalEphemerisRequest`,
`EntityOrbitalEphemerisRequestStatus`, `EntityOrbitalManeuver`,
`EntityOrbitalVCM_Request`, `FlightActivity`, `LAR_Request`, `Log`,
`NavigationActivity`, `NavigationReport`, `OperatorNotification`,
`OrbitChangeActivity`, `OrbitMetrics`, `OrbitMetricsRequest`,
`PropagatorSettings`, `PropagatorSettingsDataRequestStatus`,
`RF_ThreadInstanceSetupCommand`, `RequirementMetrics`,
`RequirementMetricsRequest`, `RouteMetrics`, `RouteMetricsRequest`,
`ServiceStatus`, `ServiceStatusDataRequestStatus`,
`SubsystemBIT_Configuration`, `SurvivabilityRiskLevel`,
`SystemEstimationRequest`, `SystemOrbitReport`, `SystemOrbitalEphemeris`,
`SystemOrbitalEphemerisRequest`, `SystemOrbitalEphemerisRequestStatus`,
`SystemOrbitalPositionReport`, `SystemOrbitalVCM_Request`.

Category A was not taken from declaration coverage. It was confirmed
separately, with the `service-check` CLI, on the before and after binaries
for all 80 release × message pairs: **before 0/40 READY, after 40/40
READY** in each release. The earlier blockers were Duration shapes only:

* 2.5: `DurationType` (27 messages), `EphemerisOrbitalModelType` (9),
  `OrbitalVCM_RequestParametersType` (2), `LogMDT` (1),
  `OrbitalProximityOperationsEventType` (1);
* 2.6: `DurationType` (40).

**B**, by first remaining blocker, in the order 2.5 / 2.6:

* `EmptyType` 91 / 91;
* `AircraftIdentifierType` 65 / 65, plus one message where it blocks only
  generated support: `OrderOfBattle`, 1 / 1;
* `AlphanumericDashSpaceUnderscoreStringLength15Type` 25 / 25;
* `AlphanumericDashSpaceUnderscoreString20Type` 5 / 5;
* `TimeType` 2 / 5 (`xs:time`, which stays unsupported, is now exposed);
* `LaunchPieceType` 2 / 2;
* `AO_PIM_CodeType` 2 / 2;
* one each: `CounterSpaceSENO_Type`, `AlphanumericStringLength4Type`,
  `AlphanumericString6Type`, `AlphanumericString54Type`,
  `AlphanumericString20Type`.

No remaining blocker is a Duration shape; the test asserts this.

**C** has two groups:

* 28 messages with cyclic generated value dependencies, identical in both
  releases (e.g. `Response`, `ActivityPlan`, the `ProductProcessing*` and
  `ProductOrFileClassification*` families);
* 4 `ProductOrFileDissemination{Plan,Report,Request,Task}` messages, where
  projection hits the unsupported abstract-value topology.

These are independent of Duration.

## 11. Real category-A service evidence

The smallest real category-A message is UCI 2.5 **`Log`**: 42 selected
declarations and no generated support. Its payload `LogMDT` has the optional
direct `ServiceUpTime : xs:duration`. Using
`tests/fixtures/service-generate/real-duration-log.yaml`, for each of Ada,
Rust and C++:

* `service-check` reports READY, and the Rust `--with-codec` status is also
  READY;
* `service-generate` succeeds;
* the generated model compiles: Rust with `-D warnings`, C++ with
  `-std=c++17 -Wall -Wextra -Werror -pedantic-errors` (the probe also checks
  that `" PT5M "` is stored as `PT5M`), and Ada with `gnatmake -gnatc`.

The generated Rust codec encodes `ServiceUpTime` as
`Value::String(x.as_str())` and decodes it through `XmlSchemaDuration::new`.
This runs as the Deep CI test
`task057_real_uci_category_a_log_generates_and_compiles`.

## 12. OrderOfBattle follow-up

`OrderOfBattle` stays NOT READY on generated support in every backend. The
table shows the counts before → after.

| Release | Selected | Support total | Support renderable | Unsupported support | First unsupported support |
| --- | --- | --- | --- | --- | --- |
| 2.5 | 55/55 | 442 | 400 → **403** | 42 → **39** | `EphemerisOrbitalModelType` → **`AircraftIdentifierType`** |
| 2.6 | 56/56 | 442 | 402 → **403** | 40 → **39** | `AircraftIdentifierType` (unchanged) |

* 2.5 gains `EphemerisOrbitalModelType`, which **is now renderable** (its
  direct `IntegratorStepSize` was the blocker). It also gains
  `OrbitalEphemerisParametersReferenceType` and `DurationType`.
* 2.6 gains `DurationType` only.
* `DurationType` has disappeared from the unsupported list in both releases.
* Both releases now have the same 39 unsupported support declarations, in
  every backend.
* Every backend's first failure on the projected schema is
  `AircraftIdentifierType`, including 2.5 Ada. Ada previously failed on
  `type reference Primitive(Duration)`.

## 13. Byte identity and regressions

**Byte identity.** Every `.xsd` fixture in the repository (180 files) was
generated with the before and after binaries, in all 3 languages × 2 worlds
(1080 cells):

* **521 cells byte-identical**;
* **0 cells differing**;
* 547 cells failing in both binaries. These include every adapted fixture,
  which now fails on `xs:time` exactly as it previously failed on
  `xs:duration`;
* 12 cells changing from failure to success. They are exactly the two new
  all-Duration fixtures, `backend-duration.xsd` and `codec-duration.xsd`,
  which the old binary could not generate.

This covers PositionReport (`position-report.xsd`), every DateTime fixture
(`backend-temporal-datetime`, `backend-direct-datetime`,
`service-generate/{temporal,direct-datetime}`), every String-profile, UUID,
visible-ASCII, whitespace-visible and NATO fixture, both constrained-Binary
fixtures, and the ordinary service fixtures. The DateTime parser and carrier
output is unchanged byte for byte.

**Temporal regressions.** The Task 036/046 suites all pass: the named Zulu
corpus, the direct DateTime corpus, and the GNAT, strict C++ and Rust
probes. Time still fails closed.

**Whole-schema first blockers.** All 12 cells (2 releases × 2 worlds × 3
backends) are unchanged:

* closed-schema Ada: `QueryType_Kind` collision;
* closed-schema Rust and C++: `SourceCommandEXT`;
* open-extensions, all backends: `CapabilityCommandBaseType`.

The Task 054 probe `uci_member_names` passes unchanged, and Task 053
`uci_constrained_binary` passes unchanged.

**Task 056.** The adapted synthetic suite passes (9 tests), and the real
parity test passes with the 39 / `AircraftIdentifierType` figures. The 34
Task 054 category-A messages stay READY and still generate, and
`OrderOfBattle` stays NOT READY.

## 14. CI placement

**Fast CI** (synthetic only) runs:

* the classifier and name-preflight unit tests;
* the Rust, C++ and Ada corpus probes;
* `cli/tests/duration.rs`;
* the Rust codec and mock-OWP tests.

The GNAT duration probe and the five Duration codec tests are also wired in
as `require_one_test` gates.

**Deep CI `real-uci`** runs
`cargo test --release -p ams-gra-codegen-oms --test uci_duration`, checking
whole-line markers:

* `UCI 2.{5,6} DURATION INVENTORY: PASSED`;
* `UCI 2.{5,6} DURATION MESSAGE IMPACT: RECORDED`;
* the libtest-prefix-tolerant `UCI 2.5 REAL CATEGORY-A DURATION SERVICE:
  PASSED`;
* `test result: ok. 4 passed`.

All marker checks use here-strings. `scripts/check-ci-split.sh` forbids
`--test uci_duration` in Fast CI and requires the target and every marker in
Deep CI. `scripts/test-check-ci-split.sh` adds adversarial cases for a
Fast-CI leak, each dropped marker, a glued prefix and a foreign prefix
(67 checks in total).

The Deep CI `pull_request` trigger paths are unchanged (Task 055). Because
this PR edits `.github/workflows/**`, Deep CI also runs on the PR.

## 15. Local validation

Toolchain: rustc 1.98.1 and 1.95.0, g++ 14.2.0, GNATMAKE 14.2.0.

| Check | Result |
| --- | --- |
| `cargo fmt --all -- --check` | pass |
| `cargo check --workspace --all-targets` | pass |
| `cargo clippy --workspace --all-targets -- -D warnings` | pass |
| `AMS_GRA_REQUIRE_GNAT=1 cargo test --workspace` | **1145 → 1170** passing tests (102 → 108 test binaries), 0 failures |
| `git diff --check` | pass |
| `scripts/check-ci-split.sh` / `scripts/test-check-ci-split.sh` | pass / pass (67 checks) |
| `cargo +1.95.0 check --locked -p ams-gra-oms-runtime-rust-facade-tests --all-targets` | pass (includes the generated `codec_duration` model + codec) |
| synthetic Duration model compiled and run with rustc 1.95.0, `-D warnings` | pass |
| Deep: `uci_duration` (4 tests, real 2.5 + 2.6) | pass (537 s + 73 s) |
| Deep: `uci_generated_support` (Task 056) | pass, with the new 39 / `AircraftIdentifierType` figures |
| Deep: `uci_member_names` (Task 054) / `uci_constrained_binary` (Task 053) | pass, unchanged |
