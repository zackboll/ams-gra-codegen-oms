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

## Task 035 — Ada optional direct primitive values

Ada now represents non-nillable `0..1` **direct primitive** Record fields using
the *identical* per-field discriminated wrapper Task 034 introduced. This closes
Ada's last pure occurrence boundary, so the next selected blocker is a genuine
primitive-support question rather than a storage-shape one.

### Corrected evidence: `constraints != default` is not a user-authored facet

Task 035 was scoped expecting `MissionID_Type.Version` to carry **default**
field-local constraints. Direct measurement of normalized IR disproved that, and
the task was re-scoped on the evidence.

The authoritative source text is a bare built-in with no author-written facet:

```xml
<!-- UCI 2.5, UCI_MessageDefinitions_v2_5_0.xsd:107803, in VersionedID_Type -->
<xs:element name="Version" type="xs:unsignedInt" minOccurs="0"/>
```

but the frontend's existing built-in primitive normalization
(`xsd-frontend/src/lib.rs`, `builtin_primitive_semantics`) deliberately turns a
built-in integer type's *domain* into semantic IR bounds, so it normalizes to:

```text
{https://www.vdl.afrl.af.mil/programs/oam}VersionedID_Type   (line 107796)
  Version  cardinality = 0..1
           nillable    = false
           target      = Primitive(UnsignedInteger)
           constraints = minInclusive 0, maxInclusive 4294967295
```

and is inherited by `MissionID_Type` (UCI 2.5 line 53847, UCI 2.6 line 53893),
whose `effective_record_fields()` therefore carries the same field. UCI 2.6 is
identical apart from line numbers (`VersionedID_Type` at 108239, `Version` at
108246).

So `ConstraintSet != default()` does **not** imply "unsupported user-authored
field-local restriction". Using it as the Task 035 gate would have produced zero
integral wrappers and left `MissionID_Type` blocked. Confirming this is
normalization rather than authored facets: across both releases **every** one of
the 213/214 constrained optional integral fields has a constraint set exactly
equal to some XSD built-in's intrinsic domain, and there are **zero**
non-intrinsic optional integral facets.

| Optional integral constraint set | UCI 2.5 | UCI 2.6 |
| --- | ---: | ---: |
| `xs:unsignedInt` domain `0 .. 4294967295` | 129 | 129 |
| `xs:int` domain | 29 | 29 |
| `xs:unsignedByte` domain | 28 | 28 |
| `xs:unsignedShort` domain | 17 | 18 |
| `xs:long` domain | 9 | 9 |
| `xs:short` domain | 1 | 1 |
| **Genuinely non-intrinsic facets** | **0** | **0** |

No constraint provenance was added to Schema IR. Instead the classifier asks the
question that actually matters — *can the existing Ada field-value lowering
represent this exact domain without semantic loss?* — which for integers is
already answered by Task 020's `inclusive_integral_domain()`, the same function
`ada_field_base` uses for **required** direct integral fields. That keeps one
integral-domain policy rather than two.

### Direct primitive optional inventory

Optional `0..1` Record fields with a **direct primitive** target, by kind.
Reconciles with the Task 034 totals (2.5: 4,949 total / 4,458 named / 491
direct; 2.6: 4,967 / 4,548 / 419).

| Direct primitive | 2.5 total | 2.5 nillable | 2.5 default | 2.5 constrained | 2.6 total | Task 035 |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| `Boolean` | 83 | 0 | 83 | 0 | 23 | supported |
| `SignedInteger` | 39 | 0 | 0 | 39 | 39 | supported (Task 020 domain) |
| `UnsignedInteger` | 174 | 0 | 0 | 174 | 175 | supported (Task 020 domain) |
| `Float32` | 41 | 0 | 41 | 0 | 41 | supported |
| `Float64` | 141 | 0 | 141 | 0 | 141 | supported |
| `Binary` | 3 | 0 | 3 | 0 | 0 | supported |
| `String` | 0 | — | — | — | 0 | unchanged (`Optional_String`) |
| `DateTime` | 3 | 0 | 3 | 0 | 0 | **unsupported primitive** |
| `Duration` | 7 | 0 | 7 | 0 | 0 | **unsupported primitive** |
| `Time`, `Decimal` | 0 | — | — | — | 0 | **unsupported primitive** |

Every constrained case is `minInclusive+maxInclusive`. **Nillable count is 0 in
every kind and both releases**, so nillability stays fail-closed on evidence.

In the UCI 2.5 selected `PositionReport` closure the only direct primitive
optional occurrence is `VersionedID_Type.Version` / `MissionID_Type.Version`
(`UnsignedInteger`), which is exactly the boundary this task removed.

### Supported subset and representation

A Record field uses the wrapper iff cardinality is exactly `0..1`, it is **not**
nillable, and either the target is a **named** type with default field-local
constraints (Task 034) or a direct primitive Ada can already represent:

| Direct primitive | Gate | Wrapped value type |
| --- | --- | --- |
| `Boolean` | default constraints | `Boolean` |
| `SignedInteger` | `inclusive_integral_domain().is_ok()` | `Long_Long_Integer`, with range when bounded |
| `UnsignedInteger` | `inclusive_integral_domain().is_ok()` | `Interfaces.Unsigned_64`, with range when bounded |
| `Float32` | default constraints | `Interfaces.IEEE_Float_32` |
| `Float64` | default constraints | `Interfaces.IEEE_Float_64` |
| `Binary` | default constraints | `Binary_Vectors.Vector` |

The value type always comes from the backend's existing `ada_field_base()`, so
no second primitive mapping exists. Generated shape, e.g. for
`xs:unsignedInt 0..1`:

```ada
type Owner_Version_Optional (Is_Present : Boolean := False) is record
   case Is_Present is
      when False => null;
      when True  => Value : Interfaces.Unsigned_64 range 0 .. 4294967295;
   end case;
end record;
...
   Version : Owner_Version_Optional;
```

`Optional_String` is **unchanged**: direct optional String still uses the shared
type and gains no per-field wrapper, so no already generated output churns.

### One shared classification

Task 034 kept closely mirrored predicates in generation and in name analysis.
Task 035 replaces both with a single `codegen-core::ada_optional` module
(`ada_record_field_uses_optional_wrapper`, plus
`ada_optional_direct_primitive_representable`), consumed by `backend-ada`, the
generated-name preflight, and coverage occurrence classification. No third
occurrence model was introduced.

Name registration still happens **after** `field_storage_semantics(...)`, so the
Task 026 corrective from PR #35 is preserved: an `AbsentOnly` or semantic-error
field contributes no member or helper surface. Wrapper names use the **emitted**
owner, so an inherited optional field on a non-emitted abstract ancestor renders
as `Derived_Version_Optional` and never `Base_Version_Optional`.

### Occurrence support and primitive support stay separate

The capability model keeps two independent questions. `DateTime`, `Time`,
`Duration`, and `Decimal` all have a conceivable storage shape inside this
wrapper, but `primitive_ref_renderable` still reports them unsupported, so
`field_renderable == false` and the diagnostic still blames the primitive/target
rather than the cardinality. A future Task 036 that adds temporal primitive
support is what should flip those — not Task 035's occurrence work.

### Fail-closed boundaries (unchanged by this task)

Nillable optionals; integral shapes Task 020 rejects (exclusive bounds,
half-open ranges, contradictory ranges, `length`/`minLength`/`maxLength`,
lexical facets); field-local facets on optional Boolean/float/Binary; temporal
and Decimal primitives; optional **Choice alternatives**, named or direct
primitive. No facet is silently dropped into an unconstrained wrapper.

### Authoritative coverage

Re-measured from scratch at the pinned roots. Rust and C++ are **unchanged in
all eight of their cells**, matching the Task 034 figures exactly.

| Release / world | Ada before | Ada after | Rust | C++ | Total |
| --- | ---: | ---: | ---: | ---: | ---: |
| UCI 2.5 closed | 4,977 | **5,298** | 5,375 (=) | 5,378 (=) | 5,557 |
| UCI 2.6 closed | 5,034 | **5,319** | 5,397 (=) | 5,401 (=) | 5,570 |
| UCI 2.5 open | 4,907 | **5,213** | 5,287 (=) | 5,290 (=) | 5,557 |
| UCI 2.6 open | 4,963 | **5,234** | 5,309 (=) | 5,313 (=) | 5,570 |

Ada field-occurrence coverage reaches **13,150/13,160** (2.5) and
**13,198/13,198** (2.6, i.e. 100%). The 10 remaining 2.5 occurrences are the
`DateTime`/`Duration` optional fields, which are primitive-support gaps.

Declaration-set diffs are strictly monotone — **no declaration was lost in any
cell**:

| Cell | Newly renderable | Regressions |
| --- | ---: | ---: |
| UCI 2.5 closed | 321 | 0 |
| UCI 2.6 closed | 285 | 0 |
| UCI 2.5 open | 306 | 0 |
| UCI 2.6 open | 271 | 0 |

Attribution of the 321 UCI 2.5 closed gains: **313** directly contain at least
one optional direct primitive field inside the Task 035 subset. The remaining
**8** are abstract Task 024 closed-sum bases — `CapabilityBaseType`,
`CapabilityStatusBaseType`, `ComponentExtendedStatusPET`,
`DataLinkIdentifierPET`, `DataLinkNativeFilterPET`, `DataLinkNativeInfoPET`,
`OpZoneFilterAreaPET`, `STANAG_4607_PackingPlanPET` — which became renderable
transitively because their last blocking concrete descendant did (for example
`DataLinkIdentifierPET` via `EW_CoordinationDataLinkIdentifierType`, and
`OpZoneFilterAreaPET` via `MTI_OpZoneFilterAreaType`). Every gain is therefore
attributable to the declared subset.

Effective field-occurrence delta by primitive kind, UCI 2.5 (occurrences now
wrapped, counted over effective fields so inherited copies are included):

| Kind | Occurrences |
| --- | ---: |
| `UnsignedInteger` | 240 |
| `Float64` | 154 |
| `Boolean` | 88 |
| `Float32` | 42 |
| `SignedInteger` | 41 |
| `Binary` | 3 |

### Selected PositionReport delta

UCI 2.5, `crates/service-contract/tests/fixtures/upstream-minimal.yaml`, one
selected message, 60-declaration closure, closed world.

| Backend | Before | After | First blocker before | First blocker after |
| --- | ---: | ---: | --- | --- |
| Ada | 44/60 | **47/60** | `{…}MissionID_Type` | `{…}DateTimeType` |
| Rust | 51/60 | 51/60 | `{…}DateTimeType` | `{…}DateTimeType` (unchanged) |

`MissionID_Type` and `VersionedID_Type` are both renderable now, which is the
direct evidence that Task 035 removed the occurrence barrier. Ada's new first
blocker, `DateTimeType`, is the *same* blocker Rust has had since Task 033 — a
temporal **primitive** gap, not an occurrence gap. Ada remains NOT READY and no
temporal support was implemented to force a readiness increase. No readiness,
`ServicePlan`, or projection special case exists for any of these names.

### Full-UCI generation boundary

Full UCI generation is still **not** supported, and the first Ada full-schema
blocker is unchanged by this task:

```text
unsupported Ada IR construct: Ada name "Range" generates reserved word "Range"
in the members of AltitudeRangePairType
```

Task 035 did not shift it and does not address it.

### What Task 035 does not add

Record fields only. No IR constraint provenance. No new constraint semantics. No
temporal or Decimal lowering, no timezone/`Z` handling, no lexical/regex runtime,
no datetime parser, no JSON codec, no temporal arithmetic. No nillability of any
kind. No constrained String/Binary. No generic CAL/runtime Optional abstraction.
No UCI-name special case exists for `MissionID_Type`, `VersionedID_Type`, or
`Version` — the regressions use synthetic fixtures with ordinary inheritance.

## Task 036 — validated Zulu DateTime values

Tested: **2026-09-21**. GNAT 14.2.0 (CI 13.3.0), rustc 1.98.1, g++ 14.2.0.

Task 036 implements one temporal vertical slice: a **named**
`PrimitiveKind::DateTime` declaration whose effective lexical constraint is
exactly the authoritative UCI Zulu profile `pattern=".+Z"`, in Ada, Rust, and
C++. It is deliberately narrower than "DateTime + Time + Duration": temporal
lexical/value semantics need a rigorously tested representation before the
capability family is broadened.

