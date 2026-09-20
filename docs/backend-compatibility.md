# Backend compatibility

Tested: **2026-09-18**, baseline `1fa2ab4d380158728d383a5e4fbaebea6a2006b1`.

This document records backend status separately from frontend compatibility.
The authoritative schemas remain external and are not vendored. The frontend
fully normalizes the UCI 2.5 and UCI 2.6 roots; the three language backends do
not yet generate those complete models.

## Task 020 — integral scalar lowering

Boolean, SignedInteger, and UnsignedInteger are now baseline backend values in
Ada, Rust, and C++. Direct Boolean fields and Choice alternatives map to
`Boolean`, `bool`, and `bool`; unconstrained named Boolean declarations retain
their identity as a derived type/newtype/wrapper.

The frontend's intrinsic XSD integer domains remain on `FieldDecl.constraints`.
For direct SignedInteger and UnsignedInteger fields or Choice alternatives, an
inclusive finite integer range is now semantic lowering rather than discarded
metadata. The supported storage domain is exactly signed 64-bit (`Long_Long_Integer`,
`i64`, `std::int64_t`) and unsigned 64-bit (`Interfaces.Unsigned_64`, `u64`,
`std::uint64_t`). Out-of-domain ranges, negative unsigned minima, exclusive
bounds, length facets, and lexical facets fail before rendering.

The full signed-i64 and unsigned-u64 boundaries are part of this Task 020
domain; C++ extreme-bound emission is compiler-regression-tested with strict
C++17 flags. This is a boundary-correctness fix, not a semantic expansion.

Rust emits checked `BoundedI64`/`BoundedU64` const-generic wrappers; C++17 emits
a checked `BoundedInteger<T, Min, Max>` template; Ada uses constrained component
and array-element subtype indications. Finite repeated payloads and Choice
alternatives reuse those constrained representations. Named bounded unsigned
declarations are preserved as unsigned types/wrappers with checked construction
outside Ada's language-enforced range checks. Local constraints on named targets
remain fail-closed.

`PrimitiveExpansion` consequently means the remaining unsupported primitive
families (Decimal, Binary, DateTime, Time, Duration), not Boolean,
SignedInteger, UnsignedInteger, Float32, or Float64. `ConstrainedSimpleTypes`
continues to cover unsupported scalar
facets such as String length/patterns, non-integral constraints, and the excluded
exclusive/lexical integral facets. These hypothetical families remain additive.

Published Task 017 authoritative evidence remains: primitive member references
for Boolean/SignedInteger/UnsignedInteger are 409/80/386 (875) in UCI 2.5 and
349/82/387 (818) in UCI 2.6; named SignedInteger/UnsignedInteger declarations
are 4/40 and 6/40 respectively; local field constraints are 466/469 and use only
`minInclusive`/`maxInclusive`. External roots were probed through detached release
coverage processes during Task 020; no completed cross-tab or coverage delta is
claimed unless a completed probe result is recorded separately.

## Task 022 — unconstrained floating-point scalar lowering

Float32 and Float64 are baseline backend values only when their
`ConstraintSet` is default. Direct fields and Choice alternatives map exactly
to Ada `Interfaces.IEEE_Float_32`/`IEEE_Float_64`, Rust `f32`/`f64`, and C++
`float`/`double`; named declarations preserve identity as an Ada derived type,
Rust newtype, or C++ wrapper. C++ output containing floating values verifies
binary32/binary64 storage, radix, precision, exponent, and IEC-559 host
characteristics at compile time.

The authoritative roots contain 56 direct Float32 and 280 direct Float64
references in each release. The pre-task Ada blocker was the unconstrained,
required `AccelerationAccelerationCovarianceType.AnAn` Float64 field (UCI 2.5
line 4703; UCI 2.6 line 4727). Finite repeated values compose with existing
containers; Ada also composes `0..*` values through vectors with the explicit
`Interfaces."="` actual required by the active GNAT toolchain.

Unconstrained values intentionally preserve native NaN, infinities, and
negative zero. All non-default floating `ConstraintSet` values—including every
inclusive/exclusive range and lexical facet—remain fail-closed. This task does
not emit floating bound literals or validation wrappers.

Rust derives `Eq` only for Records and Choices whose effective payload graph is
Eq-capable. Float32/64 payloads, including through named or nested structural
references and containers, remove only `Eq`; `Debug`, `Clone`, and `PartialEq`
remain. Existing non-floating derive output is unchanged.

Using the Task 022 baseline binary
`7d80bb3d4967495c656132d21f09599ee780682bd40e2025c8ef8580b1f87a1e`,
field-type coverage was 12,806/13,160 for UCI 2.5 and 12,862/13,198 for UCI
2.6 in every backend. The post-change binary raised those to 13,142/13,160 and
13,198/13,198 respectively. Ada advances to the existing unsupported Choice
cardinality on `AccessAssessmentID`; Rust and C++ remain first-blocked by the
out-of-scope abstract `CapabilityCommandBaseType` value reference.

Reproduce the deterministic analyzer output with:

```bash
ams-gra-codegen-oms coverage --schema /path/to/UCI_MessageDefinitions_v2_N_0.xsd --world closed-schema
```

## Metric definitions

- **Kind renderable:** the declaration's `TypeKind` has a renderer, ignoring
  constraints, abstractness, inheritance, and member details.
- **Declaration fully renderable:** the current backend can render the complete
  declaration without discarding base, abstract, constraint, member type, or
  occurrence semantics.
- **Field type renderable:** the field/alternative's immediate `TypeRef` can be
  named or mapped to a currently supported primitive. It does not imply that a
  named target's declaration is fully renderable.
- **Field occurrence renderable:** cardinality and nillability can be represented
  by the current backend. This metric is intentionally separate from type and
  constraint support.
- **Message closure renderable:** the immediate payload and every declaration in
  its transitive named dependency closure are fully renderable. Dependencies
  include bases, aliases, record fields, choice alternatives, and list items.
  Cycles and missing declarations fail closed.

## Authoritative frontend milestone

| Release | Namespaces | Types | Messages | Result |
|---|---:|---:|---:|---|
| UCI 2.5 | 1 | 5,557 | 722 | valid, fully normalized |
| UCI 2.6 | 1 | 5,570 | 725 | valid, fully normalized |

## Complete declaration inventory

| Type kind | UCI 2.5 | UCI 2.6 |
|---|---:|---:|
| Boolean primitive | 0 | 0 |
| SignedInteger primitive | 4 | 6 |
| UnsignedInteger primitive | 40 | 40 |
| Decimal primitive | 0 | 0 |
| Float32 primitive | 4 | 4 |
| Float64 primitive | 40 | 40 |
| String primitive | 126 | 126 |
| Binary primitive | 3 | 5 |
| DateTime primitive | 1 | 1 |
| Time primitive | 1 | 1 |
| Duration primitive | 1 | 1 |
| Alias | 0 | 0 |
| Enumeration | 725 | 710 |
| Record | 4,192 | 4,212 |
| Choice | 420 | 424 |
| List | 0 | 0 |
| **Total** | **5,557** | **5,570** |

| Declaration property | UCI 2.5 | UCI 2.6 |
|---|---:|---:|
| Abstract | 70 | 70 |
| Concrete | 5,487 | 5,500 |
| Any `base_type` | 2,971 | 2,970 |
| Structural named base | 2,026 | 2,036 |
| Named simple restriction base | 23 | 25 |
| Primitive simple restriction base | 922 | 909 |
| Forward base reference | 898 | 901 |
| Backward base reference | 1,151 | 1,160 |

## Structural inheritance

| Shape | UCI 2.5 | UCI 2.6 |
|---|---:|---:|
| Record with Record base | 2,025 | 2,035 |
| Record with Choice base | 0 | 0 |
| Choice with Record base | 1 | 1 |
| Choice with Choice base | 0 | 0 |
| References to abstract bases | 1,270 | 1,273 |
| References to concrete bases | 756 | 763 |
| Structural roots | 2,586 | 2,600 |
| Distinct declarations used as bases | 271 | 272 |
| Maximum inheritance edge depth | 5 | 5 |
| Empty local Record extensions | 406 | 405 |
| Non-empty local Record extensions | 1,619 | 1,630 |
| Local Choice extensions | 1 | 1 |

No inherited field redeclarations, inherited choice-name redeclarations,
record/choice cross-level name collisions, or other repeated member names were
found in either release. The projection rejects such collisions with a
structured error; it does not rename silently.

## Fields and choices

| Inventory | UCI 2.5 | UCI 2.6 |
|---|---:|---:|
| Record fields | 11,692 | 11,716 |
| Choice alternatives | 1,468 | 1,482 |
| Named references | 11,931 | 12,044 |
| Required `1..1` | 5,883 | 5,919 |
| Optional `0..1` | 4,949 | 4,967 |
| Finite repeated | 395 | 360 |
| Unbounded repeated | 1,933 | 1,952 |
| `minOccurs > 1` | 16 | 16 |
| Nillable true | 0 | 0 |
| Nillable false | 13,160 | 13,198 |
| Local field constraints present | 466 | 469 |
| Field `minInclusive` | 466 | 469 |
| Field `maxInclusive` | 466 | 469 |
| Other field constraint families | 0 | 0 |

| Primitive member reference | UCI 2.5 | UCI 2.6 |
|---|---:|---:|
| Boolean | 409 | 349 |
| SignedInteger | 80 | 82 |
| UnsignedInteger | 386 | 387 |
| Decimal | 0 | 0 |
| Float32 | 56 | 56 |
| Float64 | 280 | 280 |
| String | 0 | 0 |
| Binary | 5 | 0 |
| DateTime | 4 | 0 |
| Time | 0 | 0 |
| Duration | 9 | 0 |

Both releases have 47 choices with duplicate payload types and 113 additional
alternatives sharing a payload type. Names remain unique: duplicate alternative
names are zero. Repeated alternatives number 93/94, nillable alternatives are
zero, and 419/423 choices are standalone. `QueryType` is the sole inherited
choice.

## Abstract-type usage

The 70 abstract structural declarations have the same usage distribution in
both releases. Categories overlap because one declaration may have several use
sites.

| Usage | UCI 2.5 | UCI 2.6 |
|---|---:|---:|
| Used as a base | 57 | 57 |
| Base only | 18 | 18 |
| Referenced by Record field | 43 | 43 |
| Referenced by Choice alternative | 15 | 15 |
| Message payload | 0 | 0 |
| List item | 0 | 0 |
| Alias/other named reference | 0 | 0 |

Abstract declarations are therefore not base-only, even though no global
message directly names an abstract payload.

## Structural projection

`codegen-core` exposes `EffectiveStructuralType`, containing the target,
immediate base, base-to-derived `StructuralLevel` ancestry, and ordered borrowed
`StructuralSegment`s. A segment is either `RecordFields(&[FieldDecl])` or
`ChoiceAlternatives(&[FieldDecl])` and retains its owning declaration. All field
semantics and source provenance remain in the borrowed `FieldDecl`.

The UCI representatives below were inspected during the authoritative probes by
explicitly passing their qualified names to the generic structural projection
API; generic report generation does not select schema-specific declarations.

Empty local segments are omitted from `segments` but retained in `ancestry`.
Thus an empty derived Record still has its base's effective content and retains
its own identity, kind, and abstract flag. Missing/non-structural bases, cycles,
primitive structural bases, and inherited name collisions return structured
errors.

Synthetic CI tests cover standalone Record and Choice, three-level Records,
Record-to-Choice, Choice-to-Record, empty extension, forward-declared base,
abstract base metadata, malformed state, and collision rejection.

### Maximum-depth representative

Both releases project identically:

```text
DataRecordBaseType       Record 1
DataRecordListBaseType   Record 0
RecordDRLE               Record 0
AirRecordDRLE            Record 1
AirRecordDRL             Record 1
AirRecordMDT             Record 2
```

Ancestry has six declarations (five inheritance edges). Empty segments are
omitted, producing four effective Record segments and five effective members.

### Mixed-compositor representative

Both releases project `QueryPET -> QueryType` as:

```text
QueryPET   Record 0
QueryType  Choice 15
```

The effective sequence contains one 15-alternative Choice segment. The empty
Record remains in ancestry; the alternatives are never flattened into fields.

## Current backend baseline

All six pre-change authoritative generation probes first fail on:

```text
unsupported <language> IR construct: inherited structural type AccessAssessmentID_Type
```

This is the first structural-inheritance blocker for Ada, Rust, and C++ in both
releases. First-failure order is not treated as a coverage percentage.

### UCI 2.5

| Backend | Kind | Full declaration | Field type | Field occurrence | Message closure |
|---|---:|---:|---:|---:|---:|
| Ada | 4,921/5,557 (88.55%) | 1,062/5,557 (19.11%) | 12,011/13,160 (91.27%) | 6,183/13,160 (46.98%) | 0/722 (0.00%) |
| Rust | 4,921/5,557 (88.55%) | 1,753/5,557 (31.55%) | 12,011/13,160 (91.27%) | 11,227/13,160 (85.31%) | 0/722 (0.00%) |
| C++ | 4,921/5,557 (88.55%) | 1,753/5,557 (31.55%) | 12,011/13,160 (91.27%) | 11,227/13,160 (85.31%) | 0/722 (0.00%) |

### UCI 2.6

| Backend | Kind | Full declaration | Field type | Field occurrence | Message closure |
|---|---:|---:|---:|---:|---:|
| Ada | 4,928/5,570 (88.47%) | 1,051/5,570 (18.87%) | 12,126/13,198 (91.88%) | 6,185/13,198 (46.86%) | 0/725 (0.00%) |
| Rust | 4,928/5,570 (88.47%) | 1,765/5,570 (31.69%) | 12,126/13,198 (91.88%) | 11,246/13,198 (85.21%) | 0/725 (0.00%) |
| C++ | 4,928/5,570 (88.47%) | 1,765/5,570 (31.69%) | 12,126/13,198 (91.88%) | 11,246/13,198 (85.21%) | 0/725 (0.00%) |

These are historical Task 017 baseline measurements. Task 018 changes the
current backend baseline but does not revise these unavailable-root counts.
Current common capabilities now include bounded SignedInteger declarations,
enums, pure Record inheritance lowered to effective fields, named references,
SignedInteger/String primitive references,
selected optional/finite repeated cardinalities, and dependency ordering. Ada's
occurrence subset is narrower than Rust/C++. Unsupported families include Choice,
most primitive references, general/unbounded cardinality, nillability, unsupported
declaration/member constraints, and structural/abstract semantics beyond pure
Record lowering. Runtime regex and temporal parsing remain explicitly out of scope.

## Hypothetical feature impact

For the recorded Task 017 baseline, enabling any one family alone unblocks **zero**
complete message closures:

| Hypothetical family | UCI 2.5 | UCI 2.6 |
|---|---:|---:|
| Primitive declarations/references only | 0 | 0 |
| General cardinality/nillability only | 0 | 0 |
| Choice only | 0 | 0 |
| Structural inheritance/abstract only | 0 | 0 |
| Constrained simple types only | 0 | 0 |

The analyzer exhaustively evaluates all 31 non-empty combinations. Every proper
subset still unblocks zero complete closures; enabling all five hypothetically
unblocks 722/722 in 2.5 and 725/725 in 2.6. Results match across Ada, Rust, and
C++ for closure impact even though their declaration and occurrence baselines
differ.

This demonstrates strong transitive coupling, not that every feature has equal
priority. The recommended next tranche is **structural inheritance plus abstract
type lowering**: it is the universal first blocker, affects 2,026/2,036
declarations, and now has a shared compositor-preserving projection. Choice is
the next interacting structural tranche. No lowering was added in Task 017.

## Non-goals retained

## Task 018: pure Record inheritance lowering

Task 018 consumes the shared `EffectiveStructuralType` projection rather than
walking `base_type` in individual backends. For a concrete Record whose
projection contains only Record segments, Ada, Rust, and C++ emit one concrete
value layout containing the effective fields in oldest-base-to-local order.
Empty Record extensions contribute no fields. Choice segments are explicitly
rejected: flattening them would lose exclusivity semantics.

This is a backend lowering only; it does not mutate the schema IR or projection.
Native Ada tagged extension, C++ public inheritance, and Rust trait-object
storage were intentionally not selected because they would create divergent
ownership and polymorphism models without defining polymorphic value semantics.

Abstract Records are accepted only when they are actual inheritance ancestors.
They are retained in planning and their fields flow into concrete descendants,
but no standalone instantiable value declaration is emitted. A Record field or
Choice alternative naming an abstract structural target fails before rendering
with an `abstract structural value reference` diagnostic; abstract message
payloads likewise fail closed. Concrete structural base references retain the
existing exact named-value behavior; Task 018 does not infer substitution or
polymorphism from descendant existence.

The Ada repeated-field helper names are target-qualified (for example,
`Leaf_Base_Values_Sequence`) so inherited effective fields cannot collide at
package scope. The necessary existing Ada golden changed only for this helper
qualification.

