# Language-Neutral Schema IR

## Purpose

The schema IR is the architectural center of `ams-gra-codegen-oms`. It is a normalized semantic model that sits between UCI/OMS XSD and every language backend.

It exists to ensure that Ada, Rust, C++, and future generators all consume the **same interpretation** of the authoritative schema.

## What the IR is not

The IR is not:

- a replacement for UCI XSD;
- a new OMS wire format;
- DDS IDL;
- a language-specific AST;
- a raw XML DOM;
- a 1:1 transcription of XSD syntax.

## Design principles

### 1. Semantic, not syntactic

By the time a backend sees a record field, its namespace and type reference should already be resolved. A backend should not need to know whether the XSD used a local prefix, imported schema, anonymous type, group reference, or extension to express it.

### 2. Preserve constraints

Schema restrictions are part of the API contract. The IR must preserve enough information to generate equivalent validators and, where practical, native constrained types/contracts.

### 3. Preserve provenance

Every declaration should be traceable to its authoritative source document and schema version. Diagnostics and generated comments should be able to point back to the input.

### 4. Stable identity

A type's identity must be based on its resolved qualified name and schema context, not a filesystem-specific relative path or transient namespace prefix.

### 5. Explicit cardinality

Optionality and repetition must not be inferred later by language backends.

### 6. Separate shape from message role

A UCI publishable message and a nested complex type may have similar structural shapes but different semantic roles. The IR should represent message classification explicitly.

## Initial conceptual model

```text
SchemaIr
  schema_version
  namespaces[]
  types[]
  messages[]
  provenance

Namespace
  uri
  preferred_prefix?

TypeDecl
  name: QualifiedName
  abstract: bool
  base_type?
  kind: TypeKind
  constraints
  documentation
  source

TypeKind
  Primitive
  Alias(TypeRef)
  Enum(variants[])
  Record(fields[])
  Choice(alternatives[])
  List(item_type, bounds)

FieldDecl
  name
  type_ref: Primitive(kind) | Named(QualifiedName)
  cardinality
  nillable
  constraints
  documentation

Cardinality
  min_occurs
  max_occurs?   # None means unbounded

ConstraintSet
  min_inclusive?: NumericValue
  max_inclusive?: NumericValue
  min_exclusive?: NumericValue
  max_exclusive?: NumericValue
  length?
  min_length?
  max_length?
  lexical

LexicalConstraintSet
  pattern_groups[]
  white_space?: Preserve | Replace | Collapse  # explicit derived facet only

PatternGroup
  alternatives[]: PatternExpression

PatternExpression
  dialect: XmlSchema
  expression

MessageDecl
  name
  payload_type
  documentation
  source
```

The actual implementation will evolve as real UCI schemas expose requirements, but backends should depend on versioned IR invariants rather than frontend implementation details.

The primitive/named distinction is explicit after QName resolution. This keeps
backends from inferring whether a reference denotes an XSD primitive or a
schema declaration by inspecting namespace strings. Numeric bounds are stored
as semantic values rather than lexical XML strings. `NumericValue` distinguishes
exact `Integer(i128)`, binary32 `Float32`, and binary64 `Float64` domains. The
floating wrappers store width-correct IEEE bits and provide deterministic
equality without relying on raw float `Eq`.

Task 022 backends generate unconstrained Float32/Float64 values at their native
width. Any non-default floating `ConstraintSet` remains backend fail-closed;
this does not imply floating range lowering.

Numeric range validation compares only values in the same domain, using exact
integer ordering or semantic floating ordering rather than bit ordering. A
lower bound greater than its upper bound is contradictory; equal endpoints are
contradictory when either is exclusive. Mixed domains are invalid. Range NaN is
invalid because it is unordered. The frontend accepts finite Rust-parsable
decimal and exponent floating spellings at the primitive width and rejects
non-finite values and negative zero; this deliberately does not claim complete
XSD floating lexical support.

### Named simple restrictions

Named simple restrictions are normalized after every reachable schema document
has been parsed. A frontend-private pending form retains the immediate resolved
base, local facets, source, and output position. Recursive graph resolution
supports forward and multi-level references while final declaration order stays
document-discovery then source order.

The normalized `kind` is the ultimate primitive kind, while `base_type` remains
the immediate named relationship. Constraints are effective: intrinsic,
inherited, and local typed numeric ranges are intersected, as are exact/minimum/
maximum lengths. `TypeDecl.source` identifies the derived declaration; facet-
level provenance is not modeled.

Because constraints are effective, each normalized derived declaration must
semantically imply the effective constraints of its immediate named base.
`SchemaIr::validate()` enforces this independently of the frontend: numeric
bounds preserve domain and inclusive/exclusive strength, effective length
intervals remain subsets, inherited lexical pattern groups remain an exact
prefix, and effective whitespace policy cannot weaken. This validates
normalization without recomputing it or attempting regex-language inclusion.