### Authoritative UCI temporal inventory

Reconfirmed from the pinned release bytes and from the normalized IR, not from
prior notes. Pinned revisions `093610b7753944059360d3236770ab446d039556` (2.5)
and `78eb61b6112c8bffa40820c33124b57787fc5bd9` (2.6).

| Release | Declaration | Source | Primitive | Immediate base | Effective `ConstraintSet` |
| --- | --- | --- | --- | --- | --- |
| 2.5 | `DateTimeType` | `UCI_MessageDefinitions_v2_5_0.xsd:117038` | `DateTime` | `xs:dateTime` | 1 pattern group, 1 alternative, XML Schema dialect, `".+Z"` |
| 2.5 | `TimeType` | `UCI_MessageDefinitions_v2_5_0.xsd:145152` | `Time` | `xs:time` | 1 pattern group, 1 alternative, `".+Z"` |
| 2.5 | `DurationType` | `UCI_MessageDefinitions_v2_5_0.xsd:117745` | `Duration` | `xs:duration` | empty |
| 2.6 | `DateTimeType` | `UCI_SecurityMarkings_v2_6_0.xsd:1203` | `DateTime` | `xs:dateTime` | 1 pattern group, 1 alternative, `".+Z"` |
| 2.6 | `TimeType` | `UCI_MessageDefinitions_v2_6_0.xsd:145403` | `Time` | `xs:time` | 1 pattern group, 1 alternative, `".+Z"` |
| 2.6 | `DurationType` | `UCI_MessageDefinitions_v2_6_0.xsd:118281` | `Duration` | `xs:duration` | empty |

In every case: named restriction-chain depth 1 (a direct restriction of the
built-in), **no** explicit `whiteSpace` facet, and **no** neighbouring facet of
any kind. Each carries UCI documentation stating that the W3C definition is
used verbatim "with a further restriction that only the *Zulu* time zone be
used" (`DateTimeType`, `TimeType`) or verbatim with no restriction
(`DurationType`).

The prior evidence is confirmed exactly:

```text
DateTimeType : DateTime + pattern ".+Z"
TimeType     : Time     + pattern ".+Z"
DurationType : Duration + no facets
```

The only cross-release difference is *where* `DateTimeType` is declared: in 2.5
it lives in the message-definitions document, in 2.6 it moved to the included
security-markings document. Its profile is byte-identical, so Task 036 is not a
2.5-only special case and both releases behave the same.

### Direct temporal reference inventory

Re-counted from the source bytes:

| Release | direct `xs:dateTime` | direct `xs:duration` | direct `xs:time` |
| --- | ---: | ---: | ---: |
| UCI 2.5 | 4 | 9 | 0 |
| UCI 2.6 | 0 | 0 | 0 |

All 2.5 `xs:dateTime` fields are `0..1` except `SystemTimeAtLastReference`
(`1..1`, line 103605); the others are `CurrentSystemTime` (103610) and, in the
included security-markings document, `DeclassDate` (210) and
`CUI_DecontrolDate` (245). The nine `xs:duration` fields are `0..1` except
`MinimumRangeAnalysisDuration` (62551) and `CollectionTime` (71866). UCI 2.6
has **no** direct temporal field at all.

**Task 036 does not support any of these.** They are direct primitive
references, not named declarations, and remain unsupported by deliberate
choice.

### Applicable XML Schema version

**XML Schema 1.0 Part 2: Datatypes, W3C Recommendation 28 October 2004.**

This was determined rather than assumed. Both pinned schemas declare only
`xmlns:xs="http://www.w3.org/2001/XMLSchema"` and contain **zero** occurrences
of every XSD-1.1-only construct checked: `xs:assert`, `xs:alternative`,
`explicitTimezone`, `xs:openContent`, `xs:override`, `defaultAttributes`,
`xs:anyAtomicType`, and `vc:minVersion`. Nothing in either document requires or
signals 1.1.

The version genuinely matters here, in **two** places:

* **year `0000`** — XSD 1.0 prohibits it; a later revision admits it as 1 BCE.
  The validator implements the 1.0 rule and the corpus pins
  `0000-01-01T00:00:00Z` as invalid.
* **leap seconds** — XSD 1.0 Appendix D admits second `60`. XSD 1.1 removed
  leap seconds from the value space entirely (its seconds field is `00..59`).
  Because the applicable standard here is **1.0**, second `60` is **accepted**,
  and the corpus pins `1998-12-31T23:59:60Z` as valid.

Both rules are taken from the same selected standard; neither is mixed with
1.1 behaviour.

### The XML Schema rules implemented

From section 3.2.7.1, the lexical space is

```text
'-'? yyyy '-' mm '-' dd 'T' hh ':' mm ':' ss ('.' s+)? (zzzzzz)?
```

* **year** — "a four-or-more digit optionally negative-signed numeral"; if more
  than four digits, leading zeros are prohibited; `0000` is prohibited; a plus
  sign is not permitted;
* **month/day** — two-digit numerals; section 3.2.7 states the value of each
  numeric property is limited by the next-higher property, so "the day value
  can never be 32, and cannot even be 29 for month 02 and year 2002";
* **leap year** — from `maximumDayInMonthFor` (Appendix E): February has 29
  days when `modulo(Y,400) = 0 OR (modulo(Y,100) != 0 AND modulo(Y,4) = 0)`;
* **hour** — two digits; `24` **is permitted** when the minutes and seconds
  represented are zero, denoting the first instant of the following day;
* **minute** — two digits, `00..59`;
* **second** — "a two-integer-digit numeral"; `00..60`. Appendix D states the
  two digits of `ss` "can have values from **0 to 60**" and that "[a] value of
  60 or more is allowed only in the case of leap seconds". Second **60 is
  therefore valid** in the XSD 1.0 lexical space; **61 is not**, because the
  two-digit field itself stops at 60 (see *Leap seconds* below);
* **fractional seconds** — `'.' s+`, so a dot requires at least one digit, to
  arbitrary precision with no digit limit; Appendix D applies this to the whole
  seconds field without exempting `60`, so a point **inside** a leap second
  (`23:59:60.5Z`) carries the same arbitrary precision;
* **timezone** (3.2.7.3) — `(('+' | '-') hh ':' mm) | 'Z'`;
* **whiteSpace** (4.3.6) — for every atomic datatype other than `string` and
  its restrictions the value is `collapse` and "cannot be changed by a schema
  author". `collapse` replaces tab/LF/CR with space, squeezes runs of spaces,
  and trims the ends;
* **pattern ordering** — Datatype Valid applies `pattern` to the literal, which
  has already undergone the type's whitespace normalization;
* **multiple patterns** (4.3.4.3) — alternatives at one derivation step are
  OR-ed; patterns at different steps are AND-ed;
* **regex** (Appendix F) — `.` is the `WildcardEsc` production [37a],
  equivalent to the character class `[^\n\r]`; a literal is pattern-valid only
  if the pattern matches it **entirely**.

### Leap seconds

The seconds rule is worth stating in full, because the initial Task 036
validator got it wrong by imposing a `00..59` limit.

**The rule implemented.** The whole-seconds field is accepted for `00..60`.
`60` is the leap second; `61` and above are rejected. A `60` may carry a
fractional part to the same arbitrary precision as any other second. No
calendar-position test and no historical-table lookup is applied. Hour `24`
independently continues to require zero minutes and zero represented seconds,
so `24:00:60Z` is rejected by that rule, not by the seconds rule.

**Why `60` is valid.** Appendix D, describing the `s` picture character: "The
two digits in a `ss` format can have values from **0 to 60**. In the formats
described in this specification the whole number of seconds *may* be followed
by decimal seconds to an arbitrary level of precision. … A value of 60 or more
is allowed only in the case of leap seconds."

**Why `61` is not.** "60 or more" has to be reconciled with the two-digit `ss`
field and the separate fractional production. The field is bounded at 60 by the
same sentence; values "more" than 60 are reached through the *fraction* on
second 60 (`60.5`), never by a two-digit numeral above 60. `61` is therefore
outside the lexical space, and the corpus pins `12:00:61Z` and `12:00:61.5Z` as
invalid.

**Why fractional leap seconds are valid.** The arbitrary-precision clause is
attached to "the whole number of seconds" with no exemption for `60`. A time
point *within* the leap second is representable, so `23:59:60.1Z`,
`23:59:60.5Z`, and `23:59:60.999999Z` are all accepted.

**Why no IERS table is required.** Appendix D says a `60` is "[s]trictly
speaking … not sensible unless the month and day could represent March 31,
June 30, September 30, or December 31 in UTC" — but it does **not** reject
other placements. The very next sentence supplies a *value* mapping instead:
"In cases where the leap second is used with an inappropriate month and day it,
and any fractional seconds, should [be] considered as added or subtracted from
the following minute." That is a statement about the value space, not a
restriction on the lexical space, so the lexical validator must accept second
`60` on **any** date. This is deliberate breadth, not an oversight: Appendix E
says outright that "[a] definition that attempted to take leap-seconds into
account would need to be constantly updated, and could not predict the results
of future implementation's additions", attributing leap-second decisions to the
IERS. The generated validators therefore stay self-contained — no leap-second
table, no network access, no OS calendar or timezone database, no third-party
date library.

So the answer to "must second `60` be restricted to dates on which the IERS
actually inserted a leap second?" is **no**. XSD 1.0 permits the broader
lexical representation, and the corpus pins non-historical placements such as
`2026-09-20T12:00:60Z` as valid.

**Leap second is not leap year.** The two are independent. The February and
`maximumDayInMonthFor` logic is unchanged, and the corpus pins
`2023-12-31T23:59:60Z` — a leap second in a non-leap year — as valid.

**Lexical spelling is preserved.** A valid leap-second literal round-trips as
written. `1998-12-31T23:59:60Z` is stored and returned unchanged; it is *not*
normalized to `1999-01-01T00:00:00Z`. Task 036 is a lexical carrier and
introduces no temporal arithmetic.

### Why `.+Z` is interpreted, not run through a regex engine

Task 036 introduces no XML Schema regular-expression engine. Instead it proves,
for this one expression, that pattern matching reduces to a decision the
generated code can make directly.

For a string `s` that has **already** passed the `dateTime` lexical validator,
`.+Z` matches `s` exactly when `s` is non-empty, ends in a literal `Z`, and the
prefix that `.+` consumes contains no `#xA` or `#xD`.

That last condition is free, twice over. The intrinsic `whiteSpace` of
`dateTime` is a *fixed* `collapse`, and the pattern is applied after
normalization, so no tab, line feed, or carriage return survives.
Independently, the `dateTime` grammar admits only digits and `-`, `T`, `:`,
`.`, `+`, `Z` — no line terminator can be lexically valid anyway.

Therefore, on the post-normalization, lexically-valid `dateTime` domain:

```text
pattern ".+Z"  ==  the normalized lexical form ends in literal 'Z'
```

and by section 3.2.7.3 the only `dateTime` timezone spelling ending in `Z`
**is** `Z`, the Zulu/UTC form. That is the equivalence implemented, and it is
claimed for this expression alone. Any other pattern text, a second
alternative, a second group, or a non-XML-Schema dialect fails closed, because
the proof does not carry over.

### The shared classifier

`crates/codegen-core/src/temporal.rs` holds the single language-neutral
decision. `backend-ada`, `backend-rust`, `backend-cpp`, and `CoverageAnalysis`
all consult it; none re-derives `kind == DateTime && pattern == ".+Z"`.

```rust
pub fn temporal_profile(
    kind: PrimitiveKind,
    constraints: &ConstraintSet,
) -> Result<Option<TemporalProfile>, TemporalProfileError>;
```

Three outcomes are kept distinct so diagnostics stay precise:

| Result | Meaning |
| --- | --- |
| `Ok(Some(DateTimeZulu))` | the implemented profile; backends render it |
| `Ok(None)` | not a temporal declaration at all; Task 036 has no opinion |
| `Err(TimeUnsupported)` | a `Time` declaration, whatever it carries |
| `Err(DurationUnsupported)` | a `Duration` declaration, whatever it carries |
| `Err(DateTimeUnconstrained)` | a `DateTime` with no lexical restriction |
| `Err(DateTimeUnsupportedConstraints)` | a `DateTime` with some other facet shape |