The checked-in four-level synthetic fixture verifies an abstract base, empty
middle extension, finite repeated inherited field, named inherited reference,
and base-to-derived layouts for all three backends. It also verifies the
abstract-value firewall and Choice boundary. Existing standalone Choice remains
unsupported.

Public Rust CAL/`rcal` documentation remains compatibility evidence only: it
describes generated UCI types and interface/trait treatment for inherited
complex types, with dynamic treatment needed for abstract polymorphic values.
That evidence supports keeping abstract value references outside this simple
flattened-value tranche; this generator does not claim rcal API or binary
compatibility and does not copy its implementation.

The authoritative UCI roots are intentionally external and were unavailable in
this checkout, so Task 017's published UCI 2.5/2.6 inventory and coverage
tables above remain the recorded baseline. Recompute post-lowering authoritative
coverage and first-blocker progression only in an environment supplied with the
versioned UCI schema roots; no counts are inferred from the synthetic fixture.

## Task 019 Choice lowering

Task 019 lowers a Choice as a sum, never as a Record of independent optional
fields. A shared schema-neutral `effective_choice_alternatives` projection
accepts exactly one non-empty Choice segment and no non-empty Record segment.
It preserves source order and borrowed `FieldDecl` identity. Empty Record
ancestry is omitted by the structural projection, so the observed
`QueryPET -> QueryType` shape is supported; a non-empty Record plus Choice and
multiple Choice segments fail closed with distinct structural diagnostics.

- Rust emits `pub enum Choice { Alternative(Payload) }`, retaining the existing
  `Option<T>` and `BoundedVec<T, MIN, MAX>` payload occurrence lowerings.
- C++ emits a scoped named wrapper for every alternative and
  `std::variant<AlternativeA, AlternativeB> value`; wrappers preserve identity
  when alternatives share the same payload type. `<variant>` is emitted only
  for schemas containing a Choice.
- Ada emits a definite discriminated record with a default discriminant and one
  variant component per named alternative. Repeated helper names are qualified
  by both Choice and alternative.

Choice alternatives use the same nillability, constraint, abstract structural
value, type-reference, cardinality, and generated-name validation firewalls as
Record fields. Normalized alternative identifier collisions fail deterministically.
The checked-in synthetic fixture covers duplicate `Token` payload alternatives,
a Record containing a Choice, and empty-Record ancestry. It also retains a
separate non-empty Record + Choice rejection fixture.

Choice is baseline **kind renderable** because every backend has an ordinary
Choice renderer. `FeatureFamily::Choice` remains hypothetical only for the
unsupported composition boundary. Direct backend-generation regressions cover
normalized-name collisions, nillability, constraints, abstract value targets,
and finite repeated alternatives; repeated output is compiler-probed for all
three backends.

Published Task 017 authoritative evidence remains: UCI 2.5 has 420 Choice
declarations (419 standalone, 1 inherited), UCI 2.6 has 424 (423 standalone,
1 inherited); duplicate alternative names are zero; 47 Choices in each release
have duplicate payload types; 113 additional alternatives share a payload type;
repeated alternatives are 93/94; nillable alternatives are zero; and the sole
inherited Choice is `QueryType`. The authoritative UCI 2.5/2.6 roots were
available externally under `/tmp`, but validate/coverage/generation probes did
not complete within the environment's fixed command timeout. No post-Task-019
UCI metrics, blocker progression, or generation results are claimed or inferred.

## Post-Task-018 impact semantics

Task 017's impact families were hypothetical against its then-current baseline.
After Task 018, impact calculations are additive to current backend capability:
enabling a hypothetical family never removes pure Record inheritance lowering or
abstract Record ancestry support already present in the backends. In particular,
`StructuralInheritanceAndAbstract` now means the remaining unsupported
structural/abstract family beyond the current baseline, including non-pure-Record
structural shapes and polymorphic abstract values (including abstract field and
message payload positions). It is not a prerequisite for pure Record inheritance.
Likewise, after Task 019 `Choice` means the remaining unsupported Choice family
(multiple Choice segments and non-empty Record × Choice composition), not an
ordinary standalone or empty-Record-ancestry Choice. Hypothetical families are
additive and do not remove current Record or Choice lowering.

## Task 021 unbounded cardinality

Authoritative normalized-IR inventory was run against
`/tmp/ams-gra-uci-probe-2.5/UCI_MessageDefinitions_v2_5_0.xsd` and
`/tmp/ams-gra-uci-probe-2.6/UCI_MessageDefinitions_v2_6_0.xsd`. The original
Rust/C++ blocker is the `AccessAssessmentID` alternative of
`AccessAssessmentResultType`: it is a named `AccessAssessmentID_Type`,
non-nillable, unconstrained, `minOccurs=1`, `maxOccurs=unbounded` member. It
is at line 5013 in 2.5 and line 5037 in 2.6; its normalized semantics are
identical in both releases.

The complete occurrence cross-tab is:

| Release | shape | Record | Choice | total |
|---|---:|---:|---:|---:|
| 2.5 | 1..1 | 4,508 | 1,375 | 5,883 |
| 2.5 | 0..1 | 4,949 | 0 | 4,949 |
| 2.5 | 0..N | 300 | 0 | 300 |
| 2.5 | 1..N | 82 | 9 | 91 |
| 2.5 | min>1..N | 4 | 0 | 4 |
| 2.5 | 0..* | 1,390 | 0 | 1,390 |
| 2.5 | 1..* | 451 | 80 | 531 |
| 2.5 | min>1..* | 8 | 4 | 12 |
| 2.6 | 1..1 | 4,531 | 1,388 | 5,919 |
| 2.6 | 0..1 | 4,967 | 0 | 4,967 |
| 2.6 | 0..N | 266 | 0 | 266 |
| 2.6 | 1..N | 81 | 9 | 90 |
| 2.6 | min>1..N | 4 | 0 | 4 |
| 2.6 | 0..* | 1,406 | 0 | 1,406 |
| 2.6 | 1..* | 453 | 81 | 534 |
| 2.6 | min>1..* | 8 | 4 | 12 |

These reconcile with Task 017's unbounded totals: 1,933 in 2.5 and 1,952 in
2.6, with 16 members having `minOccurs > 1` in each. Their min distributions
are 2.5: Record 0/1/2/3 = 1,390/451/6/2 and Choice = 0/80/3/1; 2.6: Record =
1,406/453/6/2 and Choice = 0/81/3/1. All are non-nillable; 23 unbounded
members have local constraints in each release. Named/primitive distributions
are 1,907/26 (2.5) and 1,927/25 (2.6). There are 84/85 unbounded Choice
alternatives, all named; deterministic order begins with
`AccessAssessmentResultType.AccessAssessmentID`.

`Cardinality::shape()` is a schema-neutral, lossless classifier with required,
optional, bounded, and unbounded variants. Rust lowers unbounded values to a
private `UnboundedVec<T, MIN>` and C++ to a private `UnboundedVector<T, Min>`;
both expose checked construction and read-only access, retain `minOccurs`, and
preserve constrained scalar element wrappers. Ada lowers only `0..*` to an
owner-qualified `Ada.Containers.Vectors` instantiation. Positive-minimum Ada
unbounded occurrences remain fail-closed because a public Vector alone cannot
enforce the lower bound.

Nillability remains unsupported. Finite bounded and optional lowerings are
unchanged. Consequently `CardinalityAndNillability` now denotes remaining
unsupported occurrence semantics (not all unbounded cardinality): nillability,
Ada's unsupported positive-minimum unbounded forms, and its existing narrower
optional policy.

### Corrective unbounded-boundary hardening

The full schema `u64` `minOccurs` domain remains supported. C++ emits the
existing portable unsigned constant expression for an extreme unbounded
minimum, including `std::numeric_limits<std::uint64_t>::max()` for `u64::MAX`.
Rust converts the schema minimum into `usize` before comparing it with a
`Vec` length: conversion failure means the platform cannot represent a large
enough `Vec`, while no representable length is artificially capped. This does
not expand Task 021 support or alter analyzer semantics.

## Task 023 — Ada minimum-preserving repeated cardinality

Ada now preserves repeated minima structurally. Finite `MIN..MAX` values use a
fixed `Positive range 1 .. MAX` array and `Length : Natural range MIN .. MAX :=
MIN`. Positive unbounded values use a fixed required prefix of exactly `MIN`
items plus an `Ada.Containers.Vectors` additional tail; logical order is the
prefix followed by tail vector order. `0..*` retains the Task 021 vector form.

The shared portable array bound is 32,767: conforming Ada guarantees
`Standard.Integer` includes `-32_767 .. 32_767`, hence `Positive` includes this
range. Finite maxima and positive unbounded minima above it fail closed.
Nillability and Ada optional named/non-String values remain unsupported.
`CardinalityAndNillability` now covers those remaining occurrence semantics,
including bounds above this portable limit, rather than ordinary supported
positive-minimum repetition.

Fresh UCI inventory confirms the Task 021 cross-tabs: 2.5 finite Record/Choice
`0..N=300/0`, `1..N=82/9`, `min>1..N=4/0`; unbounded `1390/0`, `451/80`,
`8/4`. 2.6 is `266/0`, `81/9`, `4/0`; and `1406/0`, `453/81`, `8/4`.
Unbounded positive minima are only 1, 2, and 3. The former Ada blocker,
`AccessAssessmentResultType.AccessAssessmentID` (`1..*`, named,
non-nillable, default constraints), now lowers successfully. All three
backends next stop at abstract structural value reference `CapabilityCommandBaseType`.


## Task 024 — closed abstract structural values

Task 024 adds a schema-neutral closed-value projection for an abstract
structural declaration used in a value position. Only Record fields, Choice
alternatives, message payloads, and naturally represented Alias/List targets
are value uses; `base_type` ancestry alone never creates a wrapper. Every
concrete transitive structural descendant becomes a variant in deterministic
Schema IR order. Concrete non-leaf descendants are included and abstract
intermediates are excluded.

Rust emits an enum whose payloads are concrete values; C++ emits a struct
containing `std::variant`; Ada emits a discriminated record. These are closed,
schema-known sums: no trait objects, vtables, classwide access values, owning
pointers, or heap fallback are introduced. Existing optional and repeated
cardinality lowerings compose with the wrapper value unchanged. Rust derives
`Eq` only when every concrete payload is Eq-capable.

The shared generated-entity plan orders concrete descendants before their
wrapper and wrappers before by-value consumers. Flattened structural inheritance
is deliberately not a generated containment edge. Backends validate semantic
uses before planning all generated entities, preserving deterministic first
diagnostics when a later target is unsupported.

`CapabilityCommandBaseType` is first used by
`ActivityChoiceType.CapabilityCommand` as a required, non-nillable Choice
alternative in both roots (2.5 line 6752; 2.6 line 6763). Its closed projection
contains 24 concrete descendants in 2.5 and 23 in 2.6; the
`RF_SharedApertureCapabilityCommandBaseType` abstract intermediate is not a
variant. All three post-change UCI 2.5 generators move past it and next reject
the unrelated Binary primitive. All three UCI 2.6 generators move past it and
next reject `CommSupportPointingActivityEXT`, a genuine abstract value target
with no concrete descendants. Empty projections and recursive by-value graphs
remain fail-closed; no empty sum is generated.

`CommSupportCapabilityEXT` is also a genuine later value target, not base-only:
it is the optional `CommSupportCapabilityType.ExtensionData` Record field
(2.5 line 20531; 2.6 line 20540), has no descendants, and therefore remains a
valid fail-closed boundary when reached. `StructuralInheritanceAndAbstract` now
denotes remaining unsupported structural/abstract cases such as empty or
recursive closed sums, rather than ordinary acyclic abstract values.

Both authoritative roots contain 70 abstract declarations and 52 distinct
abstract value targets (15 Choice alternatives and 43 Record fields; no message
payload or List uses). Each has 29 acyclic targets, 13 zero-descendant targets,
and 10 targets whose generated-value closure reaches recursion. The zero-target
names are `CommSupportCapabilityEXT`, `CommSupportCapabilityStatusEXT`,
`CommSupportCommandEXT`, `CommSupportCommandStatusEXT`,
`CommSupportPlanningStatusEXT`, `CommSupportPointingActivityEXT`,
`CommSupportPointingEXT`, `CommSupportStatusEXT`, `CommSupportTaskEXT`,
`CommSupportWindowEXT`, `ConstraintEXT`, `OpNotificationEXT`, and
`SourceCommandEXT`. Concrete-descendant counts have maximum depth five; the
largest families contain 722 (2.5) / 725 (2.6) descendants. The complete count
distribution is `0:13, 1:12, 2:5, 3:7, 5:2, 6:2, 7:3, 8:1, 13:1, 22:1,
24/23:1, 32:1, 33:1, 74:1, 722/725:1`.

Fresh coverage completed without a fatal cycle. Relative to the Task 024
baseline, declaration kinds, field types, occurrences, and message closures
are unchanged. Fully renderable declarations are Ada `2761 -> 2754`, Rust/C++
`5273 -> 5332` for UCI 2.5, and Ada `2762 -> 2755`, Rust/C++ `5298 -> 5357`
for UCI 2.6; message closures remain `0/722` and `0/725` because unrelated
transitive capabilities remain unsupported. Recursive descendant requirements
can make a wrapper non-baseline-renderable in coverage even when ordered backend
validation progresses past an earlier use before global planning is reached.

The Ada net change is gross membership churn, not seven lost capabilities, and
is identical in 2.5 and 2.6. Fifteen formerly counted abstract wrappers are now
excluded: `CapabilityBaseType`, `CommWaveformActivityCommandPET`,
`CommWaveformActivityPET`, `CommWaveformCapabilityCommandPET`,
`ComponentExtendedStatusPET`, `DataLinkIdentifierPET`,
`DataLinkNativeFilterPET`, `DataLinkNativeInfoPET`,
`GatewayConfigurationPET`, `GatewayNativeStatisticsPET`,
`NITF_PackingPlanPET`, `ProcessingParametersPET`,
`STANAG_4607_PackingPlanPET`, `SubsystemExtendedStatusPET`, and
`SupportCapabilityCommandBaseType`. Each is abstract, has a non-empty direct
concrete-descendant projection, and is now correctly non-renderable because at
least one concrete descendant has an unsupported abstract-value closure. These
are unsupported-descendant topology corrections (not empty or directly
recursive wrappers). The previous Ada backend already rejected effective
abstract Record/Choice value references, so the former coverage count was an
overcount rather than generated Ada capability.

Eight concrete consumers are newly renderable in both releases:
`AssessmentRequestType` (`AchievabilityAssessmentRequestPET`, 3 variants),
`AssessmentType` (`AchievabilityAssessmentPET`, 3), `DataUpdateRequestType`
(`QuerySpecificDataPET`, 5), `EntityMetadataMDT` (`EntityMetadataPET`, 1),
`OpPointReferenceType` (`DataLinkIdentifierPET`, 8), `OpZoneCategoryType`
(`OpZoneFilterAreaPET`, 3), `OrderOfBattleML` (`RecordDRLE`, 22), and
`SystemMetadataMDT` (`SystemMetadataPET`, 1). These are actual acyclic
closed-sum gains: their required occurrence forms and concrete descendant
closures are renderable. Thus Ada is `15 removed - 8 added = net -7`; Rust and
C++ gain 59 declarations because their existing cardinality support lets the
same 29 acyclic wrappers and a larger set of downstream consumers become
renderable. Neither analysis treats every abstract declaration as supported.

The normalized Schema IR remains valid for recursive polymorphic families. For
example, the generated closed-value graph contains
`SubsystemMaintenanceTestCommandPET -> SubsystemMaintenanceTestCommandType ->
SubsystemMaintenanceSubtestCommandChoiceType ->
SubsystemMaintenanceTestCommandPET`: the first edge is the wrapper's virtual
concrete-variant edge, while the latter two are effective member references.
This requires indirection for a finite generated representation, which Task 024
does not add. The wrapper is baseline unsupported and generation fails closed
when it is demanded; coverage classifies and inventories this representation
boundary while continuing the full report. Raw Schema IR dependency-cycle
diagnostics remain unchanged.
Measured Ada coverage moved from declarations `2422 -> 2761` and field
occurrences `7573 -> 8211` in UCI 2.5, and `2421 -> 2762` plus
`7591 -> 8231` in UCI 2.6. Declaration kinds, field types, and message
closures were unchanged.

## Task 025 — unconstrained binary octet-sequence lowering

`PrimitiveKind::Binary` is now a baseline backend value in Ada, Rust, and C++,
but only when its `ConstraintSet` is exactly `ConstraintSet::default()`. Binary
denotes an owned sequence of octets, never a hexadecimal or base64 lexical
string, a borrowed pointer/span, a null-terminated byte string, or a wider
integer array. `Binary` continues to originate exclusively from `xs:hexBinary`;
the authoritative UCI 2.5 and UCI 2.6 roots contain no `xs:base64Binary`, and
Task 025 does not broaden the frontend's `xs:hexBinary -> PrimitiveKind::Binary`
mapping or add a second Binary IR kind. Existing frontend/IR tests already
established that Binary `length`, `minLength`, and `maxLength` are measured in
octets, not hexadecimal lexical characters; Task 025 does not implement any of
those three constraints, but it does give unconstrained Binary an actual 8-bit
element type so a later length-constraint tranche has a semantically natural
base to constrain.

