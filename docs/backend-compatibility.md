# Backend compatibility and Task 019 lowering

Tested: **2026-09-18**, baseline `1fa2ab4d380158728d383a5e4fbaebea6a2006b1`.

This document records backend status separately from frontend compatibility.
The authoritative schemas remain external and are not vendored. The frontend
fully normalizes the UCI 2.5 and UCI 2.6 roots; the three language backends do
not yet generate those complete models.

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

Published Task 017 authoritative evidence remains: UCI 2.5 has 420 Choice
declarations (419 standalone, 1 inherited), UCI 2.6 has 424 (423 standalone,
1 inherited); duplicate alternative names are zero; 47 Choices in each release
have duplicate payload types; 113 additional alternatives share a payload type;
repeated alternatives are 93/94; nillable alternatives are zero; and the sole
inherited Choice is `QueryType`. The authoritative roots were unavailable in
this checkout, so no post-change UCI generation, cardinality inventory, or
coverage delta is fabricated.

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