Collapsing "not temporal" into "temporal but unsupported" would let a backend
report a `Duration` as though it were an integer, so the two stay apart.

Recognition reads the **normalized Task 016 IR** structurally — group count,
alternative count, dialect, expression text — never a rendered debug string
that a formatting change could silently alter. The supported profile is exactly
one group, holding exactly one XML-Schema-dialect alternative whose expression
is the three characters `.+Z`, with no `whiteSpace` facet and no neighbouring
numeric or length facet.

The `dateTime` type genuinely admits the ordering facets (3.2.7.6), and a
lexical carrier that performs no value-space comparison cannot enforce them, so
their presence is a rejection rather than a silent drop.

**No UCI local name appears in any implementation path.** Support arises solely
from `PrimitiveKind::DateTime` plus the effective `ConstraintSet`, so a
differently named declaration with the same profile is equally supported, and a
`DateTimeType` carrying a different profile is equally unsupported. Every
fixture deliberately uses non-UCI names (`Instant`, `Deadline`).

### The representation: a validated lexical carrier

The generated type stores the **whitespace-normalized, lexically valid XML
Schema `dateTime` spelling** that satisfied the Zulu profile.

It is deliberately **not** Unix epoch seconds, a fixed-width integer timestamp,
`std::chrono`, `Ada.Calendar.Time`, a third-party Rust date/time crate, or any
platform time type. Each of those silently narrows the XML Schema space: the
year is unbounded in digit count, fractional seconds have arbitrary precision,
and `24:00:00` is a distinct legal spelling of an instant those types cannot
represent as written. Converting would also discard the wire-level information
a future codec needs.

**What the type promises:**

1. the stored string has undergone XML Schema-required whitespace
   normalization;
2. it is a valid lexical representation of XML Schema 1.0 `dateTime`;
3. it satisfies the UCI `".+Z"` lexical restriction;
4. therefore its timezone spelling is the required Zulu form.

**What it does not promise, and must not be described as providing:** date/time
arithmetic, ordering, timezone conversion, canonical UTC conversion, duration
arithmetic, semantic equivalence between two distinct legal lexical spellings,
or XML/JSON encoding.

### Equality boundaries

XML Schema `dateTime` value equality is **not** string equality.
`2026-09-20T24:00:00Z` and `2026-09-21T00:00:00Z` denote the same instant with
different spellings, and trailing fractional zeros are likewise
value-preserving. The order relation is only *partial* (3.2.7.4).

Accordingly:

* **Rust** — the carrier derives only `Clone, Debug`. No `PartialEq`, `Eq`,
  `PartialOrd`, or `Ord`. That absence propagates: a record or choice
  transitively holding a carrier omits `PartialEq` too, which is why
  `declaration_supports_partial_eq` now exists alongside the older
  `declaration_supports_eq`.
* **C++** — no `operator==`, `operator!=`, `operator<`, `operator<=>`, or
  ordering helper is emitted.
* **Ada** — the private record type inherits the predefined `"="` as a
  consequence of the language model. The generated spec documents, in the
  visible part, that it compares the **stored normalized lexical
  representation** and is **not** XML Schema value-space equality. It is not
  used as semantic conformance evidence anywhere. No ordering operator is
  declared.

### Generated APIs

**Rust** — an opaque carrier with private storage:

```rust
#[derive(Clone, Debug)]
pub struct Instant { lexical: String }

impl Instant {
    pub fn new(value: &str) -> Option<Self>;
    pub fn as_str(&self) -> &str;
}
```

Every parser helper is a private associated function, so Task 036 adds no
module-scope name. No external temporal crate and no regex crate. Compiles
under `rustc --edition 2021`.

**C++17** — an opaque class whose only constructor is private:

```cpp
class Instant {
public:
    static std::optional<Instant> create(std::string_view value);
    const std::string& value() const noexcept;
private:
    explicit Instant(std::string normalized);
    std::string value_;
};
```

Every helper is a private static member function, so no namespace-scope name is
added. Only `<string>`, `<string_view>`, and `<optional>` are used; no external
temporal or regex library. The `#include <string_view>` line is emitted **only**
when a supported temporal declaration exists, so other schemas' headers are
unchanged. Compiles clean under `-std=c++17 -Wall -Wextra -pedantic-errors`.

**Ada** — a private type completed over the package's existing owned-string
representation:

```ada
type Instant is private;
function Create (Value : String) return Instant;
function Value  (Item  : Instant) return String;
```

`Create` normalizes XML whitespace, validates the `dateTime` lexical form,
validates the Zulu profile, and raises `Constraint_Error` on invalid input.
`Value` returns the stored normalized lexical representation. The private
completion is a record holding an `Unbounded_String`; the component name is not
visible, so no client aggregate or type conversion can bypass `Create`.

### The Ada package body

The temporal validator is a real algorithm rather than an expression function,
so it requires a package body. `generate_body` returns `Some` **only** when
`schema_emits_temporal_carrier` is true, so:

* a schema with a supported DateTime declaration emits `.ads` **and** `.adb`;
* every other schema keeps its existing single-`.ads` file set exactly. No
  empty `.adb` is ever written.

Every parser helper is nested inside the declarative part of `Create`, so Task
036 introduces **no** new package-scope generated identifier beyond `Create`
and `Value`, which the shared name model already registers.

### Generalized Ada generated-callable name model

The Task 033 `ADA_FLOAT_CALLABLES` / `ada_constrained_float_owners` were
generalized to `ADA_WRAPPER_CALLABLES` / `ada_wrapper_callable_owners`, which
now model **all** currently generated Ada package-level callables: a
constrained floating wrapper's `Create` / `Value` and a Task 036 DateTime
wrapper's `Create` / `Value`. A DateTime-only collision check was deliberately
not added, because it would have to be kept in sync with the float one forever.

Callables are *tested against* the top-level region rather than inserted into
it, because Ada subprograms overload. Verified against GNAT 14.2.0 by compiling
real generated output (`backend-temporal-mixed-callables.xsd`, four wrappers in
one package) and **running** a client that calls both `Create (String)`
overloads:

```text
function Create (Value : Interfaces.IEEE_Float_64) return BurnRate;
function Create (Value : Interfaces.IEEE_Float_64) return AltitudeMeters;
function Create (Value : String) return Instant;
function Create (Value : String) return Deadline;
-- accepted: the profiles differ in result type

type Create is new Integer;
function Create (Value : String) return Instant;
-- error: "Create" conflicts with declaration
```

The last two `Create` declarations differ *only* in result type, which Ada
resolves by expected type — the strongest case, and the one the model depends
on. An **unsupported** temporal declaration reserves nothing, because it
generates no subprogram; registering a name for output that will never exist
would reject an otherwise generable schema.

### Runtime validation

Each backend enforces real XML Schema lexical validity. A bare
`ends_with('Z')` is explicitly **not** the validator: it would accept
`garbageZ` and `2026-99-99T99:99:99Z`. The order is always normalize → base
grammar and calendar rules → Zulu profile.

**No fixed-width year narrowing.** The year is validated and its leap-year
properties computed from decimal digits; it is never parsed into `i32`, `i64`,
`int`, `time_t`, or `Ada.Calendar.Year_Number`. Divisibility by 4 is decided
from the last two digits (100 is itself a multiple of 4), and the
divisible-by-400 case by reducing the remaining digits modulo 4 one at a time.
A valid XML Schema date does not become invalid because its year exceeds a host
numeric type — the corpus pins a 24-digit year as valid.

### Shared conformance corpus

`tests/fixtures/temporal/datetime-zulu.txt` is the single canonical corpus:
**33 valid** and **74 invalid** cases, each commented with the XML Schema rule
it exercises. Rust-side tests read it and emit the same cases into a probe
program for **all three** backends, which are then compiled and run. The three
languages are proven to agree rather than each passing its own curated subset.

Valid cases cover: an ordinary date/time; leap day in a leap year (2024);
**leap day in a century leap year (2000)**, which a naive "not divisible by
100" rule gets wrong; an ordinary non-leap February date; one and many
fractional digits; trailing fractional zeros (legal lexically, forbidden only
in the *canonical* form, so they must round-trip unchanged); `24:00:00` and
`24:00:00.0`; a five-digit extended year; a **24-digit** year; negative and
negative-extended years; the smallest legal boundaries
(`0001-01-01T00:00:00Z`); 30- and 31-day month maxima; second 59; and four
leading/trailing whitespace shapes that must collapse away.

Valid **leap-second** cases cover: the historical control
`1998-12-31T23:59:60Z`; three fractional leap seconds (`.1`, `.5`, `.999999`),
proving arbitrary precision inside the leap second; second 60 on each
quarter-end date and on an ordinary date (`2026-09-20T12:00:60Z`), proving no
calendar-position or IERS-table restriction is applied; second 60 at the start
of a day; a leap second in a **non-leap year** (`2023-12-31T23:59:60Z`),
proving leap second and leap year are independent; and a
whitespace-surrounded leap second that must collapse to the bare literal. Each
round-trips to its own spelling, pinning that no rollover normalization occurs.

Invalid cases cover: missing `Z`; five numeric offsets including `+00:00` and
`-00:00` (same *value* as Zulu, wrong *spelling*); lowercase `z`; trailing
characters after `Z`; months 00 and 13; day zero; day beyond a 30- and a
31-day month; **February 29 in 2023** (non-leap) and **in 1900** (century
non-leap, which a naive divisible-by-4 rule wrongly accepts); February 30 in a
leap year; hour 25; `24` with non-zero minute, second, or fraction; **hour 24
combined with second 60** (`24:00:60Z`, `24:00:60.0Z`), which stays invalid
because `24` requires zero represented seconds; minute 60; minute 60 together
with second 60, since a leap second lengthens the seconds field and not the
minutes field; **seconds 61, 61.5, and 99**, the field bound above the leap
second; a bare dot with no digits; non-digit and double-dot
fractions; a fraction on minutes; years of one to three digits; `0000` and
`-0000`; leading-zero five-digit years; a plus-signed year; non-digit years;
wrong field widths; lowercase `t`; missing separators and components; four
internal whitespace positions; the empty and whitespace-only strings; and six
arbitrary-text-ending-in-`Z` cases including `garbageZ` and
`2026-99-99T99:99:99Z`.

### No public bypasses

Compiler-enforced, not asserted at runtime:

* **Rust** — a struct literal naming `lexical`, and a field read of `lexical`,
  both fail to compile with a privacy diagnostic when the generated code is
  placed in a module;
* **C++** — direct construction and a read of `value_` both fail to compile
  with an access-control diagnostic;
* **Ada** — a client aggregate naming the private component and a type
  conversion from `String` are both GNAT compile errors.

### Coverage model

Baseline declaration renderability now accepts a named
`PrimitiveKind::DateTime` declaration with the exact Task 036 profile in all
three backends, decided by the shared classifier so coverage cannot drift from
what the backends emit.

`PrimitiveKind::DateTime` is **not** marked supported wholesale. In particular
`primitive_ref_renderable` still reports a **direct** `xs:dateTime` field
unsupported — this distinction is the whole point of the task, and is pinned by
`crates/codegen-core/tests/temporal_coverage.rs`.

No new `FeatureFamily` was added. `PrimitiveExpansion` still represents
hypothetical general primitive support and `ConstrainedSimpleTypes` generic
unimplemented lexical-constraint support; Task 036 is a concrete baseline
exception for one fully implemented semantic profile. Feature monotonicity is
maintained.

### Authoritative coverage

All twelve cells re-run. Every cell moved by exactly **+1**:

| World | Release | Backend | Before | After |
| --- | --- | --- | ---: | ---: |
| closed | 2.5 | Ada | 5298 / 5557 | **5299 / 5557** |
| closed | 2.5 | Rust | 5375 / 5557 | **5376 / 5557** |
| closed | 2.5 | C++ | 5378 / 5557 | **5379 / 5557** |
| closed | 2.6 | Ada | 5319 / 5570 | **5320 / 5570** |
| closed | 2.6 | Rust | 5397 / 5570 | **5398 / 5570** |
| closed | 2.6 | C++ | 5401 / 5570 | **5402 / 5570** |
| open | 2.5 | Ada | 5213 / 5557 | **5214 / 5557** |
| open | 2.5 | Rust | 5287 / 5557 | **5288 / 5557** |
| open | 2.5 | C++ | 5290 / 5557 | **5291 / 5557** |
| open | 2.6 | Ada | 5234 / 5570 | **5235 / 5570** |
| open | 2.6 | Rust | 5309 / 5570 | **5310 / 5570** |
| open | 2.6 | C++ | 5313 / 5570 | **5314 / 5570** |