Authoritative raw `xs:hexBinary` inventory (fresh Task 025 probes): UCI 2.5 has
5 direct local-field uses and 3 restriction bases; UCI 2.6 has 0 direct
local-field uses and 4 restriction bases. Normalized-IR inventory reconciles
these: UCI 2.5 shows `kind.Primitive.Binary: 3` named declarations and
`members.primitive_references.Binary: 5` direct field references (matching the
5 raw local-field uses exactly, with 3 Binary restriction bases normalizing to
3 named Binary declarations). UCI 2.6 shows `kind.Primitive.Binary: 5` named
declarations (its 4 restriction bases plus the zero-facet `HexBinaryType`
alias) and `members.primitive_references.Binary: 0` direct field references,
matching the observed absence of direct local-field `xs:hexBinary` elements in
that release.

The exact UCI 2.5 first Binary blocker (identical across Ada, Rust, and C++)
is the `HexBinaryValue` alternative of the `AtomicValueType` Choice declaration
(`UCI_MessageDefinitions_v2_5_0.xsd:12893`, contained in the `AtomicValueType`
Choice starting at line 12808): a direct `Primitive(Binary)` Choice
alternative, required (Choice-alternative occurrence), not nillable, with
`ConstraintSet::default()`. All three backends previously reported
`unsupported {Ada,Rust,C++} IR construct: type reference Primitive(Binary)`
for this value; after Task 025 all three progress past it.

Rust maps a direct unconstrained Binary reference to an owned `Vec<u8>`.
Required, optional, finite-repeated, and unbounded Binary fields reuse the
existing `Option<T>`/`BoundedVec<T, MIN, MAX>`/`UnboundedVec<T, MIN>` occurrence
wrappers with `T = Vec<u8>`; the outer wrapper counts Binary *values*, and each
`Vec<u8>` independently counts *octets* inside one value — these are
orthogonal dimensions, and Task 025 verifies a repeated-Binary fixture keeps
`[[0x01, 0x02], [0x03, 0x04, 0x05]]` as two distinct byte sequences rather than
one flattened sequence. A named unconstrained Binary declaration renders as
`pub struct Name(Vec<u8>)` with `new`, `as_slice`, and `into_vec`, deriving
`Debug, Clone, PartialEq, Eq`; `PrimitiveKind::Binary` was already Eq-capable
under the Task 022/024 transitive-Eq analysis catch-all, so a Record/Choice/
abstract wrapper containing only Eq-capable data including Binary retains Eq
without further change.

C++ maps a direct unconstrained Binary reference to
`std::vector<std::uint8_t>`. The C++ backend already emits `#include
<cstdint>` and `#include <vector>` unconditionally, so a Binary-only schema
(one namespace, one Record, one required direct Binary field, no Choice, no
repeated cardinality, no abstract values) needs no new include-detection logic
and is verified to strictly compile with `c++ -std=c++17 -Wall -Wextra
-pedantic-errors`. A named unconstrained Binary declaration renders as a class
owning a `std::vector<std::uint8_t> value_` with an explicit constructor and a
`const&` accessor; no raw owning pointer, span/view storage, or
null-termination semantics are introduced, and this remains valid C++17.

Ada uses `Interfaces.Unsigned_8` as the octet element type (never `Character`,
`String`, `Interfaces.Unsigned_16`, or `Integer`). Task 025 emits exactly one
shared `Binary_Vectors` package instantiation
(`Standard.Ada.Containers.Vectors (Index_Type => Natural, Element_Type =>
Interfaces.Unsigned_8, "=" => Interfaces."=")`) per generated unit, only when
Binary is actually used, following the project's established root-Ada-safe
`Standard.Ada.Containers.Vectors` qualification because generated namespaces
may end in `Ada`. A direct unconstrained Binary field's type is
`Binary_Vectors.Vector`; a named unconstrained Binary declaration is a simple
record wrapper `type Name is record Value : Binary_Vectors.Vector; end
record;` rather than a direct derivation from the private `Vector` type. GNAT
proved that a nested vector element type built from `Binary_Vectors.Vector`
(finite-repeated and unbounded-repeated Binary fields) requires an explicit
`"=" => Binary_Vectors."="` actual, mirroring the existing `Interfaces."="`
pattern already used for `UnsignedInteger`/`Float32`/`Float64` element types;
this was added only after a real GNAT compilation failure, not speculatively.
Unconstrained Binary itself is dynamically sized and does not acquire the
Task 023 32,767 portable-array-bound limit; that bound applies only to the
chosen fixed-array representation for finite repeated *cardinality*, which is
an orthogonal dimension from the byte length inside one Binary value.

Ada optional Binary remains unsupported: Task 025 does not broaden Ada's
existing narrower optional-value policy, and an `OccurrenceShape::OptionalOne`
Binary field fails closed with `unsupported Ada IR construct: cardinality on
field ...`, exactly as any other Ada-unsupported optional non-String primitive
already did. Rust and C++ retain their existing, broader optional support for
Binary. Nillable Binary remains unsupported in all three backends, unchanged
from prior primitives.

Any non-default Binary `ConstraintSet` — `length`, `minLength`, `maxLength`,
lexical constraints, or a malformed numeric constraint from a hypothetical
external IR construction — remains fail-closed for both named Binary
declarations and direct Binary fields, in all three backends. No backend
silently degrades a constrained Binary value to an unconstrained
`Vec<u8>`/`std::vector<std::uint8_t>`/`Binary_Vectors.Vector`; each backend's
existing declaration- and field-level constraint firewalls
(`reject_any_constraints`, `reject_extra_constraints`) already covered this,
and Task 025 adds an explicit whole-`ConstraintSet` check ahead of them for
Binary declarations to make the firewall self-documenting. Direct regressions
exist per backend for both constrained named Binary declarations and
constrained direct Binary fields.

`PrimitiveExpansion` no longer means Binary. After Task 025, baseline analyzer
support already includes Boolean, SignedInteger, UnsignedInteger, Float32,
Float64, and Binary; `PrimitiveExpansion` now represents the remaining
unsupported primitive families — Decimal, DateTime, Time, and Duration — plus
any equivalent still-unimplemented primitive kind. `ConstrainedSimpleTypes`
retains constrained-Binary length/minLength/maxLength as a hypothetical
unblocking family: unconstrained Binary is baseline, while `Binary(length=N)`
remains non-baseline until `ConstrainedSimpleTypes` is hypothetically enabled.
Tests that previously used Binary as the canonical PrimitiveExpansion-only
example now use Duration instead; coverage regressions confirm unconstrained
Binary is baseline-renderable in every one of the 31 non-empty feature-family
combinations and that constrained Binary alone is unblocked only by
`ConstrainedSimpleTypes`.

Fresh post-Task-025 authoritative probes: UCI 2.5 Ada, Rust, and C++ all move
past `Primitive(Binary)` and reach the same Task 024 boundary — `unsupported
abstract structural value: abstract value target
CommSupportPointingActivityEXT has no concrete structural descendants` — for
all three languages. UCI 2.6 Ada, Rust, and C++ report the identical
`CommSupportPointingActivityEXT` blocker unchanged from the Task 024 baseline,
confirming Task 025 does not alter Task 024's fail-closed boundary. Solving
that zero-descendant abstract-value boundary remains explicitly out of Task
025's scope.

Normalized frontend counts are unchanged by Task 025: UCI 2.5 remains 5,557
types / 722 messages, and UCI 2.6 remains 5,570 types / 725 messages, because
no frontend or IR semantic changed.

Measured coverage before/after (declaration kinds / fully renderable
declarations / field-type references / field occurrences / message closures,
all `renderable/total`): UCI 2.5 Ada moved `5425/5557 -> 5428/5557` kinds,
`2754/5557 -> 2755/5557` declarations, `13142/13160 -> 13147/13160` field
types, and field occurrences/message closures were unchanged at
`8211/13160`/`0/722`; UCI 2.5 Rust and C++ moved `5425/5557 -> 5428/5557`
kinds, `5332/5557 -> 5336/5557` declarations, `13142/13160 -> 13147/13160`
field types, with field occurrences/message closures unchanged at
`13160/13160`/`0/722`. UCI 2.6 Ada moved `5436/5570 -> 5441/5570` kinds,
`2755/5570 -> 2756/5570` declarations, field types were already
`13198/13198 -> 13198/13198` (no direct local `xs:hexBinary` fields in 2.6),
occurrences/closures unchanged at `8231/13198`/`0/725`; UCI 2.6 Rust and C++
moved `5436/5570 -> 5441/5570` kinds, `5357/5570 -> 5358/5570` declarations,
field types unchanged at `13198/13198`, occurrences/closures unchanged at
`13198/13198`/`0/725`. As predicted, UCI 2.5 direct Binary field-type coverage
improves (5 newly renderable references, matching
`members.primitive_references.Binary: 5`), and UCI 2.6 — which has no direct
local `xs:hexBinary` fields — instead gains declaration-kind and
fully-renderable-declaration coverage from its unconstrained named Binary
declarations. Message closures remain `0/722` and `0/725` in both releases
because the unrelated `CommSupportPointingActivityEXT` zero-descendant
abstract-value boundary is unaffected by Task 025.

## Task 026 — uninhabited abstract structural values

Task 026 classifies an abstract structural declaration with zero concrete
structural descendants as *uninhabited*: its payload set is empty, so no legal
value can ever be constructed for it. This is a classification, not a schema
error — a well-formed schema may declare an abstract extension point that
nothing extends in the current closed set.

### Supported shape and its lowering

Exactly one occurrence shape becomes supported: a Record field whose target is
uninhabited and whose occurrence is

- `minOccurs == 0` and `maxOccurs <= 1` (optional, single),
- not nillable, and
- carrying default (empty) local constraints.

Such a field's only legal state is absence, so all three backends elide it:
Rust emits no struct field, C++ emits no data member, Ada emits no record
component, and the planner emits no wrapper type and no dependency edge for the
uninhabited target. The Schema IR is not mutated; this is a codegen-boundary
projection only.

The shared decision point is `field_storage_semantics` in
`codegen-core::abstract_value`, returning `EffectiveValueMember::Stored` or
`EffectiveValueMember::AbsentOnly`. Every backend and the emission planner
consult it, so no backend re-derives elision independently.

Generated-name preflight consults it too. `backend_names::register_declaration_members`
classifies each effective Record field through the very same
`field_storage_semantics` call before registering anything, so a field the
backends store nowhere reserves no member identifier, no repeated helper, and
no Task 034 `{Owner}_{Member}_Optional` wrapper. Registering names from the raw
effective fields described output that cannot exist: it could falsely reject a
user declaration spelled like a phantom wrapper, and it could report a name
collision in place of the authoritative semantic diagnostic for a field whose
storage classification fails (for example an open-extensions abstract value).
Deferral is field-scoped — every other field and declaration is still fully
name-checked.

### Deliberately still unsupported (fail-closed)

| Occurrence of an uninhabited target | Status | Reason |
|---|---|---|
| positive minimum (`1..1`, `2..5`, …) | unsupported | requires an inhabitant that cannot exist |
| repeated `0..*` / `0..n` | unsupported | would need an element type that is uninhabited; no evidence that "always empty collection" is intended |
| nillable optional | unsupported | needs an explicit nil-versus-absent distinction |
| optional with local constraints | unsupported | constraints cannot be silently discarded |
| Choice alternative | unsupported | effectively required position |
| message payload | unsupported | effectively required position |

The emission planner only skips a zero-descendant wrapper after
`ensure_zero_descendant_target_only_used_as_absent_only` confirms every
reference to that target in the whole schema (Record fields, Choice
alternatives, message payloads, bases, aliases, list items) is an absent-only
Record field. Any other use re-raises the original `NoConcreteDescendants`
diagnostic unchanged, preserving Task 024's error text.

Ada gains no special exception: once a concrete descendant exists, the field is
an ordinary optional named value again and Ada's pre-existing
"cardinality on field" general-optional boundary applies, exactly as before.

### Authoritative evidence

Zero-descendant target inventory is unchanged by this task —
`abstract_value.zero_descendant_targets: 13` in both UCI 2.5 and UCI 2.6 —
confirming the abstract-value topology itself was not altered. Normalized
frontend counts are likewise unchanged: UCI 2.5 remains 5,557 types / 722
messages and UCI 2.6 remains 5,570 types / 725 messages.

Measured coverage before/after (declaration kinds / fully renderable
declarations / field-type references / field occurrences / message closures):

| Release / backend | Before | After |
|---|---|---|
| 2.5 Ada | `2755/5557` declarations | `2770/5557` declarations |
| 2.5 Rust | `5336/5557` declarations | `5365/5557` declarations |
| 2.5 C++ | `5336/5557` declarations | `5365/5557` declarations |
| 2.6 Ada | `2756/5570` declarations | `2771/5570` declarations |
| 2.6 Rust | `5358/5570` declarations | `5387/5570` declarations |
| 2.6 C++ | `5358/5570` declarations | `5387/5570` declarations |

Declaration kinds, field-type references, and field occurrences are unchanged in
both releases (2.5: kinds `5428/5557`, field types `13147/13160`, occurrences
Ada `8211/13160` and Rust/C++ `13160/13160`; 2.6: kinds `5441/5570`, field types
`13198/13198`, occurrences Ada `8231/13198` and Rust/C++ `13198/13198`). This is
expected: the task changes whether an owning declaration is renderable, not the
classification of individual field types or occurrences. Hypothetical
feature-combination attributions that include constrained simple types rise from
`653` to `676` in UCI 2.5, reflecting the newly unblocked declarations.

Message closures remain `0/722` and `0/725` in both releases.

### Next authoritative blocker (not addressed here)

Before this task, all six probes (UCI 2.5 and 2.6 × Ada/Rust/C++) stopped at
`CommSupportPointingActivityEXT`. After it, all six progress past that value and
stop at `SourceCommandEXT`, a `0..unbounded` repeated occurrence of a
zero-descendant abstract target.

That blocker is intentionally **not** implemented by Task 026: a repeated
occurrence of an uninhabited value would require choosing a representation for a
collection whose element type has no inhabitants, and no authoritative evidence
yet establishes that an always-empty collection is the intended meaning. It is
recorded here as the next decision point for a future task.

## Task 027 open extension-point semantics

Task 027 is an evidence and architecture task. **No backend behavior, coverage
behavior, or Task 026 implementation changed**, so every measurement in the
Task 026 section above remains current. Full evidence is in
`docs/task-027-open-extension-points.md`.

### Task 026 is confirmed correct under its stated assumption

Task 026 correctly implemented its explicit closed-schema assumption. Given that
only types declared in the supplied schema set may ever appear in a value, an
optional occurrence of a zero-descendant abstract target genuinely has exactly
one legal state, and eliding its storage is right. Task 026 also fails closed on
every other occurrence shape and verifies whole-schema usage before skipping a
wrapper. None of that is revised here.

### Task 027 limits that assumption to closed-world generation

Task 027 does **not** confirm the closed-schema assumption as general UCI
semantics. Measured from the pinned authoritative roots:

- all 13 zero-descendant abstract targets are `abstract="true"`, carry
  `uci:version="000.000.000.000"`, have no base type and no local fields, and
  are identical in 2.5 and 2.6 (topology count reconfirmed at 13);
- 12 of 13 are documented as explicit open extension points; the 13th
  (`ConstraintEXT`) is an open-ended generic description; **none** is documented
  as reserved, unused, or uninhabited;
- `SourceCommandEXT` (2.5 line 95638, 2.6 line 95712) is documented as "a point
  of abstract extension to create SourceCommands that can't be documented in the
  open, unclassified UCI schema";
- the ten `CommSupport*EXT` types instruct *adopting programs* to define their
  own details here;
- neither root ever uses `block` or `final`, so XSD derivation and substitution
  at these types are deliberately left unrestricted.

The conclusion is a **Decision B — open world**: the loaded XSD dependency graph
is closed as a *file set*, but the UCI *type universe* is not. External, private,
program-specific schemas may legally supply derived extension types.

Consequently Task 026's optional elision is recorded as **closed-world-only
behavior**. Under general UCI/CAL interoperability an elided `ExtensionData`
field could not represent an extension value a conforming peer legitimately
sent. That risk is currently unexercised — this generator emits no codec and no
runtime — but it is a real correctness boundary for future runtime work and is
deliberately not papered over.

### Why SourceCommandEXT remains fail-closed

The `0..unbounded` `DisseminationSubplanType.ExtensionCommand` occurrence
(2.5 line 30929, 2.6 line 30919; `minOccurs=0`, `maxOccurs=unbounded`,
non-nillable, unconstrained, Record not Choice) is **not** lowered as an
always-empty collection, because the evidence says the opposite of "always
empty":

- the member's own documentation repeats the extension semantics at the use
  site;
