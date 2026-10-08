# Task 069 — deterministic semantic schema diff

Immutable starting `origin/main` (BEFORE):
`42a8f369fe4a7f733fe1b0f802f575c3ec4a7fb0`.
Work is isolated on `feature/069-semantic-schema-diff` in
`/home/zboll/git/ams-gra-codegen-oms-task069`; build storage and evidence are
under `/tmp/task069`. No sibling work is imported.

## Command and API

```sh
ams-gra-codegen-oms schema-diff --before OLD_ROOT.xsd --after NEW_ROOT.xsd
ams-gra-codegen-oms schema-diff --before OLD_ROOT.xsd --after NEW_ROOT.xsd \
  --before-overlay OLD_EXTENSION.xsd --after-overlay NEW_EXTENSION.xsd \
  --format tsv
```

`--format text|tsv` defaults to text. Both overlay options are repeatable and
retain caller order independently. Singular options cannot be repeated.
No `--world`, `--language`, `--contract`, or `--output` is accepted.
Each side uses production `load_schema_set_with_overlays` exactly once and
semantic validation before comparison. Only stdout receives successful output;
input documents are never written, no directory is created and no code is generated.
Execution/loading/validation/output errors exit 1; usage errors exit 2.
A completed comparison exits 0 **even when differences exist**.

The independent crate `ams-gra-oms-schema-diff` depends only on `ams-gra-oms-ir`.
Its public `compare_schemas(&SchemaIr, &SchemaIr) -> SchemaDiff` expects validated
models and returns typed `Change`, `Property`, `ChangeKind` and `Value` records.
There is no second schema representation. Four QName indexes are built once;
declaration and member matching uses deterministic ordered maps/sets.

## Exact comparison boundary

Declaration identity is `(namespace_uri, local_name)`, never a prefix, source
location, path or array index. A namespace change is removal plus addition.
No rename is inferred. Common types compare abstractness, immediate base TypeRef,
kind family and the full normalized constraint set. Equal families compare:

* Primitive: primitive kind and existing effective declaration constraints.
* Alias: referenced TypeRef.
* Enumeration: exact wire-value additions/removals and common-value relative order.
* Record and Choice: local-name member identity, TypeRef, occurrence cardinality,
  nillability, wire namespace, field-local constraints and relative member order.
  Matching local names follows IR's member-uniqueness invariant; changed wire
  namespace is an explicit property record, not a silently ignored rename.
* List: item TypeRef and min/max occurrence cardinality.

All TypeRefs retain Primitive versus Named identity, full target QName and
Binary lexical provenance (`HexBinary`, `Base64Binary`, or unknown).
Constraints compare min/max inclusive/exclusive numeric values, exact/min/max
length, explicit whitespace policy, and complete ordered pattern groups,
alternatives, dialects and expressions using IR equality. No regex-equivalence
claim is made. A kind-family replacement has one kind record; obsolete/new
family-specific members are not redundantly inventoried.

Messages compare QName presence and their **direct** payload TypeRef. A changed
type structure is reported in TYPE, not as a duplicate message change when the
message's reference is unchanged. Transitive dependency impact is out of scope;
no service-readiness, dependency-closure, backend or runtime analysis is run.

Ignored: every `SourceRef` document/line, type/field/enum/message documentation,
namespace preferred prefixes and declaration array order. `schema_version` is
report metadata, never a per-declaration semantic change. Unused namespace
inventory itself is not inventoried; namespace identity in declarations and
references is retained.

## Stable output contract (v1)

Text includes both schema versions, before/after type/message counts and
added/removed/changed/unchanged totals, followed by detailed records with both
semantic values. Declaration identity uses Clark notation `{URI}local`.

TSV is UTF-8, LF-terminated, exactly seven columns:

```text
category namespace declaration member_or_property change before after
```

The actual delimiter is a tab. The header is always present, including empty
diffs. Record categories:

* METADATA: schema_version with before/after values.
* SUMMARY: TYPE/MESSAGE inventories and declaration counts.
* TOTAL: nonzero detailed change-kind totals, sorted by stable uppercase label.
* TYPE/MESSAGE: detailed records, ordered by category (types first), QName,
  optional local member or enum wire value, property label, then change kind.

The compact evidence fixture additionally uses IDENTITY for new message QNames
and PROPERTY_TOTAL for change-kind/property totals; those are campaign evidence
rows, not additional production CLI rows.

Each common declaration contributes exactly once to changed or unchanged counts.
Added/removed declarations contribute once to added/removed counts; they do not
emit redundant member records. Detailed totals count property records, **not**
changed declarations. A declaration can have several property records without
a redundant coarse TYPE_CHANGED marker. TYPE_CHANGED is reserved for kind,
primitive, alias-target, and list-property records. Relative order of common
members/enum values is compared; insertion/removal alone is not counted again
as an order change. Order records show complete before/after sequences.

Members are quoted local names followed by `/property`; declaration-wide records
use a property alone. Absent values are `absent`, distinct from quoted empty
strings. Primitive and kind labels use Rust enum names. TypeRefs use
`named:{URI}local` or `primitive:Kind` followed by `;binary=None` or
`;binary=Some(HexBinary|Base64Binary)`. Cardinality is `min..max` or
`min..unbounded`. Booleans are lowercase. Integer length values are decimal.
Numeric values, ordered strings and patterns use the existing typed Rust Debug
spelling (no whole TypeDecl Debug dump); this spelling is part of v1 and changes
require an output-contract revision. Strings are quoted with Rust escaping.