Renderable *kinds* likewise moved 5428 to 5429 (2.5) and 5441 to 5442 (2.6).

**The single newly renderable declaration, in every cell, is
`{https://www.vdl.afrl.af.mil/programs/oam}DateTimeType`** — the DateTime
declaration itself. There are **no** transitive consumers in the delta.

That the delta is exactly one is itself evidence. Every UCI type that
*references* `DateTimeType` does so alongside other still-unsupported types
(constrained Strings, chiefly), so no consumer became fully renderable from
this change alone. The delta is identical in the closed and open worlds because
the supported profile is a simple type with no abstract-value participation:
the world policy governs abstract structural values, which a `DateTime`
declaration has none of. `TimeType` and `DurationType` remain non-renderable in
all twelve cells, which is why the delta is +1 rather than +3.

### PositionReport

Authoritative UCI 2.5,
`crates/service-contract/tests/fixtures/upstream-minimal.yaml`, closed world.
C++ was measured too, not inferred:

| Backend | Before | After | First blocker now |
| --- | ---: | ---: | --- |
| Ada | 47 / 60, `DateTimeType` | **48 / 60** | `UCI_SchemaVersionStringType` |
| Rust | 51 / 60, `DateTimeType` | **52 / 60** | `UCI_SchemaVersionStringType` |
| C++ | 51 / 60, `DateTimeType` | **52 / 60** | `UCI_SchemaVersionStringType` |

`DateTimeType` is gone as a blocker in every backend. The measured next blocker
is a **constrained String**, which matches the prior expectation — but is
reported here because it was measured, not assumed. Task 036 does not implement
it. `PositionReport` remains NOT READY everywhere.

The selected 60-type closure contains exactly one temporal declaration,
`DateTimeType`, and no direct temporal field.

### Full-schema boundary

Unchanged by Task 036; all three are unrelated reserved-word boundaries:

```text
Ada:  Ada name "Range" generates reserved word "Range"
        in the members of AltitudeRangePairType
Rust: Rust name "Type" generates reserved word "type"
        in the members of ConfigurationParameterType
C++:  C++ name "Operator" generates reserved word "operator"
        in the members of ApprovalResponseType
```

No full-UCI generation claim is made, and no unrelated blocker was fixed.

### Task 034 composition, and Task 035 unchanged

A `Timestamp : DateTimeZulu 0..1` field composes **automatically**: the
declaration became renderable, and the existing Task 034
`Owner_Field_Optional` wrapper handles the optional named occurrence in Ada. No
optional-DateTime path was added anywhere. Rust uses the ordinary
`Option<Instant>` and C++ the ordinary `std::optional<Instant>`.

The Task 035 direct-primitive optional classifier was **not** modified. A
direct `Primitive(DateTime)` field remains unsupported in occurrence *and* in
value, and those two questions stay separate.

### Explicit non-goals

| Capability | Status |
| --- | --- |
| named DateTime Zulu profile | **supported** |
| direct `xs:dateTime` field | unsupported |
| named unconstrained `DateTime` | unsupported |
| `DateTime` with a different pattern | unsupported |
| `DateTime` with multiple patterns or alternatives | unsupported |
| `DateTime` with an unsupported neighbouring facet | unsupported |
| `Time`, including the identical UCI `.+Z` | unsupported |
| `Duration` | unsupported |
| `Decimal` | unsupported |
| generic XML Schema regex translation | not implemented, deliberately |
| temporal arithmetic / ordering / value equality | not implemented |
| timezone conversion / canonicalization | not implemented |
| XML or JSON codecs | not implemented |

No third-party runtime dependency was added in any language.

## Task 037 — validated schema-version String values

Tested: **2026-09-21**. GNAT 14.2.0, rustc 1.98.1, g++ 14.2.0.

Task 037 implements one constrained-String vertical slice: a **named**
`PrimitiveKind::String` declaration whose effective facets are exactly the
authoritative UCI schema-version profile, in Ada, Rust, and C++. It is
deliberately narrower than "constrained String": UCI 2.5 carries 125 patterned
`xs:string` owners across four facet-shape families, and each family needs its
own evidence and its own validator.

### Authoritative profile

Read from the pinned release bytes, not from prior notes. The two tracked
releases are **byte-identical** for this declaration.

| Release | Pinned revision | Source | Line |
| --- | --- | --- | ---: |
| UCI 2.5 | `093610b7753944059360d3236770ab446d039556` | `UCI_MessageDefinitions_v2_5_0.xsd` | 145460 |
| UCI 2.6 | `78eb61b6112c8bffa40820c33124b57787fc5bd9` | `UCI_MessageDefinitions_v2_6_0.xsd` | 145708 |

The exact original fragment:

```xml
<xs:simpleType name="UCI_SchemaVersionStringType" uci:version="000.001.000.000">
  <xs:annotation>
    <xs:documentation>String representing the UCI version.</xs:documentation>
  </xs:annotation>
  <xs:restriction base="xs:string">
    <xs:minLength value="7"/>
    <xs:maxLength value="57"/>
    <xs:pattern value="[0-9]{3}\.[0-9]{1,2}(\.[0-9]{1,2})([a-z]{1,2})?(_[a-zA-Z0-9\-]{1,45})?"/>
  </xs:restriction>
</xs:simpleType>
```

| Property | Value |
| --- | --- |
| base type | `xs:string` |
| immediate / primitive base | `xs:string` |
| restriction-chain depth | 1 |
| `length` | absent |
| `minLength` | 7 |
| `maxLength` | 57 |
| `whiteSpace` | absent, so the intrinsic `preserve` applies |
| PatternGroups | 1 |
| PatternExpressions | 1 |
| pattern dialect | XML Schema |
| documentation | "String representing the UCI version." |

Normalized IR, confirmed by running this project's frontend over the pinned
2.5 root rather than by reading the XSD twice:

```text
kind                     = Primitive(String)
min_length               = Some(7)
max_length               = Some(57)
length                   = None
lexical.white_space      = None
lexical.pattern_groups   = [ PatternGroup { alternatives: [
    PatternExpression { dialect: XmlSchema, expression:
      "[0-9]{3}\\.[0-9]{1,2}(\\.[0-9]{1,2})([a-z]{1,2})?(_[a-zA-Z0-9\\-]{1,45})?" } ] } ]
```

### The profile is NOT a four-group dotted version

This is the most important evidence finding of the task, and it contradicts
the shape a reader would infer from the type's own
`uci:version="000.001.000.000"` attribute.

The authoritative pattern admits exactly **three** dot-separated numeric
groups, of widths 3, 1–2, and 1–2, plus two optional suffixes. The four-group
form `000.001.000.000` is **rejected** by the very type that carries it as an
attribute. The documentation on the referencing `SchemaVersion` element agrees,
giving `002.0` and `001.9b` as the intended shape.

Implementing the conventional-looking "3 digits . 3 digits . 3 digits . 3
digits" structure would have produced a validator that rejects real UCI
versions such as `002.5.0` while accepting values the schema forbids. The
shared corpus pins this as an explicit negative case, and a classifier unit
test pins it again at the IR level.

### Structure actually implemented

```text
[0-9]{3}                  exactly 3 ASCII digits
\.                        one literal '.'
[0-9]{1,2}                1-2 ASCII digits
(\.[0-9]{1,2})            '.' then 1-2 ASCII digits   -- REQUIRED, unquantified
([a-z]{1,2})?             optional 1-2 ASCII lowercase letters
(_[a-zA-Z0-9\-]{1,45})?   optional '_' then 1-45 ASCII alnum or '-'
```

The third group is parenthesized but carries **no** quantifier, so it is
required, not optional. Reading it as optional would have wrongly accepted
`002.5`.

### Applicable XML Schema 1.0 Part 2 semantics

| Question | Answer, and why it matters here |
| --- | --- |
| Is whitespace preserved, replaced, or collapsed? | **Preserved.** `xs:string` is the one atomic datatype whose intrinsic `whiteSpace` is `preserve` and is *not* fixed (§4.3.6). This declaration adds no `whiteSpace` facet, so normalization is the identity. |
| When is `pattern` evaluated? | After whitespace normalization, as part of Datatype Valid (§4.3.4). Since normalization is the identity here, the pattern applies to the literal exactly as written. |
| What is the unit for String `length`? | **Characters**, never bytes or UTF-16 code units (§4.3.1–4.3.3). |
| Must a pattern match the whole literal? | **Yes** (§4.3.4.3), so leading and trailing junk are rejected rather than ignored. |
| Which Unicode/XML semantics matter? | Only that the character classes used here are ASCII; no `\p{...}`, no `\i`/`\c` XML-name escapes, no wildcard. |
| Is every valid value ASCII? | **Yes**, which is what licenses byte counting. |

Because `whiteSpace` is `preserve`, the generated carriers store the caller's
input **unchanged**. A value with leading, trailing, or interior whitespace is
*not* repaired into a valid one; it is rejected, since no whitespace character
appears in any character class. This is the opposite of Task 036, whose
`dateTime` carrier must collapse first.

### Character-count reasoning

XML Schema defines `minLength`/`maxLength` over characters, so counting bytes
is sound only when every accepted character is single-byte. Here it provably
is. The union of every character class in the pattern is `[0-9]`, `[a-z]`,
`[a-zA-Z0-9]`, `.`, `_`, and `-`; all are below U+0080. A non-ASCII byte fails
its character-class test and is rejected *before* any length comparison is
reached, so for every value that can possibly be accepted:

```text
UTF-8 byte count == XML Schema character count
```

Rust and C++ therefore count bytes, and Ada counts `Character` elements of a
`String`, which is already a character count. All three agree. This argument is
re-derived per profile and is not assumed for future ones. The corpus includes
non-ASCII cases, including an Arabic-Indic digit lookalike, to keep it honest.

### Why a dedicated validator instead of a regex engine

Task 037 adds **no** XML Schema regular-expression engine, and deliberately
does not approximate the expression with `regex`, `std::regex`, POSIX regex, or
`GNAT.Regpat` — none of those implement XML Schema regex semantics.

Instead the expression is *proved* to reduce to a bounded, deterministic,
single-pass structural decision. It is a plain concatenation of five bounded
pieces using only character classes, explicit `{n,m}` bounds, `?`, and the
escaped literals `\.` and `\-`. It contains no alternation, no backreference,
no unbounded repetition, and no ambiguity: each piece's alphabet is disjoint
from the literal that follows it, so a left-to-right scan never backtracks.
Anchoring is implemented by requiring the scan to finish exactly at
end-of-input.

That equivalence is asserted of this one expression. Any other pattern text, a
second alternative, a second group, or a non-XML-Schema dialect fails closed,
because the proof does not carry over.

The `length` and `pattern` facets are **both** enforced. The pattern's own
bounds are `3+1+1+2 = 7` and `3+1+2+3+2+46 = 57`, coinciding exactly with the
declared facets, so they are formally redundant *for this expression*. They are
still checked explicitly and still required exactly by the classifier: matching
on the pattern alone would silently accept a neighbouring declaration carrying
the same pattern with a different `maxLength`, and then generate a carrier that
ignores that facet. No facet is silently lost.

### Shared String-profile classifier

`crates/codegen-core/src/string_profile.rs` is the String-side analogue of Task
036's temporal classifier, and exists for the same reason: four independent
readings of the same facet set is exactly how three backends drift into
disagreeing about what is renderable while coverage measures a fourth opinion.

```rust
pub fn string_profile(
    kind: PrimitiveKind,
    constraints: &ConstraintSet,
) -> Result<Option<StringProfile>, StringProfileError>;
```

* `Ok(None)` — not a constrained `string`; Task 037 has no opinion and the
  declaration keeps whatever representation it already had.
* `Ok(Some(UciSchemaVersion))` — the implemented profile.
* `Err(UnsupportedConstraints)` — a constrained `string` outside the subset.