- secondary public UCI CAL 2.3.2 evidence (`Santiago010/Uci-Cal-api` @
  `87409b91e163931b6ac0905134367e5fb48729c9`, clearly **secondary** because it
  is an older release) exposes `ExtensionCommand` as a mutable
  `BoundedList<SourceCommandEXT>` whose `resize(size, accessorType)` explicitly
  accepts "a accessor derived from the BoundedList's base type", alongside
  `push_back` and `setExtensionCommand`, and whose JSON/XML serializers iterate
  every entry and grow the list on deserialize.

An always-empty lowering would therefore have been wrong, so all six probes
(UCI 2.5 and 2.6 × Ada/Rust/C++) continue to stop at the unchanged Task 024
diagnostic:

```text
error: unsupported abstract structural value: abstract value target SourceCommandEXT has no concrete structural descendants
```

No behavior keys on the `EXT` name suffix, and none may: the schema contains no
machine-readable discriminator for extension points. `substitutionGroup`,
`block`, `final`, `xs:appinfo`, `xs:import`, and `xs:any` are used zero times in
both roots, the only custom attribute anywhere is `uci:version`, and
`uci:version="000.000.000.000"` is shared with 1,098 ordinary inhabited types in
2.5 alone, so it cannot serve as one either.

## Task 028 — explicit generation world policy

Task 027 proved that the loaded XSD dependency graph is a closed **file** set
while the legal **type** universe is not necessarily closed, and that no
reliable machine-readable per-type discriminator distinguishes an open extension
point from an ordinary abstract type. Task 024 closed sums and Task 026 optional
elision are therefore correct only when the caller has actually asserted a
closed type universe.

Task 028 makes that assertion explicit rather than implicit. It implements
ADR-0004 recommendations 1 and 2 and validates recommendation 3 synthetically.
It does **not** implement runtime-polymorphic open extensions.

### The policy

One language-neutral `codegen-core` enum, shared by all three backends:

```text
GenerationWorld::ClosedSchemaSet
GenerationWorld::OpenExtensions
```

`ClosedSchemaSet` means the caller asserts the supplied `SchemaIr` contains
every concrete type that may legally inhabit an abstract value.
`OpenExtensions` means additional concrete derived types may exist outside it.
There is no per-backend world enum and no per-backend default; Ada, Rust, and
C++ cannot disagree about which values exist.

The policy is never stored in `SchemaIr` and never inferred — not from import
count, namespace count, `uci:version`, type names, or documentation text.

### CLI syntax

`generate` and `coverage` **require** `--world`; there is no implicit default:

```text
ams-gra-codegen-oms generate --schema ROOT --language LANG --output DIR --world closed-schema
ams-gra-codegen-oms coverage --schema ROOT --world open-extensions
```

`validate` takes no `--world`, because schema validity is independent of
generation policy. Observed usage behaviour (all exit code 2):

```text
$ ams-gra-codegen-oms generate --schema ... --language rust --output ...
error: missing required option '--world'

$ ams-gra-codegen-oms coverage --schema ...
error: missing required option '--world'

$ ams-gra-codegen-oms generate ... --world closed
error: unsupported generation world 'closed'; expected one of: closed-schema, open-extensions
```

`ams-gra-codegen-oms validate --schema ROOT` continues to succeed with exit 0.

### Closed-world preservation

Under `--world closed-schema` nothing about Task 024 or Task 026 changed:
descendant ordering, concrete non-leaf inclusion, abstract intermediate
exclusion, emission order, the recursive fail-closed boundary, Rust `Eq` logic,
the C++ variant, the Ada discriminated record, and absent-only storage elision
are all byte-for-byte preserved. Coverage under `closed-schema` reproduces the
Task 026 baseline metrics exactly, apart from the added world marker line.

### Open-world conservative behaviour

Under `--world open-extensions`:

- **any** abstract structural **value** reference fails closed — with zero, one,
  or many known descendants alike, because a known descendant set is never
  assumed exhaustive;
- Task 026 absent-only elision is disabled: a zero-known-descendant optional
  field is not absent-only, because an external derived type may legally make it
  present, so eliding its storage would silently lose data;
- Task 024 closed sums are not emitted, since they would not be exhaustive;
- abstract declarations used only as inheritance ancestry remain fully
  supported, and ordinary concrete values are unaffected;
- a schema with no abstract value reference generates byte-identical output to
  closed mode.

The shared diagnostic names the target and the policy, does not claim the schema
is invalid, and proposes no placeholder:

```text
error: unsupported abstract structural value: abstract value <Name> is not closed under open-extensions generation; external derived types cannot be represented
```

### No EXT heuristic

Nothing keys on the `EXT` suffix, on a UCI target list, or on documentation
text. A synthetic `Base` with ordinary descendants fails identically to
`SourceCommandEXT`, and regressions in all three backends assert this.

### Private same-namespace extension overlay

A generic fixture (`private-extension-overlay/`, no UCI names) validates
ADR-0004 option 3 using existing `xs:include`/schema-set support:

```text
root.xsd
  includes public-base.xsd      -> abstract ExtensionBase
                                   Container.ExtensionCommand : ExtensionBase 0..unbounded
  includes private-extension.xsd -> PrivateExtension extends ExtensionBase
```

- **closed-schema:** `ExtensionBase` now has a known concrete descendant, so
  ordinary Task 024 closed-sum lowering applies and composes with repeated
  cardinality. Ada, Rust, and C++ all generate the private descendant as a
  variant. This is the supported route to real extension support today, with no
  new generator code.
- **open-extensions:** still fails closed. Including one private descendant does
  not prove that no *other* external descendant exists. A caller who believes
  the public+private set is complete must say so with `--world closed-schema`.

A companion public-only fixture (same base, no private overlay) shows the
`0..unbounded` zero-descendant abstract value remains unsupported in **both**
worlds: Task 028 deliberately did not extend Task 026 to repeated always-empty
collections. Only the diagnostic differs (zero-descendant versus open-world).

### Cross-namespace extensions remain out of scope

Backends still require a single namespace. Supporting a private derived type
declared in a *different* target namespace would require multi-namespace backend
generation, which Task 028 deliberately did not broaden into.

### No runtime open polymorphism

No trait objects, `Box`, `Rc`/`Arc`, C++ owning polymorphic pointers, Ada
classwide access, extension registry, plugin registration, opaque payloads,
codecs, or `xsi:type` handling were added. `--world open-extensions` is strictly
*more* restrictive than `--world closed-schema`; it exists so the generator can
be honest about an assumption it cannot verify.

### Authoritative generation matrix

Binary `36d82a8`, run against the pinned probe roots
`/tmp/ams-gra-uci-probe-2.5/UCI_MessageDefinitions_v2_5_0.xsd` and
`/tmp/ams-gra-uci-probe-2.6/UCI_MessageDefinitions_v2_6_0.xsd`. All twelve cells
exit 1 (execution error, not usage error).

| Release | World | Backend | Exit | First blocker | Diagnostic class |
| --- | --- | --- | --- | --- | --- |
| 2.5 | closed-schema | Ada | 1 | `SourceCommandEXT` | zero-descendant (Task 024) |
| 2.5 | closed-schema | Rust | 1 | `SourceCommandEXT` | zero-descendant (Task 024) |
| 2.5 | closed-schema | C++ | 1 | `SourceCommandEXT` | zero-descendant (Task 024) |
| 2.6 | closed-schema | Ada | 1 | `SourceCommandEXT` | zero-descendant (Task 024) |
| 2.6 | closed-schema | Rust | 1 | `SourceCommandEXT` | zero-descendant (Task 024) |
| 2.6 | closed-schema | C++ | 1 | `SourceCommandEXT` | zero-descendant (Task 024) |
| 2.5 | open-extensions | Ada | 1 | `CapabilityCommandBaseType` | open-world (Task 028) |
| 2.5 | open-extensions | Rust | 1 | `CapabilityCommandBaseType` | open-world (Task 028) |
| 2.5 | open-extensions | C++ | 1 | `CapabilityCommandBaseType` | open-world (Task 028) |
| 2.6 | open-extensions | Ada | 1 | `CapabilityCommandBaseType` | open-world (Task 028) |
| 2.6 | open-extensions | Rust | 1 | `CapabilityCommandBaseType` | open-world (Task 028) |
| 2.6 | open-extensions | C++ | 1 | `CapabilityCommandBaseType` | open-world (Task 028) |

Closed-schema reproduces the Task 026 blocker byte-for-byte, confirmed against a
fresh pre-change baseline binary built from `bec71c5`:

```text
error: unsupported abstract structural value: abstract value target SourceCommandEXT has no concrete structural descendants
```

Open-extensions blocks *earlier*, at the first abstract **value** reference in
declaration order — `CapabilityCommandBaseType`, which Task 023 identified as
the first abstract-value blocker before Task 024 closed-sum support existed.
Because that target has known concrete descendants, it correctly uses the
open-world diagnostic rather than the zero-descendant one:

```text
error: unsupported abstract structural value: abstract value CapabilityCommandBaseType is not closed under open-extensions generation; external derived types cannot be represented
```

This expectation is evidence, not production logic: nothing in the generator
hardcodes either target name.

### Authoritative coverage matrix

| Release | World | Backend | Kinds | Full declarations | Field types | Field occurrences | Message closures |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 2.5 | closed | Ada | 5428/5557 | **2770**/5557 | 13147/13160 | 8211/13160 | 0/722 |
| 2.5 | closed | Rust | 5428/5557 | **5365**/5557 | 13147/13160 | 13160/13160 | 0/722 |
| 2.5 | closed | C++ | 5428/5557 | **5365**/5557 | 13147/13160 | 13160/13160 | 0/722 |
| 2.5 | open | Ada | 5428/5557 | 2762/5557 | 13147/13160 | 8211/13160 | 0/722 |
| 2.5 | open | Rust | 5428/5557 | 5277/5557 | 13147/13160 | 13160/13160 | 0/722 |
| 2.5 | open | C++ | 5428/5557 | 5277/5557 | 13147/13160 | 13160/13160 | 0/722 |
| 2.6 | closed | Ada | 5441/5570 | **2771**/5570 | 13198/13198 | 8231/13198 | 0/725 |
| 2.6 | closed | Rust | 5441/5570 | **5387**/5570 | 13198/13198 | 13198/13198 | 0/725 |
| 2.6 | closed | C++ | 5441/5570 | **5387**/5570 | 13198/13198 | 13198/13198 | 0/725 |
| 2.6 | open | Ada | 5441/5570 | 2763/5570 | 13198/13198 | 8231/13198 | 0/725 |
| 2.6 | open | Rust | 5441/5570 | 5299/5570 | 13198/13198 | 13198/13198 | 0/725 |
| 2.6 | open | C++ | 5441/5570 | 5299/5570 | 13198/13198 | 13198/13198 | 0/725 |

The bolded closed-schema full-declaration counts are exactly the Task 026
baseline figures. For UCI 2.5 the entire closed-schema report — inventory
counts, evidence lines, backend coverage counts, and all hypothetical
feature-impact counts — is **byte-for-byte identical** to the pre-change
baseline apart from the single added marker line:

```text
generation world: closed-schema
```

Open-world decreases land exactly where expected, on **fully renderable
declarations** only:

- Ada −8 (2.5) and −8 (2.6);
- Rust and C++ −88 (2.5) and −88 (2.6).

Kind, field-type, and field-occurrence metrics are unchanged in both worlds.
Field-type renderability deliberately measures whether a named `TypeRef` can be
*named*, not whether its target is fully renderable, so it is not reduced merely
because an abstract target cannot be represented open-world; transitive semantic
failure is carried by the full-declaration and message-closure metrics. Message
closures were already 0 in both releases because of the unrelated
`SourceCommandEXT` blocker, so they cannot drop further.

All 31 non-empty feature combinations complete under both worlds for all three
backends, with no panic, no recursion failure, and no combination reducing
capability.

### Performance

Coverage elapsed times, same machine, full authoritative runs:

| Run | Elapsed |
| --- | --- |
| pre-change baseline, UCI 2.5 | 253 s |
| UCI 2.5 closed-schema | 260 s |
| UCI 2.5 open-extensions | 264 s |
| UCI 2.6 closed-schema | 264 s |
| UCI 2.6 open-extensions | 265 s |

No order-of-magnitude regression. The Task 026 fix is preserved: world checks
are O(1), the declaration map, abstract topology map, and closed-world
fully-elided-target set are all built once in `CoverageAnalysis::new`, and no
per-field schema scan or per-field abstract projection occurs during
measurement.

Normalization is unaffected by the world, as required: UCI 2.5 normalizes to
5,557 types / 722 messages and UCI 2.6 to 5,570 types / 725 messages, unchanged.

## Task 029 — additive schema overlays

Task 028 established that the practical route to a real UCI extension point is
ADR-0004 option 4: supply the private derived-type schema in the generation
schema set and assert `--world closed-schema`. It validated that only through a
fixture whose root `xs:include`d the private document, which is unusable against
a *pinned* authoritative root — it would require editing the root or fabricating
a wrapper document of new `xs:include` directives.

Task 029 makes the composition an explicit **input** instead of a schema edit.

### Frontend and CLI semantics

One frontend API performs all schema-set loading:

```text
load_schema_set_with_overlays(root: &Path, overlays: &[PathBuf]) -> Result<SchemaIr, FrontendError>
```

`load_schema_set(root)` is now exactly its empty-overlay case, so no existing
caller changed behaviour. The CLI surfaces it as a repeatable `--overlay PATH`
on `validate`, `coverage`, and `generate`; zero overlays is the previous
behaviour.

```text
ams-gra-codegen-oms generate \
  --schema public.xsd \
  --overlay private-a.xsd \
  --overlay private-b.xsd \
  --language rust --output generated --world closed-schema
```

Unlike the singular options, `--overlay` is deliberately **not** `set_once`: it
accumulates into a `Vec<PathBuf>`, and repeating the same path is accepted.

### Deterministic order

The primary root and its complete `xs:include`/`xs:import` closure load first,
which preserves primary declaration order, message order, namespace
presentation, root schema version, and every pre-existing diagnostic. Overlay
closures follow, in caller-provided order:

```text
primary root closure -> overlay A closure -> overlay B closure -> one SchemaIr
```

Overlays are **never sorted**. Sorting by absolute filesystem path would make
declaration order — and therefore closed-sum variant order — depend on where
files happen to live on a given machine. Command-line order is explicit,
portable input, so it is the ordering source. Reversing `--overlay` order
deterministically reverses overlay declaration and variant order; that is
documented input ordering, not instability, and is regression-tested in all
three backends.

### Duplicate behaviour

Overlays are additive only. A qualified name declared twice fails existing
duplicate validation, with no precedence, no shadowing, and no "last overlay
wins" — whether the collision is root-versus-overlay or overlay-versus-overlay.
Messages with duplicate qualified names likewise still fail.

Distinct from that, *canonical file* dedupe means one physical document is
parsed exactly once no matter how many input routes reach it: listed twice,
spelled differently, or already reachable from the root through `xs:include`.
One ordinary parse per unique canonical document; no per-backend reload and no
new coverage-hot-path scan.

### Namespace boundary

Every **top-level** overlay must declare the same `targetNamespace` as the
primary root, checked through a dedicated `NamespaceExpectation::Overlay` so the
diagnostic names the overlay rather than blaming an `xs:include` the caller
never wrote:

```text
schema overlay target namespace mismatch for <file>: expected <primary>, found <overlay>
```

Once an overlay root is accepted, its own `xs:include`/`xs:import` dependencies
follow the ordinary loader rules — they are not reinterpreted as overlays, and
remote `schemaLocation` remains rejected. An accepted overlay may therefore
still import other namespaces exactly as the root could, but **the language
backends remain single-namespace**: Task 029 adds no cross-namespace generation.

### Private overlay fixture

`crates/xsd-frontend/tests/fixtures/schema-overlay/` holds a deliberately
generic fixture — no UCI identifier, no `EXT` suffix, so no production logic can
key on naming:

- `public.xsd` — namespace `urn:overlay`, abstract `ExtensionBase`, `Container`
  with `Extensions : ExtensionBase` at `0..unbounded`. It contains **no**
  `xs:include` of any private document, which is the point: the private schema
  can only enter through explicit overlay composition;
- `private-a.xsd` / `private-b.xsd` — `PrivateA` / `PrivateB` extending
  `ExtensionBase`. `private-a.xsd` binds the shared namespace to the prefix
  `private` rather than the root's `pub`, proving lexical prefix has no semantic
  effect and does not disturb namespace presentation metadata.

Loaded alone, `public.xsd` yields `ExtensionBase` with zero known concrete
descendants and no `PrivateA`. Composed with the overlay it yields one
namespace and `[ExtensionBase, Container, PrivateA]` — primary declarations
first — with `PrivateA`'s immediate base resolving to `ExtensionBase` across the
root/overlay boundary.

### Task 024 composition

Closed-schema generation of the composed fixture produces the ordinary Task 024
closed sum plus the repeated `0..*` field, with **no new backend code**:

