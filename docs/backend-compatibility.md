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
ams-gra-codegen-oms coverage --schema /path/to/UCI_MessageDefinitions_v2_N_0.xsd
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