Cycles and named bases that are structural, enumeration, or change primitive
family are invalid. A derived restriction without local patterns inherits the
base groups. A local set of patterns appends one new group while preserving the
base groups and immediate named `base_type`; groups are never flattened across
restriction levels. Named enumeration restrictions remain unsupported.

Ada, Rust, and C++ do not generate constrained floating wrappers. Integer-bound
helpers accept only `NumericValue::Integer`; constrained floating declarations
produce structured unsupported errors before rendering, never coercion or loss.

Primitive kinds preserve numeric value-space distinctions. `Decimal` denotes
decimal arithmetic and is not an umbrella numeric kind. `Float32` and
`Float64` denote the binary floating-point value spaces used by XSD `float` and
`double`, respectively. In particular, `xs:double` is never normalized to
`Decimal`. The floating kinds do not carry integer min/max constraints, and the
IR distinction leaves room for XSD floating values such as `NaN`, `INF`,
`-INF`, and negative zero without silently normalizing them away.
`NumericValue::Integer(i128)` stores integral bounds, while `Float32` and
`Float64` wrappers store width-specific floating bounds. Range validation uses
semantic ordering only within the same numeric domain. The backends still fail
closed for constrained floating declarations because generation support for
such constraints has not been implemented.

Temporal primitive kinds are similarly distinct: `DateTime`, `Time`, and
`Duration` model separate XSD value spaces. A temporal pattern remains a lexical
constraint and is not converted into a timezone/value-space policy. The IR does
not choose a runtime lexical parser, timezone policy, calendar arithmetic,
duration unit, or precision. In particular, `Duration` is not an integer count
of time units.

Named simple restrictions over supported built-in scalar primitives normalize
as `TypeKind::Primitive(kind)` plus a `ConstraintSet`; their immediate
primitive ancestry remains in `TypeDecl.base_type`. String enumerations remain
`TypeKind::Enumeration` in source order. Constraints owned by a named type stay
on that `TypeDecl`, while intrinsic constraints of a direct built-in field stay
on its `FieldDecl`.

Value-space and lexical restrictions are deliberately separate. For `String`,
`length`, `min_length`, and `max_length` count Unicode code points (the XSD
string length unit). `LexicalConstraintSet.pattern_groups` stores effective
pattern facets in base-to-derived order. XML Schema combines patterns declared
in one restriction step disjunctively, so one `PatternGroup` contains their
source-ordered alternatives. Pattern facets introduced at different derivation
levels combine conjunctively, so each level is a separate group. Every
`PatternExpression` explicitly carries `PatternDialect::XmlSchema`; it must not
be interpreted as Rust regex, PCRE, ECMAScript, or POSIX syntax.

`WhiteSpacePolicy` stores the three XSD operations: `Preserve` leaves text
unchanged; `Replace` maps tab, line feed, and carriage return to spaces; and
`Collapse` additionally strips leading/trailing spaces and coalesces runs.
Every represented primitive has an intrinsic XSD policy: `String` uses
restrictable `Preserve`; all other current `PrimitiveKind` variants use fixed
`Collapse`. `PrimitiveKind::intrinsic_white_space_policy()` and
`intrinsic_white_space_is_fixed()` expose that baseline.

`LexicalConstraintSet.white_space` stores only an explicit derived
`xs:whiteSpace` facet. `None` means “no explicit derived facet is stored here,”
not “no whitespace normalization exists.” `effective_white_space(primitive)`
selects the explicit policy when present and otherwise the primitive baseline.
Validation rejects an explicit policy that contradicts a fixed baseline, and
String restrictions may tighten only in the order
`Preserve < Replace < Collapse`. Named-restriction implication compares these
effective policies, including when either explicit field is `None`.

XML Schema applies effective whitespace normalization before length and pattern
evaluation. Pattern groups do not duplicate the intrinsic policy: a future
runtime obtains the primitive kind, computes its effective whitespace policy,
and then interprets the retained XML Schema pattern groups. Thus whitespace,
length, and pattern constraints coexist rather than replacing one another.

For `Binary`, length bounds count octets, as specified for XSD `hexBinary`;
they never count hexadecimal lexical characters. Task 016 preserves observed
patterns for String, SignedInteger, DateTime, and Time, and observed whitespace
facets for String. Unobserved primitive/facet combinations fail closed.

The current Ada, Rust, and C++ backends do not execute XML Schema regexes or
normalize whitespace, and do not generate the newly recognized temporal
primitives or constrained string/binary declarations. Their pre-render
validation is a lexical constraint firewall: any non-default lexical constraint
is rejected explicitly, so no backend can emit an unconstrained approximation
or partial output. Regex translation/execution, lexical validators, and temporal
parsing/serialization remain runtime work outside this model.