| Backend | Closed sum | Repeated field |
| --- | --- | --- |
| Rust | `pub enum ExtensionBase { PrivateA(PrivateA) }` | `pub extensions: UnboundedVec<ExtensionBase, 0>` |
| C++ | `struct ExtensionBase { std::variant<PrivateA> value; }` | `UnboundedVector<ExtensionBase, 0> extensions` |
| Ada | `type ExtensionBase (Kind : ExtensionBase_Kind := PrivateA_Kind)` | `Extensions : Container_Extensions_Sequence` |

Each backend's generated output is compiler-probed while constructing a
`PrivateA` value inside the repeated collection: Rust with `rustc`, C++ with
`c++ -std=c++17 -Wall -Wextra -pedantic-errors`, Ada with
`gnatmake -gnatwa -gnata`.

### Task 028 world interaction

Supplying an overlay **never** selects a world and never relaxes one. The same
composed fixture that closes cleanly under `closed-schema` still fails closed
under `open-extensions`, naming `ExtensionBase` and the policy: knowing one
private descendant does not prove no *other* external descendant exists.

### Real UCI `SourceCommandEXT` evidence

A **temporary, uncommitted** probe overlay was written for the real UCI
namespace `https://www.vdl.afrl.af.mil/programs/oam`, declaring one concrete
`Task029PrivateSourceCommand` extending `uci:SourceCommandEXT`. Because
`SourceCommandEXT` is itself an empty abstract type, the extension is empty too.

This is **synthetic probe data only**. It is not a real UCI private
SourceCommand definition and makes no claim to represent one. No authoritative
UCI schema file is committed to this repository.

Validation counts — exactly one added declaration, no message change:

| Release | Types (no overlay) | Types (overlay) | Messages |
| --- | ---: | ---: | ---: |
| 2.5 | 5,557 | **5,558** | 722 (unchanged) |
| 2.6 | 5,570 | **5,571** | 725 (unchanged) |

### Exact next closed-world blocker after the overlay

All six `closed-schema` probes move **past** the long-standing
`SourceCommandEXT has no concrete structural descendants` blocker. The blocker
that replaces it is unrelated to extension points and to Task 029, so no feature
expansion was attempted:

| Release | Backend | Exit | First blocker with overlay | Class |
| --- | --- | ---: | --- | --- |
| 2.5 | Ada | 1 | `unsupported Ada IR construct: type reference Primitive(Duration)` | unsupported primitive |
| 2.5 | Rust | 1 | `unsupported Rust IR construct: type reference Primitive(Duration)` | unsupported primitive |
| 2.5 | C++ | 1 | `unsupported C++ IR construct: type reference Primitive(Duration)` | unsupported primitive |
| 2.6 | Ada | 1 | `unsupported Ada IR construct: constraints on AA_CodeType` | unsupported constraint |
| 2.6 | Rust | 1 | `unsupported Rust IR construct: constraints on AA_CodeType` | unsupported constraint |
| 2.6 | C++ | 1 | `unsupported C++ IR construct: constraints on AA_CodeType` | unsupported constraint |

The two releases surface different next blockers because each backend reports
the first unsupported construct in deterministic declaration order, and the two
roots differ in declaration content. Neither blocker is implemented here.

Open-world behaviour is unchanged by the overlay: all six `open-extensions`
probes still stop at the earlier abstract value `CapabilityCommandBaseType`,
with and without the overlay, confirming overlay support does not weaken
open-world policy.

| Release | Backends | With overlay | Without overlay |
| --- | --- | --- | --- |
| 2.5 | Ada/Rust/C++ | `CapabilityCommandBaseType` | `CapabilityCommandBaseType` |
| 2.6 | Ada/Rust/C++ | `CapabilityCommandBaseType` | `CapabilityCommandBaseType` |

### Coverage delta

Root-only coverage is unchanged: all twelve no-overlay cells reproduce the
Task 028 authoritative coverage matrix exactly, so the overlay plumbing does not
perturb the ordinary case.

With the one-declaration overlay, the inventory changes are exactly what one
added concrete empty extension implies:

| Metric | 2.5 no overlay | 2.5 overlay | 2.6 no overlay | 2.6 overlay |
| --- | ---: | ---: | ---: | ---: |
| `declarations.total` | 5,557 | 5,558 | 5,570 | 5,571 |
| `declarations.concrete` | 5,487 | 5,488 | 5,500 | 5,501 |
| `declarations.with_base_type` | 2,971 | 2,972 | 2,970 | 2,971 |
| `kind.Record` | 4,192 | 4,193 | 4,212 | 4,213 |
| `abstract_usage.base` | 57 | 58 | 57 | 58 |
| `base.structural_named` | 2,026 | 2,027 | 2,036 | 2,037 |
| `base.references_backward` | 1,151 | 1,152 | 1,160 | 1,161 |
| `inheritance.abstract_bases` | 1,270 | 1,271 | 1,273 | 1,274 |
| `inheritance.distinct_bases` | 271 | 272 | 272 | 273 |
| `inheritance.empty_local_extensions` | 406 | 407 | 405 | 406 |
| `inheritance.record-with-record-base` | 2,025 | 2,026 | 2,035 | 2,036 |
| `abstract_value.acyclic_targets` | 29 | 30 | 29 | 30 |
| `abstract_value.zero_descendant_targets` | **13** | **12** | **13** | **12** |

Field types, field occurrences, and message closures are unchanged (2.5
`13147/13160` and `8211`/`13160`, `0/722`; 2.6 `13198/13198` and
`8231`/`13198`, `0/725`), because the probe overlay adds an empty extension with
no new field and no new message. These deltas are identical under both worlds:
the inventory is an objective measurement of the schema set, not a policy
measurement.

Backend full-declaration coverage with the overlay:

| Release | World | Backend | Kinds | Full declarations |
| --- | --- | --- | --- | --- |
| 2.5 | closed | Ada | 5429/5558 | 2772/5558 (was 2770/5557) |
| 2.5 | closed | Rust/C++ | 5429/5558 | 5369/5558 (was 5365/5557) |
| 2.5 | open | Ada | 5429/5558 | 2764/5558 (was 2762/5557) |
| 2.5 | open | Rust/C++ | 5429/5558 | 5279/5558 (was 5277/5557) |
| 2.6 | closed | Ada | 5442/5571 | 2773/5571 (was 2771/5570) |
| 2.6 | closed | Rust/C++ | 5442/5571 | 5391/5571 (was 5387/5570) |
| 2.6 | open | Ada | 5442/5571 | 2765/5571 (was 2763/5570) |
| 2.6 | open | Rust/C++ | 5442/5571 | 5301/5571 (was 5299/5570) |

Downstream coverage improvement was measured, not inferred.

### Zero-descendant topology change

The most important proof that overlay declarations take the *ordinary* path
rather than a special registry path:

```text
abstract_value.zero_descendant_targets: 13 -> 12

abstract SourceCommandEXT: record-field
  -> abstract SourceCommandEXT: base, record-field

abstract value SourceCommandEXT: no concrete descendants
  -> abstract value SourceCommandEXT: acyclic (1 concrete descendants)
```

measured in **both** releases. The overlay's declaration participates in normal
Task 024 topology analysis; nothing special-cases it.

This also happens under `open-extensions` coverage, and that is correct. The
topology inventory objectively measures the supplied schema set, and
`SourceCommandEXT` genuinely does now have a known descendant there. Generation
semantics remain open-world fail-closed regardless. Objective topology counts
were not altered to make policy metrics look simpler.

### Performance

Overlay loading is one ordinary parse per unique canonical document; no repeated
reload per backend and no additional scan in coverage hot paths. Measured
coverage runtimes are indistinguishable between no-overlay and overlay runs:

| Case | Seconds |
| --- | ---: |
| 2.5 closed, no overlay | 260.9 |
| 2.5 closed, overlay | 254.7 |
| 2.6 closed, no overlay | 254.6 |
| 2.6 closed, overlay | 266.6 |
| 2.5 open, no overlay | 259.3 |
| 2.5 open, overlay | 254.2 |
| 2.6 open, no overlay | 255.1 |
| 2.6 open, overlay | 263.8 |

Variation is within ordinary noise on a loaded host; there is no
order-of-magnitude regression.

### What Task 029 does not add

No runtime extension registry, runtime polymorphism, `xsi:type` dispatch, JSON
or XML codec, or CAL runtime work. No override/patch semantics. No
cross-namespace backend generation. No `SourceCommandEXT` name special-case and
no `EXT`-suffix heuristic. No `GenerationWorld` change and no world default. No
`SchemaIr` shape change: after loading it remains a normalized semantic
declaration set, with composition provenance available only through each
declaration's existing `SourceRef`. The newly exposed post-overlay blockers are
recorded, not implemented.

## Task 033 — named constrained floating ranges

Task 022 recorded that *all* non-default floating `ConstraintSet` values were
fail-closed. That remains the truthful description of the system at Task 022;
Task 033 narrows it. A **named** `Float32`/`Float64` declaration is now
backend-renderable when its effective `ConstraintSet` consists solely of numeric
range facets.

### Supported subset

`minInclusive`, `maxInclusive`, `minExclusive`, `maxExclusive`, in any
combination of lower-only, upper-only, two-sided, inclusive, exclusive, or
mixed inclusive/exclusive. Nothing else. A declaration carrying a lexical facet
(`pattern`, `whiteSpace`), a length facet (`length`, `minLength`, `maxLength`),
an ambiguous same-side pair (both `minInclusive` and `minExclusive`, or both
`maxInclusive` and `maxExclusive`), a non-finite bound, or a bound whose
`NumericValue` domain does not match the declared width still fails closed with
a structured error. No constraint is ever silently discarded, and a numeric
bound sitting beside an unsupported facet is **not** partially enforced.

The classification lives in one place, `codegen-core/src/floating.rs`
(`floating_domain`), and is consumed by all three backends *and* by
`CoverageAnalysis`. A single shared answer is what keeps the backends and the
coverage model from drifting into disagreeing about which declarations render.

### Empty two-sided domains

A two-sided domain must actually be inhabited. `lower > upper` is rejected, and
so is `lower == upper` when **either** side is exclusive: an equal pair denotes
exactly one candidate value, and an exclusive side excludes it, leaving nothing.

| Domain | Verdict |
| --- | --- |
| `[1.0, 1.0]` | valid — exactly one value |
| `(1.0, 1.0]` | empty — rejected |
| `[1.0, 1.0)` | empty — rejected |
| `(1.0, 1.0)` | empty — rejected |

The equal/exclusive cases were **missed by the original Task 033 classifier**,
which checked only `lower > upper`, and were corrected during review. Generating
a type no value can inhabit would be a silent trap, so these fail closed.
Because `+0.0 == -0.0` under IEEE comparison, a signed-zero pair is an equal
pair and follows the same rule regardless of how the signs are spelled. The rule
applies identically to Float32 and Float64, and is tested by calling
`floating_domain()` directly rather than relying on `SchemaIr::validate()`.

### Width preservation

`Float32` stays binary32 and `Float64` stays binary64 everywhere. The helper
accepts only `NumericValue::Float32` bounds for a `Float32` declaration and only
`NumericValue::Float64` for a `Float64` one; a mismatched domain is rejected
rather than converted, because widening or narrowing a bound would move the
accepted set of the generated type. Nothing passes through `i128`, decimal, or
string, and no external numeric crate was added.

### Floating semantics

Bounds lower to ordinary `>=`/`>`/`<=`/`<` comparisons, so IEEE-754 behaviour
is inherited rather than reimplemented:

| Value | Lower-only finite bound | Upper-only finite bound | Two-sided finite range |
| --- | --- | --- | --- |
| `NaN` | rejected | rejected | rejected |
| `+Infinity` | accepted | rejected | rejected |
| `-Infinity` | rejected | accepted | rejected |

NaN is not special-cased into acceptance; it simply compares false against every
bound. Infinities are **not** rejected merely because a type is constrained.
`+0.0` and `-0.0` compare equal, so `minInclusive = 0` admits both signs and
`minExclusive = 0` rejects both. Unconstrained Task 022 floats continue to carry
NaN, infinities, and negative zero unchanged.

Bound literals are emitted from the stored binary value, formatted to round-trip
to the identical width; the original XSD lexical spelling is not preserved by the
frontend and is deliberately not reconstructed.

### Representations

**Rust.** A checked newtype preserving named identity:

```rust
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct AltitudeMeters(f64);

impl AltitudeMeters {
    pub const MIN: f64 = -6378237.0;
    pub const fn new(value: f64) -> Option<Self> { ... }
    pub const fn get(self) -> f64 { self.0 }
}
```

`Eq`, `Ord`, and `Hash` are deliberately not derived: floats have no total order
and no reflexive equality, so claiming those traits would be unsound.
Unconstrained named floats keep their Task 022 infallible `new(value) -> Self`
API byte for byte — only constrained ones return `Option`, because only they can
fail.

**C++.** An owning value wrapper with no public unchecked construction:

```cpp
class BurnRate {
public:
    static constexpr double min_value = 0.0;
    static std::optional<BurnRate> create(double value) noexcept { ... }
    double value() const noexcept { return value_; }
private:
    explicit BurnRate(double value) noexcept : value_(value) {}
    double value_;
};
```

Nothing beyond C++17 and the already-included `<optional>` is required. The Task
022 binary32/binary64 host `static_assert`s are unchanged and still emitted for
constrained-float-only schemas.

**Ada.** A **private** type whose completion is a derived IEEE type carrying a
`Dynamic_Predicate`, with `Create` and `Value` as the only public operations:

```ada
pragma Assertion_Policy (Dynamic_Predicate => Check);
...
   type AltitudeMeters is private;

   function Create (Value : Interfaces.IEEE_Float_64) return AltitudeMeters;

   function Value (Item : AltitudeMeters) return Interfaces.IEEE_Float_64;

private

   type AltitudeMeters is new Interfaces.IEEE_Float_64
     with Dynamic_Predicate =>
       AltitudeMeters >= -6378237.0;

   function Create (Value : Interfaces.IEEE_Float_64) return AltitudeMeters
   is (AltitudeMeters (Value));

   function Value (Item : AltitudeMeters) return Interfaces.IEEE_Float_64
   is (Interfaces.IEEE_Float_64 (Item));
```

A finite Ada `range` subtype is deliberately **not** used: it would exclude
`+Infinity` from a lower-only XSD constraint that actually admits it, silently
narrowing the schema's domain.

### Why constrained Ada floats are private

**This defect was found during Task 033 review, and the representation was
corrected in response.** Task 033 first shipped the predicate on a *publicly*
derived numeric type. That made `Create` checked but left the representation
itself open, because a public numeric derivation also publishes a conversion and
a full set of inherited operators.

Two ordinary client paths therefore still manufactured invalid values, with no
diagnostic at all, when the client was compiled without `-gnata` — reproduced
against GNAT 14.2 before the fix:

```ada
Bad : BurnRate := BurnRate (0.0);          --  0.0 in a "> 0.0" type

A : FloatUnitInterval := Create (0.75);
B : FloatUnitInterval := Create (0.75);
C : FloatUnitInterval := A + B;            --  1.5 in a "[0.0, 1.0]" type
```

The lesson is that **a client's assertion policy cannot be trusted to preserve a
generated type's invariant**, and neither can the client's restraint: an
invariant that survives only when callers avoid legal operations is not an
invariant. Rust and C++ already prevented this class of bypass with private
storage and private constructors; Ada now provides the same guarantee
structurally.

Hiding the derivation removes both paths from the public surface. Both probes
above are now **compile errors** — "invalid conversion" and "no applicable
operator `+` for private type" respectively — and both are asserted as such, by
their intended diagnostic rather than by mere failure, in the backend and CLI
regressions. A runtime exception is deliberately not accepted as sufficient
here: the unchecked surface should not exist publicly at all.

The public contract is therefore:

* `Create` is the single checked construction boundary.
* `Value` is read-only extraction of the underlying IEEE scalar.
* No conversion, inherited arithmetic, writable field, unchecked construction,
  or representation clause is publicly available.

No public arithmetic over constrained wrappers is provided. If it is ever
wanted, it belongs in a deliberate design as explicit checked operations.

### How the check survives a client without `-gnata`

Predicate enforcement follows the `Assertion_Policy` in force **where the
conversion is written**, not where the type is declared. Because `Create`'s
expression-function completion is written inside the generated spec, under that
spec's own `pragma Assertion_Policy (Dynamic_Predicate => Check)`, the
conversion sits on the generated side of that boundary and is checked regardless
of client flags. The pragma is a configuration pragma on the generated unit
only; no repository-wide compiler flag was changed.

The regression probes compile *without* `-gnata` precisely so an unenforced
predicate would fail rather than quietly pass, and this was confirmed by
temporarily removing the pragma and observing the runtime test fail. They drive
every accepted and rejected case through `Create` and round-trip accepted finite
values back through `Value`.

The pragma, the private part, `Create`, and `Value` are emitted only when some
declaration in the unit actually carries a floating predicate, so unconstrained
Task 022 Ada output remains `type Name is new Interfaces.IEEE_Float_32;` with no
predicate, no pragma, no private part, and no operations — byte-for-byte
unchanged.