Distinguishing `Ok(None)` from `Err(_)` is what keeps ordinary unconstrained
`String` output completely unchanged while every *constrained* neighbour fails
closed. Recognition is purely structural — primitive kind plus effective
`ConstraintSet` shape, including facet values, group count, alternative count,
dialect, and exact expression text. There is no name parameter at all, so no
`if name == "UCI_SchemaVersionStringType"` is possible, no pattern substring
heuristic is used, and no debug-string comparison is made. Every backend and
`CoverageAnalysis` call this one function.

### Generated APIs

Rust — equality **is** derived, unlike the Task 036 carrier:

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SchemaVersion { value: String }

impl SchemaVersion {
    pub fn new(value: &str) -> Option<Self>;
    pub fn as_str(&self) -> &str;
}
```

C++ — construction and observation only, per current backend convention:

```cpp
class SchemaVersion {
public:
    static std::optional<SchemaVersion> create(std::string_view value);
    const std::string& value() const noexcept;
private:
    explicit SchemaVersion(std::string validated);
    std::string value_;
};
```

Ada — an opaque private type completed over the existing owned-string type:

```ada
type SchemaVersion is private;
function Create (Value : String) return SchemaVersion;  --  Constraint_Error
function Value (Item : SchemaVersion) return String;
```

In all three the storage is private and there is no unchecked public
constructor. Compile-time bypass probes prove it: a Rust struct literal or
field read, a C++ direct construction or `value_` read, and an Ada attempt to
name the representation all fail to compile.

### Equality semantics

For `xs:string` the value space *is* the set of lexical forms (§3.2.1 maps each
literal to itself), so two accepted values are equal exactly when their stored
text is equal. Stored-text equality is therefore genuine XML Schema value
equality, and Rust may derive `PartialEq`/`Eq` honestly. This is precisely the
property `dateTime` lacks — distinct legal spellings can denote one instant —
which is why Task 036's carrier deliberately derives none.

No ordering is added in any language: XML Schema defines no order relation on
`string`. C++ invents no comparison operators. Ada's private type inherits
predefined equality, which compares the stored representation; the generated
comment states that this *is* value equality here, in contrast to the Task 036
wording.

Because the carrier supports equality, records containing it **retain** their
derives. A regression asserts the generated `Payload` is
`#[derive(Debug, Clone, PartialEq, Eq)]`, so String-profile consumers do not
lose equality merely because the DateTime carrier has none.

### Ada package body

Task 037 **generalized** the Task 036 body predicate rather than adding a
second body-generation mechanism: a schema needs a `.adb` when it emits *any*
validator-backed carrier. Schemas requiring neither keep their existing
single-`.ads` output exactly, which is asserted for `track.xsd` and
`backend-constrained-floating.xsd`.

Every parser helper is declared inside `Create`'s own declarative part, so no
new package-scope identifier is introduced and nothing new is owed to name
preflight beyond the existing `Create` / `Value` overload model. The single
shared `ada_wrapper_callable_owners` list gained one arm; GNAT 14.2 confirms
that constrained-float `Create`, DateTime `Create`, and String-profile `Create`
coexist, including two `Create (String) return _` overloads that differ *only*
in result type, exercised by a client that calls both.

### Shared conformance corpus

`tests/fixtures/string/schema-version.txt` — **41 cases, 10 valid and 31
invalid**, in the same format as the Task 036 corpus and read by the same
loader, so one escaping bug cannot make the two disagree. All three backends
consume exactly these cases.

Valid cases cover the 7-character minimum, the Sleet baseline `002.5.0`,
two-digit groups, maximum digits, one- and two-letter suffixes, the underscore
tail at minimum and full width, both optional groups together, and the exact
57-character maximum. Invalid cases cover the empty string, the four-group
form, wrong group widths, missing/extra/wrong separators, alphabetic where
numeric is required, an uppercase suffix, an over-long suffix, an empty and an
over-long underscore tail, leading/trailing/interior whitespace, tab, LF, CR,
leading and trailing junk, embedded text, and non-ASCII input.

Every VALID case stores its input unchanged, which is the corpus-level
statement of `whiteSpace = preserve`.

### Selected String blocker inventory

Evidence for **future** tasks. Nothing here is implemented merely because it is
listed. Measured from authoritative UCI 2.5 over the same 60-type
`PositionReport` closure and its security-markings include.

| Declaration | length | minLength | maxLength | whiteSpace | Groups | Alts | Depth | Pattern |
| --- | ---: | ---: | ---: | --- | ---: | ---: | ---: | --- |
| `UCI_SchemaVersionStringType` | — | 7 | 57 | — | 1 | 1 | 1 | `[0-9]{3}\.[0-9]{1,2}(\.[0-9]{1,2})([a-z]{1,2})?(_[a-zA-Z0-9\-]{1,45})?` |
| `UniversallyUniqueIdentifierType` | 36 | — | — | — | 1 | 1 | 1 | `(0{8}(-0{4}){3}-0{12})\|([a-fA-F0-9]{8}-[a-fA-F0-9]{4}-[1-5][a-fA-F0-9]{3}-[89abAB][a-fA-F0-9]{3}-[a-fA-F0-9]{12})` |
| `VisibleString256Type` | — | 1 | 256 | — | 1 | 1 | 1 | `[ -~]{1,256}` |
| `NATO_SpecialWordsType` | — | 6 | 261 | — | 1 | 1 | 1 | `NATO:[a-zA-Z\-_]{1,256}` |
| `WhitespaceVisibleString1024Type` | — | 0 | 1024 | `collapse` | 1 | 1 | 1 | `[&#x20;-&#x7E;\n\r]{0,1024}` |
| `WhitespaceVisibleString4096Type` | — | 0 | 4096 | `collapse` | 1 | 1 | 1 | `[&#x20;-&#x7E;\n\r]{0,4096}` |

`SecurityInformationType` is listed in the task brief but is an
`xs:complexType`, not a constrained String, so it is not a String-profile
candidate at all.

Only the first row is implemented. The UUID type is notable for carrying **two
alternatives** in one group, which is an OR the Task 037 proof does not cover;
the `WhitespaceVisibleString*` pair is notable for an explicit `whiteSpace`
facet, which is a real narrowing for `xs:string`.

### Release-level String evidence, re-measured

Counting `xs:simpleType` declarations that restrict `xs:string` and carry at
least one `xs:pattern`, across the message-definitions root **and** its
security-markings include:

| Facet shape | UCI 2.5 | UCI 2.6 |
| --- | ---: | ---: |
| `length` + `pattern` | 65 | 65 |
| `minLength` + `maxLength` + `pattern` | 55 | 57 |
| `maxLength` + `pattern` | 3 | 3 |
| `minLength` + `maxLength` + `whiteSpace` + `pattern` | 2 | 0 |
| **total patterned String owners** | **125** | **125** |

Earlier notes recorded roughly 128 (2.5) and 127 (2.6); the current measured
figure is **125 in both**, counted as described above. Task 037 implements
exactly **one** of those 125 owners' facet shapes, with one specific
expression — which is precisely why this must not become a generic
constrained-String feature.

### Coverage delta — all twelve cells

Whole-schema declaration coverage, re-run for both releases and both worlds.

| Release / world | Backend | Before | After | Δ |
| --- | --- | ---: | ---: | ---: |
| UCI 2.5 closed | Ada | 5299 / 5557 | **5300 / 5557** | +1 |
| UCI 2.5 closed | Rust | 5376 / 5557 | **5377 / 5557** | +1 |
| UCI 2.5 closed | C++ | 5379 / 5557 | **5380 / 5557** | +1 |
| UCI 2.6 closed | Ada | 5320 / 5570 | **5321 / 5570** | +1 |
| UCI 2.6 closed | Rust | 5398 / 5570 | **5399 / 5570** | +1 |
| UCI 2.6 closed | C++ | 5402 / 5570 | **5403 / 5570** | +1 |
| UCI 2.5 open | Ada | 5214 / 5557 | **5215 / 5557** | +1 |
| UCI 2.5 open | Rust | 5288 / 5557 | **5289 / 5557** | +1 |
| UCI 2.5 open | C++ | 5291 / 5557 | **5292 / 5557** | +1 |
| UCI 2.6 open | Ada | 5235 / 5570 | **5236 / 5570** | +1 |
| UCI 2.6 open | Rust | 5310 / 5570 | **5311 / 5570** | +1 |
| UCI 2.6 open | C++ | 5314 / 5570 | **5315 / 5570** | +1 |

Exactly **one** newly renderable declaration per cell, in every cell:

| QName | Reason | Direct/transitive | Closed/open difference |
| --- | --- | --- | --- |
| `{https://www.vdl.afrl.af.mil/programs/oam}UCI_SchemaVersionStringType` | the one supported facet profile | direct | none |

There are **no transitive gains**. Every consumer of this type in the UCI model
also reaches at least one still-unsupported declaration — most immediately
`UniversallyUniqueIdentifierType` — so no record becomes newly renderable. The
closed/open difference is nil: this is a primitive value declaration, and the
open-world distinction concerns abstract structural values only. That nothing
other than the exact supported profile became renderable was verified rather
than assumed.

A note on how this delta was reached. The first measurement showed **zero**
change in all twelve cells, which was investigated rather than accepted: the
shared classifier accepted the real UCI declaration, but coverage's
`kind_renderable` had never listed `PrimitiveKind::String` as a baseline
primitive kind at all, so a named String declaration was gated behind
`PrimitiveExpansion` before the constraint gate was ever consulted. That arm
now mirrors the temporal one — baseline exactly when the classifier accepts the
profile — and an *unconstrained* named String remains gated exactly as before.

### Selected PositionReport delta

UCI 2.5, `crates/service-contract/tests/fixtures/upstream-minimal.yaml`, one
selected message, 60-declaration closure, closed world.

| Backend | Before | After | First blocker before | First blocker after |
| --- | ---: | ---: | --- | --- |
| Ada | 48/60 | **49/60** | `UCI_SchemaVersionStringType` | `UniversallyUniqueIdentifierType` |
| Rust | 52/60 | **53/60** | `UCI_SchemaVersionStringType` | `UniversallyUniqueIdentifierType` |
| C++ | 52/60 | **53/60** | `UCI_SchemaVersionStringType` | `UniversallyUniqueIdentifierType` |

All three backends advanced by exactly one and now agree on the next measured
blocker. `UniversallyUniqueIdentifierType` is the `length + pattern` family —
correctly still closed as of Task 037. This is the measured result; the next
profile is deliberately **not** implemented here.

> **Task 038 evidence correction.** The row above originally recorded this
> declaration as carrying **two** pattern alternatives, and abbreviated its
> second branch behind an ellipsis. Both readings were wrong, and Task 038's
> evidence gate corrected them against the pinned bytes. The declaration has
> **one** `xs:pattern` facet — hence one IR group holding one expression, with
> the `|` internal to that text — and the general branch constrains the version
> nibble to `[1-5]` and the variant nibble to `[89abAB]`, which the ellipsis had
> hidden. The table row is now the verbatim expression. See
> "Task 038: the UUID String profile" below.

### A pre-existing Ada hazard observed, not introduced

While building the Task 037 Ada fixture, a latent generator hazard surfaced: a
schema namespace whose final segment is `string` produces a package named
`Test.String`, and inside it the identifier `String` denotes that package
rather than `Standard.String`, so the generated
`function Create (Value : String) return …` fails to compile.

This is **pre-existing and shared with Task 036** — its carrier emits the same
`Value : String` profile — and is not caused by this task. It was confirmed
with a minimal hand-written GNAT reproduction independent of this generator.
Task 037 keeps its scope and simply does not name its fixture namespace
`string`; fixing the generator would churn Task 036 output and belongs in its
own task. It is recorded here so it is not later rediscovered as a Task 037
regression.

### Full-schema generation boundary

Unchanged by this task, and not addressed here:

```text
Ada:  Ada name "Range" generates reserved word "Range"
      in the members of AltitudeRangePairType
Rust: Rust name "Type" generates reserved word "type"
      in the members of ConfigurationParameterType
C++:  C++ name "Operator" generates reserved word "operator"
      in the members of ApprovalResponseType
```

No full-UCI generation claim is made.

### Explicit non-goals