The frontend maps direct XSD scalar fields by resolved namespace URI:
`boolean` to `Boolean`; `byte`, `short`, `int`, `long`, and `integer` to
`SignedInteger`; `unsignedByte`, `unsignedShort`, and `unsignedInt` to
`UnsignedInteger`; `float`/`double` to `Float32`/`Float64`; `dateTime` to
`DateTime`; `time` to `Time`; `duration` to `Duration`; `hexBinary` to `Binary`;
and `string` to `String`. The generic
integer kinds deliberately do not encode machine width. Fixed-width XSD types
instead carry exact intrinsic minima and maxima in `ConstraintSet`. Restrictions
intersect explicit inclusive or exclusive bounds with those intrinsic bounds;
unbounded `xs:integer` remains unconstrained.

A named complex type whose sole content model is a default-cardinality
`xs:choice` becomes `TypeKind::Choice`. Each local element becomes one
`FieldDecl` alternative through the same QName, documentation, nillability,
cardinality, primitive-constraint, and provenance normalization as a sequence
field. Alternative source order is preserved, and an empty semantic choice is
invalid. Task 012 evidence contains only default 1..1 choice-group cardinality,
so group cardinality is not added to the IR; non-default group bounds remain
explicitly unsupported rather than being smeared across alternatives.

`Cardinality.max_occurs = None` represents `maxOccurs="unbounded"` for record
fields and choice alternatives. Frontend recognition does not imply backend
generation support: current backends continue to reject choices, unbounded
containers, and newly recognized scalar kinds where they cannot preserve the
semantics.

`PrimitiveKind` describes value types usable by semantic fields and general
type references. It does not broaden UCI message classification: in the
currently supported authoritative schema model, every `MessageDecl` payload
must reference a named schema type. A global UCI element whose `type` resolves
to any XSD primitive is rejected rather than normalized into a message.

## Normalization examples

### Namespace prefixes

Input documents may refer to the same namespace with different prefixes:

```xml
<xs:schema xmlns:u="urn:uci:example">...</xs:schema>
```

or:

```xml
<xs:schema xmlns:uci="urn:uci:example">...</xs:schema>
```

The IR stores the resolved namespace URI, not the arbitrary source prefix.

### Occurrence bounds

XSD:

```xml
<xs:element name="Track" type="uci:TrackType" minOccurs="0" maxOccurs="16"/>
```

IR:

```text
field.name = Track
field.type = {urn:...}TrackType
field.cardinality = 0..16
```

Backends then choose language-native representations:

- Ada: bounded container + contract/subtype strategy;
- Rust: bounded wrapper or `Vec<T>` plus generated validation;
- C++: container plus validation/helper type.

### First frontend slice

```text
xs:simpleType restriction of xs:integer, minInclusive=1, maxInclusive=65535
    ↓
Primitive(SignedInteger), constraints = 1..65535

xs:element type="oms:Track_Id", minOccurs="0", maxOccurs="8"
    ↓
field.type = Named({urn:example:oms:track}Track_Id)
field.cardinality = 0..8
```

The frontend accepts either one standalone schema document or a recursively
loaded schema set containing local `xs:include` and `xs:import` dependencies.
Each QName is resolved with the namespace bindings of the document in which it
appears, so prefixes remain document-local aliases. Includes, imports,
`schemaLocation` values, and lexical prefixes are discarded before the IR
boundary. The semantic subset includes direct `float` and `double` field
references, named integer restrictions, string enumerations, and sequence-based
records. Floating restriction facets and other unsupported XSD syntax produce
an explicit diagnostic rather than being silently discarded.

Schema-set declaration order is pre-order depth-first document discovery in
dependency source order, then declaration source order. Namespace URI order and
preferred-prefix selection are first-seen under the same traversal. Preferred
prefixes are presentation metadata, not global QName bindings.

For the authoritative UCI message-definition schemas, the currently supported
global-message form is a schema-level `xs:element` carrying unqualified `name`
and `type` attributes and the UCI/OAM namespaced `version` attribute. That
observed declaration shape normalizes to a `MessageDecl`. Its qualified element
name is the message identity and its resolved `type` QName is the payload
reference. The payload remains one `TypeDecl`; the frontend does not synthesize
a second type named after the element. Generic global XSD elements without the
UCI version marker remain outside the supported semantic subset. Local
`xs:element` declarations inside sequences remain `FieldDecl`s. Message order is
deterministic document discovery order followed by source order within each
document, independent of type declaration order.