### Named restriction chains

Task 015 already resolves inherited constraints, so `declaration.constraints` is
already the *effective* domain. Backends consume it directly and never re-walk
raw XSD restriction chains. A derived declaration therefore enforces its final
narrowed domain: given `BaseFloat` with `minInclusive = 0` and `DerivedFloat`
restricting it with `maxInclusive = 10`, the generated `DerivedFloat` enforces
both bounds. In authoritative UCI this is what makes `DecibelNonNegativeType`,
`GeomagneticApIndexType`, and `GeomagneticKpIndexType` renderable.

Named identity is preserved without modelling XSD simple-type derivation as host
inheritance: no C++ class inheritance and no Rust wrapper nesting was introduced
merely because `base_type` is named. The semantic relation stays in
IR/provenance/dependency ordering, as before.

### Boundaries that did not move

* **Field-local floating constraints.** A field whose `TypeRef` is a direct
  `Primitive(Float32/Float64)` and whose *field* `ConstraintSet` carries numeric
  bounds remains fail-closed in all three backends. The field stores the
  primitive directly and no field-specific checked wrapper exists, so accepting
  it would mean silently dropping the constraint. Coverage continues to attribute
  it to `ConstrainedSimpleTypes`.
* **Floating lexical constraints.** Still unsupported; a regex is never
  reinterpreted as numeric range semantics. No authoritative floating lexical
  constraint exists in either pinned release.
* **Length facets on floats.** Meaningless, and still rejected.

### Revised meaning of `ConstrainedSimpleTypes`

After Task 033 this hypothetical family no longer means "all constrained
floating declarations". It now covers the remaining unsupported simple
constraints: constrained String, constrained Binary, floating lexical/length
facets, ambiguous or wrong-width floating bounds, field-local floating
constraints, the excluded integral exclusive/lexical shapes, and future
constrained-simple cases. Supported named floating ranges are **baseline**, so
no feature family adds them.

### Authoritative floating inventory

Recomputed from both pinned roots during Task 033; identical in UCI 2.5 and 2.6.

| Metric | Float32 | Float64 |
| --- | ---: | ---: |
| Named declarations | 4 | 40 |
| Direct (`base=xs:float`/`xs:double`) | 4 | 25 |
| Named restriction chains | 0 | 15 |
| Unconstrained (default) | 0 | 26 |
| Range-facet-only constrained | 4 | 14 |
| Lexical-constrained | 0 | 0 |
| Length-constrained | 0 | 0 |

Direct range-facet counts, also identical across releases:

| Facet | Float32 | Float64 |
| --- | ---: | ---: |
| `minInclusive` | 4 | 10 |
| `maxInclusive` | 3 | 7 |
| `minExclusive` | 0 | 1 |
| `maxExclusive` | 0 | 0 |

Every constrained floating declaration in both authoritative releases is
therefore inside the Task 033 supported subset: there are zero lexical and zero
length facets on floating types. The named Float32 declarations are
`IFF_BarometricPressureType`, `SpoilFactorType`, `UnitBallFloatType`, and
`UnitIntervalFloatType`. The directly constrained Float64 declarations include
`AltitudeBarometricType`, `AltitudeType`, the five angle-range declarations,
`DoubleNonNegativeType`, `DoublePositiveType` (the single `minExclusive`),
`UnitBallDoubleType`, and `UnitIntervalDoubleType`.

### Measured coverage delta

Fresh post-change `coverage --world closed-schema` runs against both pinned
roots. Normalized frontend counts are unchanged, as they must be: Task 033
touched no frontend code.

| Release | Types | Messages |
| --- | ---: | ---: |
| 2.5 | 5,557 (unchanged) | 722 (unchanged) |
| 2.6 | 5,570 (unchanged) | 725 (unchanged) |

UCI 2.5, closed world:

| Backend | Kinds | Full declarations | Field types | Field occurrences | Message closures |
| --- | --- | --- | --- | --- | --- |
| Ada | 5428/5557 (unchanged) | **2800/5557** (was 2770) | 13147/13160 (unchanged) | 8211/13160 (unchanged) | 0/722 (unchanged) |
| Rust | 5428/5557 (unchanged) | **5395/5557** (was 5365) | 13147/13160 (unchanged) | 13160/13160 (unchanged) | 0/722 (unchanged) |
| C++ | 5428/5557 (unchanged) | **5395/5557** (was 5365) | 13147/13160 (unchanged) | 13160/13160 (unchanged) | 0/722 (unchanged) |

The movement is exactly where the design predicts it: **+30 fully renderable
declarations** per backend, and nothing else. Declaration *kinds* do not move
because a constrained float was always a recognized `Primitive` kind; field type
references do not move because a *reference* to a named float was always
renderable regardless of the target's constraints; field occurrences are
untouched because Task 033 changed no cardinality rule. Message closures remain
`0/722` in both releases, because every UCI message closure still reaches some
unrelated unsupported construct — principally the temporal primitives.

The delta is identical for all three backends, which is the expected consequence
of a single shared classifier: the new capability is not language-specific.

UCI 2.6, closed world:

| Backend | Kinds | Full declarations | Field types | Field occurrences | Message closures |
| --- | --- | --- | --- | --- | --- |
| Ada | 5441/5570 (unchanged) | **2801/5570** (was 2771) | 13198/13198 (unchanged) | 8231/13198 (unchanged) | 0/725 (unchanged) |
| Rust | 5441/5570 (unchanged) | **5417/5570** (was 5387) | 13198/13198 (unchanged) | 13198/13198 (unchanged) | 0/725 (unchanged) |
| C++ | 5441/5570 (unchanged) | **5417/5570** (was 5387) | 13198/13198 (unchanged) | 13198/13198 (unchanged) | 0/725 (unchanged) |

The +30 delta is identical in both releases, which matches the inventory: the
same 4 Float32 and 14 Float64 range-facet-only declarations exist in each, and
the remainder of the 30 comes from declarations that were blocked *only* by a
constrained floating dependency.

### Selected PositionReport readiness delta

Measured after Task 033 against authoritative UCI 2.5 with
`crates/service-contract/tests/fixtures/upstream-minimal.yaml`, closed world.

> **Superseded.** These figures predate generated-name preflight. The current
> authoritative readiness is Rust 51/60 and Ada 32/60; see "Selected
> PositionReport readiness after the correction" below. The Task 033 delta this
> table records is still accurate *as a Task 033 result*.

| Backend | Task 031 | Task 033 | First blocker now |
| --- | --- | --- | --- |
| Rust | 47/60, `AltitudeType` | **52/60** | `{…}DateTimeType` |
| C++ | 47/60, `AltitudeType` | **52/60** | `{…}DateTimeType` |
| Ada | 32/60, `Acceleration3D_Type` | **37/60** | `{…}Acceleration3D_Type` (unchanged) |

`AltitudeType` is no longer a blocker for any backend, and no *other* constrained
floating type took its place — which is the check that the implementation is
generic rather than shaped around one declaration. The new Rust/C++ first
blocker, `DateTimeType`, belongs to a different feature family (temporal
primitives) and is deliberately **not** implemented here.

Ada's first blocker is unchanged, exactly as expected: `Acceleration3D_Type`
fails for an unrelated reason — it carries an ordinary optional named field
(`Timestamp : DateTimeType`), and Ada's current occurrence model supports
optional `String` only, aside from Task 026 absent-only elision. Task 033 still
raised Ada's renderable selected count from 32 to 37 behind that blocker. Ada
general `Optional<T>` support remains out of scope.

The remaining Rust/C++ unsupported selected types are all temporal or
constrained-String: `DateTimeType`, `UCI_SchemaVersionStringType`,
`UniversallyUniqueIdentifierType`, `VisibleString256Type`,
`SecurityInformationType`, `NATO_SpecialWordsType`,
`WhitespaceVisibleString1024Type`, and `WhitespaceVisibleString4096Type`. None
is a floating type.

### Cost

No topology-construction regression. Authoritative elapsed times are in line
with Task 031's ~255 s, essentially all of which is `CoverageAnalysis::new` over
the whole schema:

| Run | Seconds |
| --- | ---: |
| 2.5 readiness, Rust, closed | 258 |
| 2.5 readiness, Ada, closed | 260 |
| 2.5 coverage, closed | 262 |
| 2.6 coverage, closed | 255 |

These were measured with two probes running concurrently on a loaded host, so
they are upper bounds rather than best-case figures.

### What Task 033 does not add

No Ada general `Optional<T>` and no Ada optional named values. No `DateTime`,
`Time`, `Duration`, or `Decimal` support, and no temporal lexical parsing. No
constrained String or constrained Binary. No field-local floating constraints
and no anonymous constrained simple types. No floating pattern/lexical facets
and no floating `whiteSpace` semantics. No arbitrary-precision decimal bounds.
No CAL wrappers, service wrappers, JSON codecs, or runtime code. No profile
validation, Capability inference, or UCI version normalization.

No `AltitudeType` or `Acceleration3D_Type` name special-case exists anywhere (see
also the corrective-cleanup section at the end of this document):
both are ordinary consequences of the generic rules above. No frontend semantic
change and no `SchemaIr`/`ConstraintSet`/`NumericValue` shape change was made,
and no `ServicePlan`, readiness, or selected-generation special-case was added —
Tasks 031 and 032 inherit the new capability through the existing shared
coverage snapshot and the ordinary backends.

## Corrective cleanup: generated names and the abstract-descendant boundary

This section describes a retrospective correction made after Task 033. Where it
describes a defect, the **historical behaviour** and the **current corrected
behaviour** are distinguished explicitly; earlier sections of this document
remain an accurate record of what each task did at the time.

### Abstract descendant projection is fail-closed

**Historical behaviour.** `project_abstract_value` decided descendancy from a
successful structural projection, and skipped any candidate whose projection
failed. A concrete descendant that existed but could not be represented — for
example because of an inherited member-name collision — was therefore silently
dropped. Two different facts became indistinguishable:

```text
a legal concrete payload exists but cannot be represented
no legal payload exists
```

If every descendant failed, the target was reported as
`NoConcreteDescendants`, which is the precondition for Task 026's absent-only
optional elision. An optional field of such a target could then be elided
entirely.

**Current corrected behaviour.** Descendancy is decided from the declared
`base_type` chain, which is available whether or not the candidate is
representable. A descendant that cannot be projected now fails closed with

```text
AbstractValueProjectionError::UnrepresentableConcreteDescendant
```

carrying the abstract target, the concrete descendant, and the underlying
`StructuralProjectionError`. It is never converted to `NoConcreteDescendants`,
so Task 026 elision cannot be reached for a target that does have a legal
payload, and a closed sum can never be emitted with an alternative missing.

A genuinely descendant-free abstract target still reports
`NoConcreteDescendants`; the correction narrows that classification rather than
removing it.

Authoritative UCI 2.5 and 2.6 abstract-value topology and coverage counts are
unchanged, because neither schema contains an unrepresentable concrete
descendant.

### Generated host-language names are part of backend representability

**Historical behaviour.** IR identifiers are transformed per backend — Rust and
C++ upper-camel declarations and snake_case members, Ada verbatim
case-insensitive identifiers. Only Choice alternatives were collision-checked,
and only inside each renderer; top-level declarations, effective Record
members, enumeration variants, and Ada's user-derived helper types were not
checked at all. Coverage modelled none of it. Two consequences followed:

* two distinct XSD names could normalize to one generated identifier
  (`foo_bar` and `fooBar` both become `FooBar`);
* a legal XSD name could land on a host-language reserved word.

Either produced source that does not compile, while capability analysis could
still report the declaration renderable. This was not hypothetical: the
repository's own `backend-unbounded-cardinality.xsd` fixture declared a type
named `Record`, and GNAT rejects the Ada spec generated from it with
`reserved word "record" cannot be used as identifier`.

**Current corrected behaviour.** `codegen-core::backend_names` owns one shared
model of generated-name validity, consumed by **both** backend generation
validation and capability/readiness analysis, so the two cannot disagree. Per
backend it rejects, in schema order:

| Region | Rust / C++ | Ada |
|---|---|---|
| top-level declarations | upper-camel collisions, reserved words | case-insensitive collisions, reserved words |
| Record members (effective, post-inheritance) | snake_case collisions, reserved words | case-insensitive collisions, reserved words |
| enumeration variants | upper-camel collisions, reserved words | case-insensitive collisions, reserved words |
| Choice variants | upper-camel collisions | case-insensitive component collisions |
| generated helper types | not emitted | `{Owner}_{Member}_Array`/`_Sequence`/`_Item`/`_Vectors`, registered in the **flat package** scope beside declared types |

Ada's helper names are user-controlled and share the flat package scope with
every declared type, so they are checked there rather than in the owning
declaration's member scope.

The rules are asymmetric by design and the asymmetry is asserted, not smoothed
over: a case-only difference collides in Ada but not in Rust or C++, and an
upper-camel convergence collides in Rust and C++ but not in Ada.

This module **rejects**; it never mangles, escapes, suffixes, or renames.
Inventing a disambiguation scheme would silently change the generated API
surface. Existing accepted UCI identifiers keep their exact generated spelling.

> **Superseded claim.** This section originally stated that authoritative UCI
> coverage was unchanged. That was written before the preflight was integrated
> into coverage, and it is **false**: the first preflight implementation went
> on to expose real compiler-invalid identifiers in authoritative UCI, which
> moved the Rust and C++ declaration counts. The final corrected figures are
> recorded in "Authoritative coverage correction" and in "Final corrective:
> projection-scoped readiness and remaining Ada generated names" below, and
> those are the authoritative numbers.

> Language-specific naming policy lives in shared codegen infrastructure. It is
> never stored in Schema IR, and no generated name is written back into IR.

### Single-namespace generation is a readiness-visible boundary

**Historical behaviour.** The frontend represents imported and multiple
namespaces; the three language backends deliberately remain single-namespace
and reject multi-namespace input globally. Service readiness, however, operated
only at declaration and member capability level. A selected service closure
spanning two namespaces could therefore be reported READY even though
generation of that same projected schema fails.

**Current corrected behaviour.** `codegen-core::backend_preflight` models the
global preconditions a backend imposes on a whole schema — the single generable
namespace, plus generated-name safety — and both backend `validate_schema` and
`analyze_service_readiness` call it.

Readiness runs the preflight on the **projected selected schema**, exactly the
input selected-service generation hands to the backend. That is what makes the
scope right in both directions:

* a selected closure that genuinely spans namespaces is NOT READY, with a typed
  `BackendPreflightError::MultipleNamespaces` blocker naming every namespace;
* a selected closure that narrows to one namespace stays READY even when the
  full schema set contains unrelated declarations in other namespaces.

A violated precondition is reported as a **backend capability** limit, not as
malformed Schema IR: multi-namespace input is valid IR that the backends simply
do not generate yet. Nothing here implements multi-namespace generation, and the
backends' existing rejection is not weakened.

An empty projected schema — what a contract with zero OMS Message exchanges
produces — is vacuously generable: there is nothing to emit, so there is no
namespace to require.

### Corrective review: coverage now honours generated-name safety

The shared preflight above was consulted by backend generation and by service
readiness, but **not** by `CoverageAnalysis`. Coverage could therefore report a
declaration fully renderable while generation rejected the same schema, which
is exactly the coverage/generation disagreement the cleanup set out to remove.

`CoverageAnalysis` now consumes the same shared model, with the distinction
that makes the result honest rather than merely conservative:

* a **declaration-attributable** name failure (a reserved generated
  identifier, a collision, an unusable identifier) marks exactly the
  declarations responsible as not renderable, leaving the rest of the schema's
  metrics intact;
* a **namespace-unit** failure (multi-namespace input, or a package/namespace
  identifier that is illegal or reserved) zeroes full-declaration and
  message-closure capability, because the backend then emits nothing at all
  and no declaration is individually at fault.

Narrower per-kind, per-type-reference, and per-occurrence figures are retained
in both cases: those questions remain answerable, and feature-impact analysis
reads them.

No naming or namespace policy is reimplemented in coverage; it consumes
`backend_preflight()` and the attributed `unsafe_named_declarations()`, both
of which run the same registration logic that backend generation runs.

#### Authoritative coverage correction

Integrating the check corrected real overclaiming in the closed-coverage
numbers. Authoritative UCI contains declarations whose generated member
identifiers are reserved words, which the backends cannot emit and coverage
previously counted as fully renderable:

| Release / backend | Before | After | Newly excluded |
| --- | --- | --- | --- |
| 2.5 Ada | `2800/5557` | `2731/5557` | 69 |
| 2.5 Rust | `5395/5557` | `5375/5557` | 20 |
| 2.5 C++ | `5395/5557` | `5378/5557` | 17 |
| 2.6 Ada | `2801/5570` | `2731/5570` | 70 |
| 2.6 Rust | `5417/5570` | `5397/5570` | 20 |
| 2.6 C++ | `5417/5570` | `5401/5570` | 16 |

These are the **final authoritative closed-world figures**, measured after the
generated-name attribution and identifier-syntax corrections below. The
earlier `2800/5395/5417` figures remain rejected as genuine overclaims.