| Capability | Status |
| --- | --- |
| named schema-version String profile | **supported** |
| direct field-local constrained String | unsupported |
| named unconstrained `String` | unchanged: plain `String` / `std::string` / `Unbounded_String` |
| `length` + `pattern`, including UUID | unsupported |
| a different pattern with the same bounds | unsupported |
| the same pattern with different bounds | unsupported |
| multiple PatternGroups | unsupported |
| multiple alternatives in one group | unsupported |
| explicit `whiteSpace` profiles | unsupported |
| `NATO_SpecialWordsType`, `VisibleString*` | unsupported |
| generic XML Schema regex translation | not implemented, deliberately |
| String ordering or collation | not implemented |
| XML or JSON codecs | not implemented |

No new `FeatureFamily` was introduced: Task 037 is a concrete baseline
implementation inside the existing conceptual constrained-simple-type family,
and all existing feature-combination monotonicity tests remain green. Task 036
DateTime behaviour and Task 035 occurrence behaviour are unchanged. No
third-party runtime dependency was added in any language.

## Task 038: the UUID String profile

Task 038 adds the **second** supported constrained-`string` profile:
`UniversallyUniqueIdentifierType`. It extends the Task 037 architecture rather
than duplicating it — the shared classifier gained a variant, and coverage, the
three backends, the Ada body predicate, and the Ada callable model all picked
the new profile up without change.

### Authoritative evidence

Both pinned releases are **byte-identical** for this declaration after
end-of-line normalization (2.5 uses CRLF, 2.6 LF):

* UCI 2.5 `093610b7753944059360d3236770ab446d039556` —
  `UCI_MessageDefinitions_v2_5_0.xsd:145719`
* UCI 2.6 `78eb61b6112c8bffa40820c33124b57787fc5bd9` —
  `UCI_MessageDefinitions_v2_6_0.xsd:145967`

```xml
<xs:simpleType name="UniversallyUniqueIdentifierType" uci:version="000.001.000.000">
  <xs:annotation>
    <xs:documentation>A UUID is a 128-bit number (32 hexadecimal digits, 16 bytes) that is conformant to any version of variant 1 or nil UUID, as described in IETF RFC 4122.</xs:documentation>
  </xs:annotation>
  <xs:restriction base="xs:string">
    <xs:length value="36"/>
    <xs:pattern value="(0{8}(-0{4}){3}-0{12})|([a-fA-F0-9]{8}-[a-fA-F0-9]{4}-[1-5][a-fA-F0-9]{3}-[89abAB][a-fA-F0-9]{3}-[a-fA-F0-9]{12})"/>
  </xs:restriction>
</xs:simpleType>
```

Normalized IR, read from the frontend for both releases:

| Property | Value |
| --- | --- |
| primitive | `String` |
| immediate + ultimate base | `xs:string` (restriction depth 1) |
| `length` | `36` |
| `minLength` / `maxLength` | absent |
| explicit `whiteSpace` | absent (so intrinsic `preserve`) |
| numeric facets | absent |
| `PatternGroup`s | **1** |
| `PatternExpression`s in that group | **1** |
| dialect | `XmlSchema` |

### One facet, one expression, internal alternation

The schema contains exactly one `<xs:pattern>` element, so the IR holds one
group with one expression and the `|` is **internal to that expression's text**.

This distinction is load-bearing. Task 016's IR represents several `xs:pattern`
facets at one restriction level as several alternatives in one group, and that
machinery is *not* involved here. A classifier written to require two IR
alternatives would reject the authoritative declaration outright and support
nothing. The classifier therefore requires exactly one group holding exactly one
expression, compared verbatim.

### The two internal branches, and why neither is redundant

```text
A   0{8}(-0{4}){3}-0{12}
    the nil UUID 00000000-0000-0000-0000-000000000000, and nothing else

B   [a-fA-F0-9]{8} - [a-fA-F0-9]{4} - [1-5][a-fA-F0-9]{3}
                   - [89abAB][a-fA-F0-9]{3} - [a-fA-F0-9]{12}
```

Branch A denotes exactly one string: every atom is the literal `0` under a fixed
quantifier.

It is tempting to conclude branch A is redundant, since `0` is a hexadecimal
digit. **It is not.** Branch B constrains two positions beyond plain hexadecimal
— the version nibble must be `[1-5]` and the variant nibble `[89abAB]` — and the
nil UUID has `0` in both. Branch B therefore *rejects* the nil UUID, and branch A
contributes exactly one otherwise-unreachable value. The union is implemented
explicitly:

* collapsing to branch B alone would reject the nil UUID;
* collapsing to a general 8-4-4-4-12 hexadecimal shape would accept values the
  schema forbids, including every all-`f` UUID.

### Version and variant are schema constraints, not imported RFC policy

`[1-5]` and `[89abAB]` are present in the authoritative pattern text, matching
the declaration's own documentation ("conformant to any version of variant 1 or
nil UUID"). Enforcing them implements the schema; it does not import RFC rules.

Nothing beyond them is imposed. There is no RFC 9562 v6/v7/v8 policy, no
canonical-lowercase rule, no URN or brace syntax, no nil prohibition, and no
conversion to a 128-bit integer. Concretely, these are **invalid** under the
authoritative profile even though a general hexadecimal reading would accept
them:

```text
ffffffff-ffff-ffff-ffff-ffffffffffff    version f, variant f
FFFFFFFF-FFFF-FFFF-FFFF-FFFFFFFFFFFF    version F, variant F
123e4567-e89b-12d3-c456-426614174000    variant c is hexadecimal but not [89abAB]
```

### The runtime validator

No regular-expression engine is introduced in any language. Both branches are
fixed-width, so every position's character class is determined by its index
alone and the decision is a bounded positional test with no backtracking:

```text
length must be 36
the exact nil literal is accepted outright                     (branch A)
otherwise:                                                     (branch B)
  indexes 8, 13, 18, 23 must be '-'
  every other index must be [0-9a-fA-F]
  index 14 must be [1-5]        (version nibble)
  index 19 must be [89abAB]     (variant nibble)
```

The group widths `8-4-4-4-12` place the third group at indexes 14..17 and the
fourth at 19..22, which is what fixes the version index at 14 and the variant
index at 19. XML Schema patterns are anchored (§4.3.4.3), enforced directly by
the fixed length requirement.

`length` is enforced explicitly even though both branches are 36 characters
wide, and the classifier requires `length = Some(36)` exactly, so a neighbouring
declaration carrying the same pattern under a different `length` cannot be
mistaken for this profile.

### Character counting, case, equality, and whitespace

The accepted alphabet is exactly `0-9`, `a-f`, `A-F`, and `-`, all ASCII.
Every accepted value is therefore pure ASCII and its UTF-8 byte count equals its
XSD character count, which licenses the Rust and C++ validators to use byte
length for a facet XSD defines over characters; Ada's `String'Length` is a
character count already. The corpus carries non-ASCII negative controls to keep
this assumption explicit. The proof is profile-specific and is re-derived, not
inherited, for any future profile.

`[a-fA-F0-9]` admits both letter cases and `[89abAB]` admits `a`/`b` and `A`/`B`,
so accepted values are stored **exactly as supplied**. This remains an
`xs:string` carrier, whose value equality is equality of the stored text, so two
otherwise-valid UUIDs differing only in letter case are *distinct* values. No
case-insensitive comparison is introduced anywhere, and no ordering is claimed.

There is no explicit `whiteSpace` facet, so `xs:string`'s intrinsic `preserve`
applies and normalization is the identity. No whitespace character appears in
either branch, so a value carrying leading, trailing, or interior whitespace is
rejected rather than repaired.

### Generated APIs

Identical in shape to Task 037's carriers.

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Uuid { value: String }
impl Uuid {
    pub fn new(value: &str) -> Option<Self>;
    pub fn as_str(&self) -> &str;
}
```

```cpp
class Uuid {
public:
    static std::optional<Uuid> create(std::string_view value);
    const std::string& value() const noexcept;
private:
    explicit Uuid(std::string validated);
    std::string value_;
};
```

```ada
type Uuid is private;
function Create (Value : String) return Uuid;   --  Constraint_Error if invalid
function Value (Item : Uuid) return String;
```

Storage is private in all three, with no unchecked construction; compile-fail
bypass probes assert this rather than arguing it. Rust derives `PartialEq`/`Eq`
(correct for `xs:string`) but not `Ord`; C++ keeps the existing convention of
inventing no comparison operator; Ada's predefined `"="` compares the stored
text, which *is* XML Schema value equality here.

Composition is automatic: a required `Uuid` field, an optional `Uuid 0..1`
occurrence, and a record mixing both String profiles all use the existing Task
034 machinery with no UUID-specific path. A record holding only
equality-capable members retains its Rust derives.

### Conformance corpus

`tests/fixtures/string/uuid.txt`, 78 cases (12 valid, 66 invalid), consumed by
all three backends through the shared Task 036/037 loader, so the languages are
proven to agree rather than each passing a curated subset. Valid cases cover the
nil UUID, versions 1-5, variants `8`/`9`/`a`/`b`/`A`/`B`, lower/upper/mixed case,
and `f`/`F` saturation of the unconstrained positions. Invalid cases cover every
illegal version and variant nibble, both all-`f` controls, nil-lookalikes,
shape and width failures, braces, a `urn:uuid` prefix, non-hexadecimal
characters, all five whitespace positions, and non-ASCII lookalikes.

As secondary evidence, the structural validator was compared against an
independent implementation of the two exact internal branches over **800,000
randomized inputs with 0 mismatches**. That comparison is evidence only — the
pinned XSD remains authoritative, and no regex semantics are part of runtime
behaviour.

### Coverage delta

Exactly **one** newly renderable declaration per cell, in all twelve cells:

| Cell | Before | After |
| --- | ---: | ---: |
| 2.5 closed, Ada / Rust / C++ | 5300 / 5377 / 5380 | **5301 / 5378 / 5381** |
| 2.5 open, Ada / Rust / C++ | 5215 / 5289 / 5292 | **5216 / 5290 / 5293** |
| 2.6 closed, Ada / Rust / C++ | 5321 / 5399 / 5403 | **5322 / 5400 / 5404** |
| 2.6 open, Ada / Rust / C++ | 5236 / 5311 / 5315 | **5237 / 5312 / 5316** |

| QName | Reason | Direct/transitive | Closed/open difference |
| --- | --- | --- | --- |
| `{https://www.vdl.afrl.af.mil/programs/oam}UniversallyUniqueIdentifierType` | the Task 038 facet profile | direct | none |

There are **no transitive gains**: every consumer of this type also reaches at
least one still-unsupported declaration, so no record becomes newly renderable.
The closed/open difference is nil, as this is a primitive value declaration.
That nothing outside the exact profile became renderable was verified, not
assumed. No new `FeatureFamily` was introduced, and coverage itself was not
modified — it already consults `string_profile`.

### Selected PositionReport delta

UCI 2.5, one selected message, 60-declaration closure, closed world.

| Backend | Before | After | First blocker before | First blocker after |
| --- | ---: | ---: | --- | --- |
| Ada | 49/60 | **50/60** | `UniversallyUniqueIdentifierType` | `VisibleString256Type` |
| Rust | 53/60 | **54/60** | `UniversallyUniqueIdentifierType` | `VisibleString256Type` |
| C++ | 53/60 | **54/60** | `UniversallyUniqueIdentifierType` | `VisibleString256Type` |

All three advanced by exactly one and agree on the next measured blocker.
`VisibleString256Type` is a `minLength`/`maxLength` + printable-range pattern
profile — a third, different constrained-String family. It is the measured
result, and is deliberately **not** implemented here. `PositionReport` remains
NOT READY in every backend.

### Full-schema boundary

Unchanged and unrelated to this task: Ada `AltitudeRangePairType / Range`, Rust
`ConfigurationParameterType / Type`, C++ `ApprovalResponseType / Operator`. No
full-UCI generation claim is made.

### Scope

| Concern | Status |
| --- | --- |
| the exact UUID profile above | **supported (Task 038)** |
| the schema-version profile | **supported (Task 037), unchanged** |
| `VisibleString256Type`, `NATO_SpecialWordsType`, `WhitespaceVisibleString*` | unsupported |
| same pattern with a different or absent `length` | unsupported |
| same pattern split into two IR alternatives | unsupported |
| general 8-4-4-4-12 hexadecimal without version/variant classes | unsupported |
| a second `PatternGroup`, or an explicit `whiteSpace` | unsupported |
| generic `length + pattern` String support | not implemented, deliberately |
| generic XML Schema regex translation | not implemented, deliberately |
| UUID parsing, arithmetic, canonicalization, RFC rules beyond the XSD | not implemented, deliberately |
| String ordering or collation | not implemented |