Every cell additionally escapes backslash as `\\`, tab as `\t`, newline as
`\n`, carriage return as `\r`, and other Unicode control characters as
`\u{hex}`. Thus every record occupies exactly one line, including unusual
names/expressions. There are no timestamps or host paths. Byte equality is
tested for repeated comparisons and CLI executions.

## What this does not establish

This is an exact **normalized IR change inventory**, not a backward/forward,
wire or safe-upgrade compatibility checker. Compatibility depends on cardinality
and defaults, direction, runtime behavior, polymorphism, enum handling, generated
API expectations and codec behavior. IR deliberately discards some XSD metadata,
including individual UCI declaration-version attributes. Even zero differences
cannot establish full UCI version-history compatibility or raw-XSD equivalence.
A later conservative analysis can consume these typed records.

## Validation and CI

Eleven exact synthetic test groups cover semantic families and properties,
provenance/documentation/prefix/declaration-order controls, direction, QName
collisions, Binary provenance, output determinism, overlays, usage errors,
malformed schemas and output failure. The Fast wrapper requires exactly one
passing test for every name, preventing zero-test false greens. No real download,
GNAT or generated-code compilation is needed by these Task069 controls.
Inherited compiler/backend/runtime gates are preserved.

Deep real-uci invokes the single pinned test harness once, reusing the job's
already-fetched roots. It verifies SHA256 again, loads/validates each side once,
checks inventories, compares both directions, verifies every reverse record,
checks selected properties directly against normalized IR, compares deterministic
TSV bytes and requires exact compact fixture equality plus:
`TASK069 REAL UCI SEMANTIC DIFF: PASSED`.
The 240-minute inherited budget is unchanged.

## Pinned source identities

Authoritative repository: `https://gitlab.com/open-arsenal/uci/standard.git`.

| Release | Commit | Root SHA256 |
|---|---|---|
| 2.5 | `093610b7753944059360d3236770ab446d039556` | `ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27` |
| 2.6 | `78eb61b6112c8bffa40820c33124b57787fc5bd9` | `af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b` |

The existing pinned fetch scripts verify revisions and digests; roots include
their complete dependency closure. Full forward/reverse detail is retained
locally under `/tmp/task069/evidence`, not vendored UCI schema files.

## Measured UCI 2.5 → 2.6 results

| Inventory | Before | After | Added | Removed | Changed | Unchanged |
|---|---:|---:|---:|---:|---:|---:|
| Types | 5,557 | 5,570 | 39 | 26 | 162 | 5,369 |
| Messages | 722 | 725 | 3 | 0 | 0 | 722 |

Detailed distribution (309 total property/presence records):

| Change kind | Property | Records |
|---|---|---:|
| BASE_TYPE_CHANGED | base_type | 37 |
| CONSTRAINT_CHANGED | minLength | 4 |
| CONSTRAINT_CHANGED | pattern_groups | 5 |
| CONSTRAINT_CHANGED | whiteSpace | 2 |
| ENUM_VALUE_ADDED | presence | 5 |
| MEMBER_ADDED | presence | 30 |
| MEMBER_CHANGED | cardinality | 3 |
| MEMBER_CHANGED | type_ref | 99 |
| MEMBER_REMOVED | presence | 56 |
| MESSAGE_ADDED | presence | 3 |
| TYPE_ADDED | presence | 39 |
| TYPE_REMOVED | presence | 26 |

All other categories have zero records in this campaign. New messages are:

* `{https://www.vdl.afrl.af.mil/programs/oam}SystemSchedule`
* `{https://www.vdl.afrl.af.mil/programs/oam}SystemScheduleDataRequest`
* `{https://www.vdl.afrl.af.mil/programs/oam}SystemScheduleDataRequestStatus`

Reverse 2.6 → 2.5: types added 26, removed 39, changed 162, unchanged 5,369;
messages added 0, removed 3, changed 0, unchanged 722. Every reverse record was
verified against swapped forward values and the corresponding reversed kind.
139 base/member properties were independently checked against both normalized
declaration indexes. Examples include `ADS_B_KinematicsContributionType`'s
`AirborneSurfaceFormat` (Boolean → named EmptyType), its `SV_QualitySIL_Supplement`
(Boolean → named enum), and `AMTI_CapabilityType`'s base reference
(CommandableCapabilityDeclarationType → CapabilityDeclarationType).

One completed local debug-build campaign took **463.481 seconds**, including
both loads, explicit validation, forward/reverse comparisons, repeated TSV
comparison and IR checks. A preceding foreground attempt was interrupted by
the tool's 30-second timeout before any evidence; it is not counted as a
completed campaign. Existing frontend source-position calculation dominates
debug loading; Task069 does not change that frontend or run backend analysis.

The inspected, measured compact fixture is
`tests/fixtures/schema-diff/task069-uci25-to-uci26-summary.tsv` (36 lines).
Full reports each contain 330 lines (header/metadata/summary/totals plus 309
detail records), approximately 55 KB, retained outside the repository:

* forward SHA256: `c58fbca0f0958d063ec156dfca065f6826f5a1c0088cd2f688947644a433d09c`
* reverse SHA256: `c3e2d76fff81b42372fe067f47c123eda794efc0e2adaa0b78b6bf1504ebd2c4`

The fixture was frozen from measured engine output after inspecting its exact
rows, not from a hand-authored expected difference list. Hosted Deep CI requires
byte-for-byte fixture equality. Local repeated-output controls passed; local
fmt, workspace check, workspace Clippy (`-D warnings`), engine tests, 280 CLI
tests and Rust 1.95 engine/affected CLI tests passed. The split guard and 144
adversarial checks passed, as did two adversarial wrapper test groups.