Open-world declaration figures, measured the same way:

| Release | Ada | Rust | C++ |
| --- | --- | --- | --- |
| 2.5 | `2724/5557` | `5287/5557` | `5290/5557` |
| 2.6 | `2724/5570` | `5309/5570` | `5313/5570` |

The cause is attributed, not assumed. In UCI 2.5 the Ada set is 97
declarations with an unsafe generated name, of which 68 were previously
counted renderable; the other 29 were already excluded for unrelated
capability reasons. The complete Rust set is `ConfigurationParameterType`,
`DamagedObjectNonEntityType`, `ExpendableType`, `FileNameAndOutputType`,
`IdentityComparisonType`, `JPEG_WaveletTransformType`, `Link16_HazardType`,
`OrderOfBattleMDT`, `QueryInstanceOfType`, and `RelationshipEW_Type`. The
complete C++ set is `ApprovalResponseType`, `COMINT_ChangeDwellType`,
`ComponentControlsB_Type`, `EntityOrbitalCSO_MDT`,
`GatewayLink16_ConfigurationIdentityType`, `NotificationSourceType`, and
`SystemStatusMDT`.

Both compilers confirm these are genuine rather than modelling artefacts.
`AltitudeRangePairType` carries a member `Range`, and GNAT 14.2 reports
`reserved word "range" cannot be used as identifier`.
`ConfigurationParameterType` carries a member `Type`, and rustc reports
`expected identifier, found keyword type`.

Kind, field-type, field-occurrence, and message-closure figures are unchanged
in both releases and both worlds; frontend normalization is unchanged at 5,557
types / 722 messages and 5,570 types / 725 messages.

No identifier mangling, escaping, or renaming is introduced. An unsafe
generated name remains a rejection.

#### Final corrective: ownership, world-aware companions, identifier syntax

Peer review of the code found three further defects in this boundary. Fixing
them moved the closed figures above once more, and every moved declaration is
attributed individually.

**Structured generated-name ownership.** Coverage attribution recovered the
responsible declaration by splitting the human-readable diagnostic label
(`"Owner.Member helper"`, `"Owner companion"`). That matched nothing whenever
the label was not a declaration local name, so Ada repeated helpers, `_Kind`
companions, and enumeration/Choice literals attributed to no declaration:
generation rejected the schema while coverage still counted the *generating*
declaration renderable. Registration now carries an explicit private
`NameSource` (declaration, member, helper, companion, enum literal, generated
support, namespace URI), and attribution reads that structure. Diagnostics
still render the same strings; nothing parses them. Both sides of a collision
are recorded, so every declaration responsible for emitting one side is
excluded.

In UCI 2.5 this adds exactly one Ada declaration, `QueryPET`. `QueryType` is a
Choice *and* a concrete descendant of the abstract value target `QueryPET`, so
Ada emits `QueryType_Kind` as the Choice companion and also emits
`QueryType_Kind` as a literal of the `QueryPET_Kind` closed-sum enumeration.
GNAT 14.2 rejects the pair:

```text
p.ads:8:27: error: "QueryType_Kind" conflicts with declaration at line 2
```

`QueryType` was already excluded; `QueryPET`, which generated the conflicting
literal, was not. Both are now excluded, which is the rule: mark every
declaration responsible for emitting a side.

**World-aware `_Kind` companions.** `ada_kind_companion_owners` derived
companions from `abstract_value_targets`, but being an abstract value target
does not prove a wrapper is emitted. A Task 026 zero-descendant target used
only as supported absent-only optional storage emits no wrapper and no
`{Owner}_Kind`, yet the name was reserved anyway, falsely rejecting generable
schemas. A Task 024 wrapper also exists only under `ClosedSchemaSet`; under
`OpenExtensions` the abstract value fails closed before any wrapper exists, and
the semantic abstract-value blocker must stay authoritative rather than being
displaced by a manufactured name collision.

The predicate is now `project_abstract_value` succeeding under the requested
world -- exactly the condition producing `TypeEmission::AbstractValue` -- so
preflight and generation share one companion predicate. Ordinary Choice
lowering still registers `{Choice}_Kind` in both worlds, because the renderer
emits it unconditionally. `GenerationWorld` is threaded through the single
shared `backend_preflight()` / `validate_backend_names()` /
`unsafe_named_declarations()` entry points rather than re-decided per backend.

**Rust/C++ identifier-start syntax.** The shared word transformation only
guaranteed ASCII alphanumeric or `_` characters, not a legal identifier
*start*. `1Foo` survived upper-camel unchanged and passed preflight. Ada
already required an alphabetic first character; Rust and C++ now enforce the
equivalent ASCII rule (first character alphabetic or `_`, rest alphanumeric or
`_`) on the **generated** spelling, before reserved-word checking, and for
every emitted C++ namespace component.

This is what moves Rust and C++. The 10 newly excluded declarations in each are
identical and all are enumerations with a leading-digit variant:
`CapabilityTransmitPowerEnum` (`70W`), `CommCapabilityEnum` (`5G`),
`DeclassExceptionEnum` (`25X1`), `GCP_OffsetEnum` (`1METER`),
`IFF_AltitudeResolutionEnum` (`25_FEET`), `LateralAxisOffsetEnum` and
`LongitudinalAxisOffsetEnum` (`0_TO_2METERS`), `MaxPOR_Enum` (`1_IN_1`),
`TransponderAntennaOffsetLongitudinalEnum` (`0_TO_1METERS`), and
`UncertaintyEnum` (`1_SIGMA`). Ada already rejected all ten, which is why the
Ada count does not move for them. Both compilers confirm the rule:

```text
rustc: error: expected identifier, found `70W`
c++:   error: expected identifier before numeric constant
```

No declaration became renderable again; there are no removals from the unsafe
set in either release.

**Emitted-but-unregistered helper.** Re-reading the whole PR also found
`backend-ada`'s `write_unbounded_helper` splitting on the occurrence minimum:
`min == 0` emits `{stem}_Vectors`, but `min > 0` emits `{stem}_Required_Array`
plus `{stem}_Additional_Vectors`. Only the `min == 0` spelling was reserved, so
a user declaration could collide with a required-minimum helper undetected. The
suffix set now matches the renderer branch for branch.

#### Selected PositionReport readiness after the correction

Re-measured on authoritative UCI 2.5, closed world:

| Backend | Renderable selected | First blocker |
| --- | ---: | --- |
| Rust | 51/60 | `DateTimeType` |
| Ada | 32/60 | `Acceleration3D_Type` |

Rust moves 52 -> 51 for exactly one declaration, `DeclassExceptionEnum`, whose
`25X1` variant is the leading-digit rule above; the previous 52 was an
overclaim for that declaration. The first blocker is unchanged.

Ada measures 32/60, and that is **not** a change from this corrective: the same
32 was already produced at `538b0d1`. The `37/60` recorded in the Task 033
section above predates the preflight integration and is stale as a *current*
figure. The Ada unsupported-selected-type set is byte-identical before and
after this correction, and the first blocker remains `Acceleration3D_Type`.
Neither blocker is implemented here.

Service-check cost on full UCI 2.5 was ~15 s per language at this point, so the
world-aware, structurally attributed model reintroduces no per-declaration
whole-schema scan: preflight and the attributed unsafe set are still computed
once per language, under one world, outside every feature loop.

> **Superseded timing.** The later projection-scoped correction moved readiness
> onto the projected schema, reducing this to ~5.9 s per language. See "Final
> selected PositionReport readiness" below.

## Final corrective: projection-scoped readiness and remaining Ada generated names

### Selected readiness must not inherit full-schema naming failures

**Historical behaviour.** Integrating the generated-name preflight into
`CoverageAnalysis` was correct for *full-schema* coverage, but
`analyze_service_readiness()` then built its `CoverageAnalysis` and baseline
renderability snapshot from the **entire original schema** before consulting
the selected projection. Selected declarations were therefore judged with a
renderability vector that already carried failures contributed by
declarations selected generation removes.

Generated-name safety and the global backend preconditions are
**scope-dependent**. They are relationships between the declarations emitted
*together*, not properties of a declaration in isolation:

```text
selected:    foo_bar  -> FooBar
unselected:  fooBar   -> FooBar
```

Full-schema analysis marks both unsafe. The projection retains only
`foo_bar`, so generation emits one legal `FooBar` and succeeds — while
readiness still reported the selected declaration unsupported. That is a
readiness/generation disagreement, and the selected-service contract is
explicit that unselected declarations must not affect selected readiness
unless they are required generated-support dependencies.

Conditional generated support has the same shape. Rust emits `UnboundedVec`
only when some member is unbounded, so a *full* schema whose unselected part
has an unbounded member legitimately collides with a selected type named
`UnboundedVec` — yet a projection that drops that member emits no support
type and the spelling is free again.

**Corrected behaviour.** Projection now runs **first**, because it decides
which schema the capability questions may legitimately be asked about.
Whenever projection succeeds, the projected schema is authoritative: it is
exactly the schema `service-generate` hands to the backend.

```text
verify plan/schema binding
compute selected closure            (original schema: counts and order)
project selected generation schema
    integrity error            -> propagate
    abstract-value failure     -> analyze original, retain attribution
    success                    -> analyze the PROJECTION
                                  CoverageAnalysis::new(projection)
                                  baseline renderability
                                  backend_preflight(projection)
```

The invariant is now:

> A selected-service READY/NOT READY decision is made against the same schema
> subset and world that `service-generate` hands to the backend.

No naming policy is duplicated: the same `CoverageAnalysis`,
`validate_backend_names()`, and `backend_preflight()` machinery is applied to
the appropriate schema. There is no `readiness_name_rules.rs`, and no ad-hoc
exception for any particular collision.

Counts and ordering still come from the original schema, because the selected
closure's identity and order are facts about the *contract*. Only capability
is projection-scoped. Exactly **one** `CoverageAnalysis` is constructed per
readiness call — the projection when one exists, the original only when
projection produced none — so the dominant cost is not doubled.

### Ada reserves the fixed `Kind` discriminant

Ada lowers a Choice to a discriminated record whose discriminant is the fixed
identifier `Kind`:

```ada
type Selection (Kind : Selection_Kind := First_Kind) is record
   case Kind is
      when First_Kind => First : Some_Type;
   end case;
end record;
```

The discriminant occupies the same record declarative region as the
alternatives, but preflight validated alternatives only against each other.
An alternative named `Kind` therefore passed preflight and emitted invalid
Ada. Confirmed against GNAT 14.2:

```text
p3.ads:6:13: error: "Kind" conflicts with declaration at line 3
```

A new structured `NameSource::GeneratedMember { owner, generated }` occupies
the member region **before** the alternatives are registered. Because the
discriminant is generated *by* the owning Choice, a collision makes that
Choice unsafe, and `unsafe_named_declarations` attributes it there. This is an
Ada-only rule: Rust enum variants and C++ `std::variant` alternatives carry no
generated discriminant component, and a regression asserts they do not
inherit it.

**Abstract closed-sum wrappers.** These emit the same `Kind` discriminant, but
their variant components are `{Descendant}_Value`, derived from declaration
names rather than arbitrary source field names. No descendant spelling can
produce the bare identifier `Kind`, so no additional rule is registered; the
determination is recorded by
`closed_sum_wrapper_components_cannot_collide_with_their_kind_discriminant`
rather than left implicit.


### Ada models overloadable `Create` / `Value`

Every *constrained* named Float32/Float64 declaration emits two package-level
subprograms:

```ada
function Create (Value : Interfaces.IEEE_Float_64) return Some_Type;
function Value  (Item : Some_Type) return Interfaces.IEEE_Float_64;
```

These were not represented in the name model at all, so a collision with a
non-overloadable declaration went undetected. Reserving them as ordinary
unique type names would have been equally wrong: Ada subprograms *overload*,
and several constrained floats legitimately emit several `Create`/`Value`
functions. Both halves were verified against GNAT 14.2:

```text
-- accepted: profiles differ
function Create (Value : Interfaces.IEEE_Float_64) return Burn_Rate;
function Create (Value : Interfaces.IEEE_Float_64) return Altitude;

-- rejected
type Create is new Integer;
function Create (Value : Interfaces.IEEE_Float_64) return Burn_Rate;
   p2.ads:5:13: error: "Create" conflicts with declaration at line 3
```

The model therefore distinguishes **non-overloadable declaration names** from
**overloadable callable names**. A new
`NameSource::GeneratedCallable { owner, callable }` is *checked against* the
accumulated top-level names without being *inserted* into them — the same
asymmetry already used for Ada enumeration literals, and for the same reason.
Callable-vs-callable is silent; callable-vs-type fails. Both sides are
attributed: the float that generates the subprogram and the declaration
occupying the identifier.

The names are reserved only when the output exists. An unconstrained float
emits a plain derived type and no subprograms, and a schema with no
constrained float at all keeps `Create` and `Value` available to user
declarations.

This deliberately stops short of general Ada overload resolution. Only the
generated callables the backend emits today are modelled; future generated
subprogram names extend the same representation.

### Bounded audit of other synthesized Ada identifiers

`Kind` discriminants, `Create`, `Value`, `Optional_String`, `Binary_Vectors`,
`_Kind` companions, `_Kind` literals, repeated helper names, and
required-minimum unbounded helper names were each re-checked against the
renderer branch that emits them. `Kind` and `Create`/`Value` were the two gaps;
the remainder were already registered, each gated on the renderer's own
emission predicate. No further missed identifier was found.

### Final authoritative coverage

Re-measured after the `Kind` and `Create`/`Value` rules were added. Closed
world:

| Release | Ada | Rust | C++ | total |
| --- | ---: | ---: | ---: | ---: |
| UCI 2.5 | 2,731 | 5,375 | 5,378 | 5,557 |
| UCI 2.6 | 2,731 | 5,397 | 5,401 | 5,570 |

Open world:

| Release | Ada | Rust | C++ | total |
| --- | ---: | ---: | ---: | ---: |
| UCI 2.5 | 2,724 | 5,287 | 5,290 | 5,557 |
| UCI 2.6 | 2,724 | 5,309 | 5,313 | 5,570 |

Every figure is **unchanged** from `c07c4518`. The two new rules are real
compiler-confirmed boundaries, but authoritative UCI does not exercise either:
UCI declares no type named `Create` or `Value`, and its single `name="Kind"`
occurrence is a Record element (`WeatherReportType.Kind`, of
`WeatherKindEnum`), not a Choice alternative. A Record field named `Kind` is
legal because no discriminant is generated for a Record. The rules are
therefore validated by synthetic fixtures plus raw GNAT confirmation rather
than by a coverage movement.

### Final selected PositionReport readiness

Re-measured on authoritative UCI 2.5, closed world, after projection-scoped
name analysis:

| Backend | Renderable selected | First blocker |
| --- | ---: | --- |
| Rust | 51/60 | `DateTimeType` |
| Ada | 32/60 | `Acceleration3D_Type` |

Both are unchanged. This contract selects a single closure, so no unselected
declaration was contributing a naming failure to it, and neither new Ada rule
is exercised by the selection. Neither blocker is implemented here.

Service-check cost on full UCI 2.5 measured **~5.9 s per language**, down from
~15 s. Readiness now builds its `CoverageAnalysis` over the much smaller
projected schema instead of the whole of UCI, and still constructs exactly one
analysis and one renderability snapshot per call.

## Final corrective: generated names track emitted entities

### A Schema IR declaration is not a generated declaration

**Historical behaviour.** Generated-name preflight registered the top-level
name of *every* Schema IR declaration:

```text
for declaration in &schema.types:
    reserve declaration_name(language, declaration)
```

That assumed every IR declaration produces one host-language top-level
declaration. It does not. The emission planner and the backends deliberately
omit some declarations, so preflight reserved identifiers that never appear in
the generated source and rejected schemas for collisions that cannot occur.

**Corrected behaviour.** Preflight now registers only the schema-owned names
that the requested world actually emits, derived from the same
`plan_type_emissions()` the backends consume. There is no second emission
model and no isolated `if abstract { skip }`: the plan is formed once per
schema/world and walked directly, so the loop iterates emitted entities rather
than rescanning the schema per declaration.

The rule is exactly:

> A schema declaration's top-level host name participates in name preflight if
> and only if backend generation can emit a top-level entity carrying that
> name in the requested world.

| Entity | Emitted? | Reserves its name? |
| --- | --- | --- |
| Concrete declaration | yes | **yes** |
| Ancestry-only abstract **Record** | no | **no** |
| Abstract **Choice** | yes | **yes** |
| Task 024 abstract-value wrapper | yes | **yes** |
| Task 026 elided target | no | **no** |

**The Record/Choice asymmetry is deliberate and load-bearing.** All three
backends return early from their `TypeKind::Record` arm when `is_abstract` is
set, folding the base's effective fields into each concrete descendant, so an
abstract Record is pure inheritance metadata and writes no type. No backend
skips an abstract **Choice**: it renders normally, and Ada additionally emits
its `_Kind` companion. Collapsing both into one "abstract is never emitted"
rule would have stopped reserving a name that genuinely appears in the output,
converting a false rejection into a false *acceptance*. Verified by direct
probe against all three renderers before the change was written.