The UCI/OAM declaration version is the same change-history metadata on global
messages, named complex types, and named simple types. When present, its value
must be nonempty, is otherwise treated as opaque, and is discarded during
semantic normalization. It does not participate in type identity, validation
constraints, JSON/OWP representation, or backend-visible IR. The marker is
required to classify a schema-level element as a UCI message, but is optional on
generic named type declarations; authoritative UCI certification separately
requires it on UCI types. Other declaration attributes and anonymous global
types fail closed.

This per-declaration change history is distinct from `SchemaIr.schema_version`,
which records the root `xs:schema @version`. Declaration versions must not be
folded into that schema-release field.

The frontend validates schema-level `elementFormDefault` and
`attributeFormDefault` values. These settings govern local element and attribute
qualification in XML instances, but this IR models local field wire names for
language-native UCI JSON/LA-CAL types rather than XML instance serialization.
They are therefore deliberately discarded during normalization. Other schema
attributes remain unsupported unless handled explicitly.

Leading XSD annotations are metadata rather than content-model children. The
frontend accepts optional leading annotations containing supported plain-text
`xs:documentation` children and normalizes documentation owned by named simple
or complex types, global messages, sequence fields, and enumeration variants
into their existing IR `documentation` fields. XML whitespace runs are
collapsed to one space, empty documentation nodes are omitted, and multiple
nonempty documentation nodes are joined with one blank line. This makes the
value independent of XSD indentation and line wrapping while retaining document
boundaries.

Schema-level documentation and documentation attached to supported
restrictions have no corresponding semantic IR owner and are deliberately
discarded. An annotation on an otherwise unsupported owner, such as the
observed pattern facet, does not make that owner supported. The IR does not
preserve raw annotation XML. `xs:appinfo`, embedded markup, unknown annotation
children, and annotations outside the leading position remain explicit frontend
errors rather than silently discarded metadata.

This deterministic frontend order is not a declaration schedule. Schema IR does
not promise that source/discovery order can be emitted directly by a language
backend. The common `codegen-core` planner visits declarations in IR order,
builds unique named-dependency edges, and performs a stable topological sort.
Among currently dependency-satisfied declarations, the declaration with the
lowest original IR index is emitted next. Cycles are explicit planning errors.

### Extension/inheritance

For a structural complex type derived through supported XSD extension,
`base_type` is a named reference to the **immediate** base declaration and
`kind` contains only the content declared locally by the derived type. A local
sequence becomes `TypeKind::Record`; a local choice becomes
`TypeKind::Choice`. An extension with no local compositor becomes an empty
local record. That means “no locally declared content,” not “no effective
content.”

Inherited fields and alternatives are **not** duplicated into the derived
`TypeKind`. Each base remains a separate `TypeDecl`, including through
multi-level chains. This preserves type identity, immediate source-level
inheritance, provenance, and documentation ownership. `TypeDecl.is_abstract`
preserves XSD complex-type abstract metadata independently of backend policy.

This structural use of `base_type = Named(...)` is distinct from primitive
`base_type` ancestry on a normalized named simple restriction. Structural
records and choices may only have named structural bases. The IR validator
rejects non-structural bases and inheritance-only cycles without treating
ordinary field-reference recursion as an inheritance cycle.

The shared declaration planner includes immediate base references as
dependencies, so bases precede their derived declarations regardless of source
order. Current Ada, Rust, and C++ backends do not implement inheritance: each
rejects a record or choice with a named base before rendering, rather than
silently generating only its local content. They continue to accept supported
simple declarations whose `base_type` records primitive restriction ancestry.

## IR invariants

Before code generation begins:

1. all non-external type references are resolved;
2. namespace URIs are canonicalized;
3. type identities are unique;
4. occurrence bounds are valid;
5. inherited restrictions are compatible;
6. anonymous types have deterministic synthetic identities;
7. message classification references concrete payload types;
8. unsupported XSD constructs produce explicit diagnostics rather than silent degradation;
9. declaration ordering is deterministic;
10. provenance exists for every generated declaration.

`SchemaIr::validate` enforces the currently representable language-neutral
invariants after a frontend has assembled the complete schema. It validates
declared namespace membership and uniqueness, qualified type identity,
qualified message identity, resolution of every modeled named-reference
location, finite cardinality, numeric and length consistency, and nonempty
enumerations. Type and message names occupy separate symbol spaces, so matching
qualified names across those categories are valid. Primitive references need no
declaration. Unbounded cardinality and unconstrained integers remain valid IR
even where an initial backend cannot yet represent them.

## Why this matters for SPARK

A normalized constraint model allows the Ada backend to make principled decisions about which XSD restrictions can become:

- Ada subtypes;
- discriminants;
- bounded container types;
- `Pre`/`Post` aspects;
- `Type_Invariant`/predicate aspects;
- generated validation functions.

Without a semantic IR, these decisions become tangled with XML parsing and namespace mechanics, making formal reasoning much harder.