Task 036 temporal behaviour, Task 035 occurrence behaviour, and Task 037
schema-version behaviour are all unchanged. No third-party runtime dependency
was added in any language, and the pre-existing Ada namespace-ending-in-`String`
hazard is untouched and still open.

## Task 039: the visible-ASCII String family

Task 039 adds the **third** supported constrained-`string` profile, and the
first *parameterized* one: `StringProfile::VisibleAscii { min_length,
max_length }`.

The parameterization is bounded by evidence. The implementation is
parameterized over the **ten bound pairs observed in the authoritative UCI
2.5/2.6 family**, and the shape is *not* an open-ended generic visible-ASCII
datatype facility. The thirteen measured declarations collapse onto these ten
unique effective pairs, which are the entire accepted parameter domain
(`UCI_VISIBLE_ASCII_BOUNDS` in `codegen-core/src/string_profile.rs`):

```text
1..20   1..32   1..64   1..81   1..128
1..256  1..480  1..512  1..1024  2..4
```

An arbitrary `[ -~]{M,N}` whose `minLength`/`maxLength` facets agree with its
own quantifier is therefore **still unsupported** unless `(M, N)` is one of the
ten. A synthetic `3..17`, and a synthetic `1..18446744073709551615`, both fail
closed with `UnsupportedConstraints`. Three reasons:

* **no authoritative evidence** — a bound pair no pinned release contains has
  no measured declaration and no conformance corpus behind it;
* **baseline support must be compiler-backed** — calling a declaration
  baseline-renderable asserts that the *generated* code compiles in every
  claimed backend, and only the ten accepted pairs are exercised under GNAT,
  rustc, and g++;
* **unrestricted `u64` bounds would exceed backend literal and host-size
  assumptions** — the bounds are emitted as length constants (C++
  `static constexpr std::size_t kMaxLength`, Rust `const MAX_LENGTH: usize`,
  Ada `Max_Length : constant`), and a `u64::MAX` literal is not established as
  portable under `-std=c++17 -Wall -Wextra -pedantic-errors`.

Membership is semantic, never nominal: a differently named declaration carrying
one of the ten exact profiles classifies, and a UCI-looking local name carrying
unobserved bounds does not. Widening the domain is a future task with its own
evidence and its own compile-backed tests.

### Authoritative evidence

The selected blocker, `VisibleString256Type`, read from the pinned release
bytes. UCI 2.5 `093610b7753944059360d3236770ab446d039556`
`UCI_MessageDefinitions_v2_5_0.xsd:146232`, and UCI 2.6
`78eb61b6112c8bffa40820c33124b57787fc5bd9`
`UCI_MessageDefinitions_v2_6_0.xsd:146583`. The two are **byte-identical after
end-of-line normalization** (both fragments hash to
`1b034c43036649d96f87eae5253136c13cd37315935521690462e3fefa541278`):

```xml
<xs:simpleType name="VisibleString256Type" uci:version="000.001.000.000">
  <xs:annotation>
    <xs:documentation>A string representing up to 256 characters in length, restricted to visible characters (0x20-0x7E).</xs:documentation>
  </xs:annotation>
  <xs:restriction base="xs:string">
    <xs:minLength value="1"/>
    <xs:maxLength value="256"/>
    <xs:pattern value="[&#x20;-&#x7E;]{1,256}"/>
  </xs:restriction>
</xs:simpleType>
```

The immediate base is `xs:string`, which is also the ultimate primitive, so the
restriction-chain depth is 1 and the effective constraint set is this single
step. The normalized IR, read by running the frontend over both pinned roots,
is:

```text
PrimitiveKind::String
length     = None      minLength = Some(1)     maxLength = Some(256)
whiteSpace = None      (explicit; xs:string's intrinsic `preserve` applies)
PatternGroups = 1      PatternExpressions = 1  PatternDialect = XmlSchema
expression = [ -~]{1,256}
```

#### The source text uses character references

The XSD spells the class `[&#x20;-&#x7E;]`, not `[ -~]`. Those are XML
character references, expanded by the parser before any schema processing, so
the IR expression is the five characters `[ -~]` carrying a literal SPACE and a
literal TILDE. Reading the raw file and reading the IR therefore disagree
textually while agreeing semantically. The classifier compares against the IR
form, built by `visible_ascii_pattern`, and this was confirmed against the
frontend's own output rather than assumed.

### Family inventory

Reading the XSD twice is not independent evidence, so the inventory was taken
from the pinned bytes *and* from the normalized IR. Both releases agree
exactly. Thirteen declarations have effective constraints of the family shape —
`minLength = M`, `maxLength = N`, no `length`, no explicit `whiteSpace`, one
XML-Schema pattern `[ -~]{M,N}` — and in every one the bounds and the
quantifier agree:

| QName (both releases) | M | N | Depth | Expression |
| --- | ---: | ---: | ---: | --- |
| `AttributedURI_Type` | 1 | 256 | 1 | `[ -~]{1,256}` |
| `MIME_Type` | 1 | 256 | 1 | `[ -~]{1,256}` |
| `MissionCategoryType` | 1 | 32 | 2 | `[ -~]{1,32}` |
| `VisibleString20Type` | 1 | 20 | 1 | `[ -~]{1,20}` |
| `VisibleString2_4Type` | 2 | 4 | 1 | `[ -~]{2,4}` |
| `VisibleString32Type` | 1 | 32 | 1 | `[ -~]{1,32}` |
| `VisibleString64Type` | 1 | 64 | 1 | `[ -~]{1,64}` |
| `VisibleString81Type` | 1 | 81 | 1 | `[ -~]{1,81}` |
| `VisibleString128Type` | 1 | 128 | 1 | `[ -~]{1,128}` |
| `VisibleString256Type` | 1 | 256 | 1 | `[ -~]{1,256}` |
| `VisibleString480Type` | 1 | 480 | 1 | `[ -~]{1,480}` |
| `VisibleString512Type` | 1 | 512 | 1 | `[ -~]{1,512}` |
| `VisibleString1024Type` | 1 | 1024 | 1 | `[ -~]{1,1024}` |

Two observations decided the design:

* `AttributedURI_Type` and `MIME_Type` are members whose names contain no
  "VisibleString" at all, and `MissionCategoryType` is a depth-2 restriction of
  `VisibleString32Type` that adds no facets and appears only in the IR.
  Name-based inference would have been wrong in three different ways;
* the members differ from one another in *nothing but* the bound pair.

This is the "clear family differing only in bounds" case, so the profile is
**parameterized** rather than fixed. It is not generic `minLength + maxLength +
pattern` support: the expected expression is *derived from the facets*, so a
declaration matches only if its own quantifier agrees with its own bounds. A
same-pattern declaration under `maxLength = 128`, or these bounds under a
different pattern, fails closed.

### XSD semantics

**Character range.** `[ -~]` is one inclusive XML Schema character range from
U+0020 SPACE to U+007E TILDE. It admits SPACE, every ASCII punctuation mark, the
digits, both letter cases, and `{ | } ~`. It excludes TAB (U+0009), LF
(U+000A), CR (U+000D), every other C0 control, DEL (U+007F), and every
non-ASCII character. All three validators test that ordinal interval with
explicit comparisons. No locale-sensitive classifier is used: `std::isprint` and
the rest of `<cctype>` vary by locale, Rust's `is_ascii_graphic` would wrongly
*exclude* SPACE, and `Ada.Characters.Handling` is likewise avoided. The C++
validator casts to `unsigned char` first, so a non-ASCII byte cannot compare as
negative under an implementation where plain `char` is signed.

**`whiteSpace`, and why ordinary SPACE is valid.** The base `xs:string` has
intrinsic `whiteSpace = preserve`, which is not fixed but which this restriction
does not override, so normalization is the identity. SPACE is itself a member of
the class. Values such as `" hello"`, `"hello "`, and `"   "` are therefore
**valid** whenever their lengths fit, and are stored exactly as supplied.
Nothing is trimmed or collapsed. This is the opposite of Tasks 037 and 038,
whose alphabets excluded whitespace entirely, so their corpus expectations were
deliberately not reused. TAB, LF, and CR are different characters and still fail.

**Length units.** XML Schema measures `minLength`/`maxLength` in *characters*.
Every character this profile accepts is at most U+007E, hence single-byte in
UTF-8, so for this profile the UTF-8 byte count equals the XSD character count.
That is what licenses the Rust and C++ validators to use `len()`/`size()`; Ada's
`String'Length` is a character count already. A multi-byte character contains
bytes outside the range and fails the class test, so it can never reach a length
comparison as an accepted value. The argument is re-derived per profile from its
own alphabet and is **not** generalized.

**Both facets and pattern are enforced.** For every member the quantifier
repeats the bounds, so they are formally redundant. All three are still required
by the classifier and checked by the generated validators; no facet is silently
lost.

### No regular-expression engine

The authoritative expression is one character class under one bounded
quantifier, so membership is a length test plus an independent per-character
range test, with no backtracking. XML Schema patterns are anchored, which
testing *every* character enforces directly. No regex crate, `<regex>`,
`GNAT.Regpat`, PCRE, or RE2 was added.

### Generated API

| Language | API |
| --- | --- |
| Rust | `pub struct {T} { value: String }`, `new(&str) -> Option<Self>`, `as_str(&self) -> &str`, deriving `Clone, Debug, PartialEq, Eq` |
| C++ | `static std::optional<{T}> create(std::string_view)`, `const std::string& value() const noexcept`, private constructor and storage |
| Ada | `type {T} is private;`, `Create (Value : String) return {T}` raising `Constraint_Error`, `Value (Item : {T}) return String`, `Unbounded_String` in the private part |

Each carrier emits its **own** bounds as constants, so a 1..32 member cannot be
widened to 1..256 by a shared constant. Privacy is proven by compiler probes in
all three languages: a Rust struct literal or field read, a C++ private
constructor or storage access, and an Ada aggregate or component read must all
fail to compile.

Equality follows Tasks 037/038: for `xs:string` the value space *is* the set of
lexical forms, so stored-text equality is genuine XML Schema value equality.
Rust derives `PartialEq`/`Eq`; Ada's predefined `"="` compares the stored text;
C++ invents no comparison operators. No ordering is claimed in any language.
Because nothing is trimmed, `"abc"` and `"abc "` are *distinct* values, and case
remains significant.

Composition reuses the Task 034 machinery with no visible-string-specific path:
a required field uses the carrier directly, an optional one becomes
`Option<T>` / `std::optional<T>` / the existing `_Optional` wrapper. A record
holding only equality-capable members keeps its Rust `PartialEq`/`Eq`.

In Ada the new carrier participates automatically in `Create` / `Value` overload
handling through `string_profile`, with no profile-specific naming logic and no
new package-body predicate. A fixture package carrying six carriers spanning all
three String profiles compiles under GNAT, and several
`Create (Value : String) return <different type>` functions coexist because each
call site supplies a typed expected result.

### Shared corpus

`tests/fixtures/string/visible-ascii.txt` — 29 valid and 31 invalid cases,
executed by all three backends through the shared loader. It pins single SPACE,
`!`, `~`, digits, both letter cases, punctuation, interior spaces, leading and
trailing and all-space values, the 1-character minimum, the 256-character
maximum, 256 spaces, and all 95 class members in one value; and rejects the
empty string, 257 characters, TAB/LF/CR alone and embedded and trailing, NUL and
other controls, and a range of non-ASCII characters including a Unicode digit
lookalike and an emoji. The four ordinal boundaries are pinned explicitly:

```text
U+001F -> invalid    U+0020 -> valid
U+007E -> valid      U+007F -> invalid
```

which is what distinguishes the exact `[ -~]` interval from an informal
"printable ASCII" guess. The corpus loader gained a `\u{HEX}` escape for this,
since those code points cannot appear literally in a text file unambiguously;
the Task 036--038 corpora and their loader behaviour are untouched.

### Coverage delta