Attribution (`unsafe_named_declarations`) uses the identical registration, so
capability analysis cannot condemn a declaration for a name the backend never
emits while validation accepts it.

### Member and helper surfaces come from `TypeEmission` too

Top-level emission awareness was necessary but **insufficient**. Member-region
and Ada helper analysis still walked raw Schema IR declarations, so a
declaration that produces no host-language output could still manufacture
member-name failures and Ada helper reservations. The invariant is now
stronger, and unconditional:

> Generated-name analysis operates on actual `TypeEmission` surfaces, not raw
> Schema IR declarations — for top-level names, member regions, and Ada helper
> types alike.

`register_emission_names()` switches on the emitted shape:

| `TypeEmission` arm | Registers |
| --- | --- |
| `Declaration(D)`, emitted | `D`'s top-level name, `D`'s effective member region, Ada helpers stemmed on **`D`** |
| `Declaration(D)`, not rendered | nothing at all |
| `AbstractValue(P)` | the wrapper's top-level name, Ada `Kind` + `{Descendant}_Value`, `{Base}_Kind`, `{Descendant}_Kind` literals |

**Non-emitted abstract Records have no member or helper scope.** A base that no
renderer writes has no generated record, therefore no component region and no
helper types. It is neither diagnosed for its own members nor allowed to
reserve `{Base}_{Member}_Array`, `_Sequence`, `_Item`, `_Vectors`,
`_Required_Array`, or `_Additional_Vectors`.

**Inherited members are validated under the emitted descendant.** The base's
fields still reach the output through `effective_record_fields()` on each
concrete descendant, and they are checked there — in the scope that really
exists, under the owner `backend-ada` really uses. For

```text
abstract Base { Items : Item [0..4] }
Derived extends Base
```

Ada emits `Derived_Items_Array` / `Derived_Items_Sequence` and never
`Base_Items_*`. Preflight now matches exactly: a user type named
`Base_Items_Array` is accepted (and compiles under GNAT), while
`Derived_Items_Array` is still rejected. A reserved-word member reached only
through inheritance still condemns `Derived`, and no longer condemns `Base`.

**Abstract wrappers use wrapper-specific naming.** A Task 024
`AbstractValue` does not render the original abstract Record's fields, so it is
not fed through Record member validation. Only Ada places user-derived
identifiers in the wrapper's region (`Kind`, `{Descendant}_Value`); Rust's
`Variant(Variant)` enum arms are the descendants' own already-validated
declaration names and C++ emits a single fixed `value` member, so neither
contributes a second collision domain. Because every base field necessarily
reappears in a concrete descendant, member *spelling* cannot distinguish the
wrapper's scope from the original Record's — the helper **owner** can, and is
what the regression asserts.

**Task 026 elided targets contribute nothing.** A fully elided zero-descendant
target produces no `TypeEmission`, hence no top-level, member, helper, or
wrapper names.

### Semantic emission-plan failure is not converted into a naming failure

**Historical behaviour.** When `plan_type_emissions()` failed, preflight fell
back to registering all raw schema declarations. That was intended as
conservatism, but it fabricated names for output that can never exist and could
report a phantom collision *before* the real semantic diagnostic.

**Corrected behaviour.** Name preflight defers instead:

```rust
enum NamePreflightPlan<'a> {
    Planned(Vec<TypeEmission<'a>>),
    UnavailableBecauseSemanticFailure(Vec<TypeEmission<'a>>),
}
```

An entity with no emitted surface contributes no names, so no naming verdict is
invented for it. Ownership stays where it belongs:

> Name preflight diagnoses names of output that *can* exist; semantic emission
> failure remains the authoritative error when no emission surface exists.

Worked example. An `OpenExtensions` schema whose abstract value target is named
`BoundedVec` — the Rust support type's spelling — previously reported a
`BoundedVec` generated-name collision, masking the real cause. It now reports
the semantic diagnostic:

```text
abstract value BoundedVec is not closed under open-extensions generation;
external derived types cannot be represented
```

The same schema under the closed world still fails on the genuine collision,
because there the wrapper really is emitted and really does take the
identifier. Coverage attribution follows the same rule.

**Deferral is scoped to the entity, not to the schema.** This is load-bearing
and was caught by measurement. `plan_type_emissions()` fails **closed and
globally**: a single unrepresentable target aborts the whole plan. Authoritative
UCI contains exactly such a target (`SourceCommandEXT`, zero concrete
structural descendants), so an early version of this corrective that treated a
planner failure as "no surface exists anywhere" silently stopped checking every
other declaration's names and restored the previously **rejected** overclaims
`2800/5395/5417`.

The surfaces are therefore rebuilt from renderer policy per declaration when
the planner aborts — the same entity-selection rule, without the global error
propagation and without the topological ordering, neither of which affects
which names are emitted. Only the entity that genuinely has no emitted shape
loses its names; every other emitted surface is still checked. A dedicated
regression (`a_global_planning_abort_still_checks_unrelated_emitted_names`)
pins this, asserting that an unrelated `foo_bar`/`fooBar` convergence is still
rejected and still attributed to both declarations while whole-schema planning
fails.

### False rejections removed

Both of these previously failed preflight against a *generated support type*
whose identifier the abstract base never actually occupied:

* **Rust** — an ancestry-only abstract `BoundedVec` collided with the
  unconditional `pub struct BoundedVec<T, const MIN, const MAX>` support type.
* **Ada** — an ancestry-only abstract `Optional_String` collided with the
  unconditional `Optional_String` support type.
* **C++** — the same shape against the `BoundedVector` class template.

All three now pass preflight, generate, and compile, with the generated source
containing exactly one definition of the support name. The Task 026 elided
target reserves neither its own name nor an `Optional_String_Kind` companion.

Genuine collisions are unchanged: a **concrete** `BoundedVec` /
`Optional_String` is still rejected, and a real Task 024 wrapper still owns its
declaration name and is still rejected against the support type.

### Final authoritative coverage

Re-measured from scratch on both pinned roots after the correction. **Every
cell is unchanged** from the previous final figures:

| Release / backend | Closed | Open |
| --- | ---: | ---: |
| 2.5 Ada | `2731/5557` | `2724/5557` |
| 2.5 Rust | `5375/5557` | `5287/5557` |
| 2.5 C++ | `5378/5557` | `5290/5557` |
| 2.6 Ada | `2731/5570` | `2724/5570` |
| 2.6 Rust | `5397/5570` | `5309/5570` |
| 2.6 C++ | `5401/5570` | `5313/5570` |

Authoritative UCI therefore **does not exercise this defect**. UCI's 70
abstract declarations are either referenced as values (real Task 024 wrappers,
which still reserve their names) or carry names that collide with nothing, so
no declaration was previously excluded for a non-emitted declaration-name
issue and none becomes renderable here. The correction is a
name-attribution-truth fix, validated by synthetic fixtures and by all three
compilers rather than by a coverage movement. The pre-cleanup
`2800/5395/5417` figures remain **rejected as overclaims**.

**Re-measured again after the member/helper correction**, from scratch on both
pinned roots, in all four release/world combinations. Every one of the twelve
cells is byte-identical to the table above. Authoritative UCI does not exercise
this final emitted-surface defect either: its abstract Records with repeated
fields are all either real wrappers or have descendant helper names that
collide with nothing, so no phantom helper was ever the sole reason for an
exclusion. The evidence for this pass is the synthetic fixtures plus GNAT,
`rustc`, and strict C++17 — not a coverage movement.

### Final selected readiness and cost

Selected `PositionReport`, UCI 2.5, closed world, re-measured: **Rust 51/60**
first blocking `DateTimeType`; **Ada 32/60** first blocking
`Acceleration3D_Type`. Both unchanged — the selection contains no ancestry-only
abstract whose name was being phantom-reserved. Neither blocker is implemented.

Service-check cost measured **~5.8 s per language**, matching the ~5.9 s
projection-scoped figure. Consulting the planner adds one emission plan per
schema/world, not one per declaration, so there is no order-of-magnitude
regression.

Re-measured after the member/helper correction: readiness is **unchanged** —
Rust `51/60` first blocking `DateTimeType`, Ada `32/60` first blocking
`Acceleration3D_Type`, both still NOT READY on the same declarations. Nothing
in the selection depended on a phantom member or helper surface. Service-check
cost re-measured at **5.82 s (Rust) / 5.82 s (Ada)** per language. The emission
plan is still formed exactly once per schema/world and then walked, so member
and helper analysis reuses the same plan rather than re-planning per
declaration.

## Task 034 — Ada optional named Record values

Ada now represents non-nillable `0..1` **named-type Record fields** with
deterministic, generated, value-owning optional wrapper types. This removes
Ada's independent "optional named value" occurrence boundary so the next
blocker can be measured honestly.

### Authoritative evidence

Measured from a fresh clone of the authoritative public UCI Standard
repository, at the pinned roots
`093610b7753944059360d3236770ab446d039556` (2.5) and
`78eb61b6112c8bffa40820c33124b57787fc5bd9` (2.6).

| Optional `0..1` Record fields | UCI 2.5 | UCI 2.6 |
| --- | ---: | ---: |
| Total | 4,949 | 4,967 |
| Named target | 4,458 | 4,548 |
| Direct primitive target | 491 | 419 |
| Nillable | **0** | **0** |
| With field-local constraints | 213 | 214 |
| Named + non-nillable + default constraints | **4,458** | **4,548** |

Target declaration kinds for those named optional fields:

| Target kind | UCI 2.5 | UCI 2.6 |
| --- | ---: | ---: |
| record | 2,097 | 2,107 |
| primitive | 1,383 | 1,431 |
| enumeration | 680 | 705 |
| choice | 298 | 305 |

Two facts shaped the design. Nillability is **entirely absent** from optional
Record fields in both releases, so a two-state wrapper loses nothing real, and
keeping nillability fail-closed costs nothing. Every named optional field with
default local constraints is in the supported subset, so the rule needs no
per-target special-casing.

### The recorded blocker

`Acceleration3D_Type.Timestamp`, verified directly rather than inferred from
the name:

| Property | Value |
| --- | --- |
| cardinality | `0..1` |
| nillable | `false` |
| target QName | `{https://www.vdl.afrl.af.mil/programs/oam}DateTimeType` |
| target kind | primitive (`PrimitiveKind::DateTime`) |
| local constraints | default |

So the occurrence is squarely inside the Task 034 subset, while the *target*
is a temporal primitive this project does not lower. That separation is the
whole point: after Task 034 this field no longer fails for being optional and
named, and `Acceleration3D_Type` is no longer Ada's selected-service blocker.

### Supported subset

An Ada Record field uses the new representation iff **all** hold:

```text
cardinality = 0..1
nillable = false
target = named type
field-local ConstraintSet = default
target/value representation is otherwise supported by existing Ada rules
```

This changes the **occurrence representation only**. It does not make an
unsupported target kind supported. `Timestamp : DateTimeType 0..1` passes the
optional-occurrence rule and still fails, attributed to `DateTimeType`.

### Generated representation

For an emitted Record `Owner` with optional named field `Field : Target 0..1`:

```ada
type Owner_Field_Optional (Is_Present : Boolean := False) is record
   case Is_Present is
      when False => null;
      when True  => Value : Target;
   end case;
end record;
```

stored as `Field : Owner_Field_Optional`. Helper declarations precede the
record that uses them, in the existing deterministic emission order.

The wrapper is **per emitted field** rather than a shared generic runtime
Optional. That keeps the task self-contained, allocation-free beyond whatever
`Target` itself owns, compatible with this backend's `private` and otherwise
constrained generated target types, naturally tied to the emitted concrete
owner, and independent of a runtime package that does not exist yet.

The existing `Optional_String` is unchanged and still serves
`Primitive(String), 0..1`; Task 034 adds a separate named-target path rather
than redesigning it, so unrelated generated output does not churn.

**SPARK-oriented properties.** The discriminant alone controls whether `Value`
exists. There is no access type, no heap allocation introduced by the wrapper
itself, no unchecked conversion, and no in-band sentinel: absence and presence
are explicit, distinguishable states, and reading `Value` through an absent
wrapper raises `Constraint_Error` rather than yielding a wrong value. The GNAT
probe asserts that. No proof annotation and no GNATprove CI is added here; the
representation is intentionally compatible with future Phase 4 work.

### Inherited-owner semantics

Generated helper names belong to the declaration that actually emits them, the
rule PR #34 established for repeated helpers. For

```text
abstract Base
    Maybe : T 0..1

Derived extends Base
```

where `Base` is not emitted, the wrapper is `Derived_Maybe_Optional`, never
`Base_Maybe_Optional`, because `Derived` is the scope where the field is really
rendered. `effective_record_fields(...)` on emitted declarations drives this,
exactly as for repeated helpers.

### Generated-name collision handling

The wrapper is a generated Ada top-level identifier in a flat package, so it
participates in the **shared emitted-surface generated-name model** rather than
an Ada-renderer-only check. `Owner_Maybe_Optional` is reserved only when that
helper is really emitted, and structured ownership points back to `Owner`. A
user declaration spelled `Owner_Maybe_Optional` rejects generation and marks
both responsible declarations under existing attribution semantics. A
non-emitted abstract ancestor reserves nothing, and there is a regression for
exactly that case.

### Coverage delta

Full authoritative re-run from scratch, all four roots, all three backends:

| Root | Backend | Before | After |
| --- | --- | ---: | ---: |
| UCI 2.5 closed | Ada | 2731 / 5557 | **4977 / 5557** |
| UCI 2.5 closed | Rust | 5375 / 5557 | 5375 / 5557 |
| UCI 2.5 closed | C++ | 5378 / 5557 | 5378 / 5557 |
| UCI 2.6 closed | Ada | 2731 / 5570 | **5034 / 5570** |
| UCI 2.6 closed | Rust | 5397 / 5570 | 5397 / 5570 |
| UCI 2.6 closed | C++ | 5401 / 5570 | 5401 / 5570 |
| UCI 2.5 open | Ada | 2724 / 5557 | **4907 / 5557** |
| UCI 2.5 open | Rust | 5287 / 5557 | 5287 / 5557 |
| UCI 2.5 open | C++ | 5290 / 5557 | 5290 / 5557 |
| UCI 2.6 open | Ada | 2724 / 5570 | **4963 / 5570** |
| UCI 2.6 open | Rust | 5309 / 5570 | 5309 / 5570 |
| UCI 2.6 open | C++ | 5313 / 5570 | 5313 / 5570 |

Rust and C++ are byte-identical in every cell, as required. Ada field
occurrences rise from 8211/13160 to 12669/13160 (2.5) and 8231/13198 to
12779/13198 (2.6) — consistent with the 4,458 / 4,548 in-subset named optional
fields the evidence counted, and nothing more. Ada *kinds* and *field-types*
are unchanged, which is the check that no unrelated feature was enabled: Task
034 moved occurrences only.

### Selected PositionReport delta

UCI 2.5, `crates/service-contract/tests/fixtures/upstream-minimal.yaml`, one
selected message, 60-declaration closure, closed world.

| Backend | Before | After | First blocker before | First blocker after |
| --- | ---: | ---: | --- | --- |
| Ada | 32/60 | **44/60** | `{…}Acceleration3D_Type` | `{…}MissionID_Type` |
| Rust | 51/60 | 51/60 | `{…}DateTimeType` | `{…}DateTimeType` (unchanged) |

`Acceleration3D_Type` is no longer a blocker, which is the direct evidence that
Task 034 removed the occurrence barrier. Ada's new blocker, `MissionID_Type`,
is **not** temporal: it inherits `Version : xs:unsignedInt 0..1` from
`VersionedID_Type`, an optional **direct non-String primitive**, which Task 034
deliberately leaves unsupported. That is an honest boundary, reported rather
than engineered around — no readiness or ServicePlan special case exists for
`Acceleration3D_Type` or for any other declaration name.

Ada remains NOT READY, so no temporal support was implemented to force a
readiness increase.

### Full-UCI generation boundary

Full UCI generation is **not** supported. The first Ada full-schema blocker is
unchanged by this task:

```text
unsupported Ada IR construct: Ada name "Range" generates reserved word "Range"
in the members of AltitudeRangePairType
```

Task 034 did not shift it, and no attribution change is claimed.

### What Task 034 does not add

Record fields **only**. Optional named Choice alternatives are deliberately
left fail-closed: a Choice's exclusivity is already carried by its generated
`Kind` discriminant, and no authoritative evidence justifies giving one
alternative a second, nested discriminant. That distinction is enough that it
should not hitchhike into this task.

Also unchanged and still rejected: optional direct non-String primitive fields;
nillable values; optional values with field-local constraints; optional
Alias/List shapes that are otherwise unsupported; unsupported target
declaration kinds. No temporal primitive lowering, no lexical validation, no
constrained String/Binary, no runtime codecs, and no generic CAL/runtime
Optional abstraction. No nillability support of any kind is claimed.