| Cell | Before | After |
| --- | ---: | ---: |
| 2.5 closed, Ada / Rust / C++ | 5301 / 5378 / 5381 | **5314 / 5391 / 5394** |
| 2.5 open, Ada / Rust / C++ | 5216 / 5290 / 5293 | **5229 / 5303 / 5306** |
| 2.6 closed, Ada / Rust / C++ | 5322 / 5400 / 5404 | **5335 / 5413 / 5417** |
| 2.6 open, Ada / Rust / C++ | 5237 / 5312 / 5316 | **5250 / 5325 / 5329** |

Every cell gains exactly **+13**, one per family member, in all three backends
and both releases. The gain is larger than one because the evidence gate proved
a genuine parameterized family, not because the profile was widened; the totals
match the inventory exactly, so there is no unexplained movement.

| QName | Effective profile | Direct/transitive | Why renderable |
| --- | --- | --- | --- |
| `AttributedURI_Type` | `VisibleAscii { 1, 256 }` | direct | exact family facets |
| `MIME_Type` | `VisibleAscii { 1, 256 }` | direct | exact family facets |
| `MissionCategoryType` | `VisibleAscii { 1, 32 }` | transitive (depth 2) | chain resolves to family facets |
| `VisibleString20Type` | `VisibleAscii { 1, 20 }` | direct | exact family facets |
| `VisibleString2_4Type` | `VisibleAscii { 2, 4 }` | direct | exact family facets |
| `VisibleString32Type` | `VisibleAscii { 1, 32 }` | direct | exact family facets |
| `VisibleString64Type` | `VisibleAscii { 1, 64 }` | direct | exact family facets |
| `VisibleString81Type` | `VisibleAscii { 1, 81 }` | direct | exact family facets |
| `VisibleString128Type` | `VisibleAscii { 1, 128 }` | direct | exact family facets |
| `VisibleString256Type` | `VisibleAscii { 1, 256 }` | direct | the selected blocker |
| `VisibleString480Type` | `VisibleAscii { 1, 480 }` | direct | exact family facets |
| `VisibleString512Type` | `VisibleAscii { 1, 512 }` | direct | exact family facets |
| `VisibleString1024Type` | `VisibleAscii { 1, 1024 }` | direct | exact family facets |

The closed/open difference is nil for all thirteen, as these are primitive value
declarations. There are **no further transitive gains**: every other consumer
also reaches at least one still-unsupported declaration, so no record becomes
newly renderable. No new `FeatureFamily` was introduced, and coverage itself was
not modified — it already consults `string_profile`.

### Selected PositionReport delta

UCI 2.5, one selected message, 60-declaration closure, closed world.

| Backend | Before | After | First blocker before | First blocker after |
| --- | ---: | ---: | --- | --- |
| Ada | 50/60 | **51/60** | `VisibleString256Type` | `SecurityInformationType` |
| Rust | 54/60 | **55/60** | `VisibleString256Type` | `SecurityInformationType` |
| C++ | 54/60 | **55/60** | `VisibleString256Type` | `SecurityInformationType` |

All three advanced by exactly one and agree on the next measured blocker.
`SecurityInformationType` is a *record*, and its own remaining blockers are
`NATO_SpecialWordsType` and `WhitespaceVisibleString1024Type` /
`WhitespaceVisibleString4096Type` — precisely the neighbouring String profiles
Task 039 deliberately leaves closed — plus several enumerations. Ada
additionally reports its pre-existing identifier boundary on
`DeclassExceptionEnum` (`"25X1"`). None of these is implemented here.
`PositionReport` remains NOT READY in every backend.

### Full-schema boundary

Unchanged and unrelated to this task: Ada `AltitudeRangePairType / Range`, Rust
`ConfigurationParameterType / Type`, C++ `ApprovalResponseType / Operator`. No
full-UCI generation claim is made.

### Scope

| Concern | Status |
| --- | --- |
| the visible-ASCII family above | **supported (Task 039)** |
| the schema-version profile | **supported (Task 037), unchanged** |
| the UUID profile | **supported (Task 038), unchanged** |
| same pattern under different bounds | unsupported |
| same bounds under a different pattern | unsupported |
| fixed-`length` `[ -~]{N}` (`VisibleStringLength*`, `NITF_*`) | unsupported |
| `QueryString4096Type`, whose class also admits LF and CR | unsupported |
| `WhitespaceVisibleString1024Type` / `WhitespaceVisibleString4096Type` | unsupported: explicit `whiteSpace = collapse` needs its own normalization analysis |
| `NATO_SpecialWordsType` (`NATO:[a-zA-Z\-_]{1,256}`) | unsupported: a distinct lexical profile, ASCII-only notwithstanding |
| a second `PatternGroup`, an extra expression, or an explicit `whiteSpace` | unsupported |
| an agreeing `[ -~]{M,N}` whose `(M, N)` is not one of the ten observed pairs (e.g. `3..17`) | unsupported: no authoritative evidence |
| an agreeing `[ -~]{M,N}` with huge bounds (e.g. `1..18446744073709551615`) | unsupported: baseline support must be compiler-backed |
| generic `minLength + maxLength + pattern` String support | not implemented, deliberately |
| generic XML Schema regex translation | not implemented, deliberately |
| String ordering or collation | not implemented |

Task 036 temporal behaviour, Task 035 occurrence behaviour, and the Task 037 and
038 String profiles are all unchanged, and their shared corpora still pass. No
third-party runtime dependency was added in any language, and the pre-existing
Ada namespace-ending-in-`String` hazard is untouched and still open.

## Task 040: the validated lexical-carrier lifecycle

Corrective. Task 040 changed **lifecycle behavior** of the carriers Tasks
036--039 introduced; it changed no capability. The classifier, the effective
facets, the accepted lexical spaces, and the normalization semantics are all
untouched, coverage moved by zero in every measured cell, and the Task 039
observed-bound domain was not widened.

Two gaps were closed, and both affected **all four** existing families — the
UCI schema-version profile, the UUID profile, every supported visible-ASCII
profile, and the named Zulu DateTime profile. Rust was affected by neither and
its generated output is byte-identical.

### Ada: unchecked default initialization

The private completion's `Unbounded_String` component carried the predefined
null-string default, so an ordinary default declaration produced a usable,
fully unchecked carrier whose `Value` returned the empty string — a value every
one of these profiles rejects.

The component default is now an explicitly failing `raise` expression:

```ada
type Uuid is record
   Text : Standard.Ada.Strings.Unbounded.Unbounded_String :=
     raise Standard.Program_Error
       with "Uuid requires initialization from Create";
end record;
```

Enforcement is part of Ada's initialization semantics, so it does **not** depend
on `-gnata`, on `Assertion_Policy`, on a caller-side precondition, or on a type
invariant a client setting could disable — the probes are built without `-gnata`
and repeated under `Assertion_Policy (Ignore)`. `Create` already returned an
explicit named aggregate after validation, so legitimate construction never
evaluates the default, and the public `Create` / `Value` signatures are
unchanged. No new package-scope name is introduced, so the shared
generated-name model is unaffected.

An **absent** optional wrapper still default-creates normally; a *constrained
present* wrapper, `..._Optional (Is_Present => True)`, now raises, which is the
intended result.

### C++: implicit destructive move

The classes declared no special members, so the compiler implicitly declared
move operations that transferred `value_`'s buffer and left the **still-live
source** holding a representation its own `create` rejects — which could then be
copied, propagating it.

Each class now declares its copy operations:

```cpp
Uuid(const Uuid&) = default;
Uuid& operator=(const Uuid&) = default;
```

This suppresses the *implicit declaration* of the move operations (C++17
`[class.copy.ctor]/8`, `[class.copy.assign]/4`), so rvalue operations select the
copy operations: the destination gets the right value and the source keeps its
exact spelling. Nothing is `= delete`d, because deleted overloads would still
participate in overload resolution and would break rvalue construction,
`std::optional`, and generated container composition.

`std::is_move_constructible` remains **true** — satisfied by the copy
constructor — so that trait is not evidence a destructive move exists. The
regressions assert observed values, not traits.

Tradeoff: a `std::move` on these carriers copies the string and may allocate,
and the operations are correctly **not** `noexcept` because a copy can throw.
The validated-value invariant is prioritized over destructive string-move
optimization.

Preserved: private unchecked construction, private storage, factory validation
and rejection, exact lexical spelling for the preserving profiles, the existing
DateTime normalization, and the read-only accessor. No new public base class and
no broad template rewrite.

### Correcting an earlier over-broad claim

Prior notes stated, in effect, that privacy alone proves there is no unchecked
construction path. That is too broad, and Task 040 is the counterexample. The
earlier records are preserved as historical evidence; the current guarantee is:

- privacy still prevents a client from **naming** the representation, so no
  client aggregate, component read, conversion, or raw `value_` read compiles —
  and those compile-fail probes are retained and required to fail *for
  privacy*;
- privacy never prevented the **language** from default-initializing the Ada
  representation or from synthesizing C++ move operations. Those are what Task
  040 closes.

Neither backend claims protection against arbitrary unchecked or foreign-memory
manipulation.

### Evidence

GNAT 14.2.0; `g++` (Debian 14.2.0-19) 14.2.0 under
`-std=c++17 -Wall -Wextra -pedantic-errors`; rustc/cargo 1.98.1. Backend probes
live in `crates/backend-ada/tests/carrier_lifecycle.rs` and
`crates/backend-cpp/tests/carrier_lifecycle.rs`; contract-selected integration
probes are `task040_selected_ada_output_requires_explicit_initialization` and
`task040_selected_cpp_output_preserves_the_validated_value`. The visible-ASCII
bounds exercised include the **2..4** profile, whose minimum is above one
character, so no accidental valid-default assumption can hide behind a
1-character minimum. The shared corpora are unchanged.

Measured UCI 2.6 (`78eb61b6112c8bffa40820c33124b57787fc5bd9`) declarations are
identical before and after — closed `5335 / 5413 / 5417` and open
`5250 / 5325 / 5329` for Ada / Rust / C++ — matching the Task 039 record
exactly.

The pinned UCI 2.5 set (`093610b7753944059360d3236770ab446d039556`) was **not**
obtainable when the first revision of this section was written, and its six
cells were recorded as missing evidence. It **was** obtainable on the
corrective pass and is now freshly measured: closed `5314 / 5391 / 5394` and
open `5229 / 5303 / 5306` for Ada / Rust / C++, all deltas zero, and the
`PositionReport` recheck reproduces at Ada 51/60, Rust 55/60, C++ 55/60 with
first blocker `SecurityInformationType`. The missing-measurement limitation is
therefore withdrawn rather than carried forward.

### Corrective: repeated storage of validated carriers

The rejecting Ada component default above fixed unchecked scalar
initialization, but it interacted badly with **repeated storage**, which
default-initializes physical capacity before any live element is assigned. The
earlier blanket claim that all legitimate Ada construction paths were
unaffected was therefore too broad and is corrected.

Two paths regressed, and both are fixed:

- an **unbounded** field instantiated the *definite* `Ada.Containers.Vectors`,
  which default-initializes the replacement array it allocates when growing, so
  appending valid carriers raised `Program_Error` from inside the container. It
  now instantiates `Ada.Containers.Indefinite_Vectors` in the **private part**
  behind an opaque sequence type with five package-level operations
  (`Length`, `Append`, `Element`, `Clear`, `Reserve_Capacity`);
- a **bounded** field sized its `Items` array to `maxOccurs` over the element
  type, so a valid *empty* `0..N` sequence default-initialized `N` carriers.
  Each physical slot is now a discriminated record whose unused variant has no
  payload component. The bounded array, the allocation-free shape, and the
  `minOccurs .. maxOccurs` logical length are all preserved.

A separate, **pre-existing** defect surfaced during reproduction and is fixed
by the same change: the visible-part instantiation did not compile at all when
the element was a generated `private` type (`premature use of private type`).
That reproduces on original `main` for a Task 033 constrained-float element and
is not a Task 040 regression.

Storage is chosen from semantic information only — occurrence shape and element
structure — never from a declaration spelling, and the policy is uniform across
direct carriers, composed records, and ordinary scalars. Tradeoff: an
indefinite vector heap-allocates per element, accepted deliberately so that
live elements are constructed only from supplied values. Rust and C++ generated
output is byte-identical before and after.

See `docs/task-040-validated-carrier-lifecycle.md` §10.
