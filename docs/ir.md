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
  min_inclusive?
  max_inclusive?
  min_exclusive?
  max_exclusive?
  length?
  min_length?
  max_length?
  pattern[]
  enumeration[]

MessageDecl
  name
  payload_type
  role metadata
  source
```

The actual implementation will evolve as real UCI schemas expose requirements, but backends should depend on versioned IR invariants rather than frontend implementation details.

The primitive/named distinction is explicit after QName resolution. This keeps
backends from inferring whether a reference denotes an XSD primitive or a
schema declaration by inspecting namespace strings. Integer bounds are stored
as numeric values rather than lexical XML strings.

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
boundary. The semantic subset remains named integer restrictions, string
enumerations, and sequence-based records; other XSD syntax produces an explicit
diagnostic rather than being silently discarded.

Schema-set declaration order is pre-order depth-first document discovery in
dependency source order, then declaration source order. Namespace URI order and
preferred-prefix selection are first-seen under the same traversal. Preferred
prefixes are presentation metadata, not global QName bindings.

The frontend validates schema-level `elementFormDefault` and
`attributeFormDefault` values. These settings govern local element and attribute
qualification in XML instances, but this IR models local field wire names for
language-native UCI JSON/LA-CAL types rather than XML instance serialization.
They are therefore deliberately discarded during normalization. Other schema
attributes remain unsupported unless handled explicitly.

Leading XSD annotations are metadata rather than content-model children. The
frontend accepts annotations containing one or more plain-text
`xs:documentation` children and normalizes documentation owned by named simple
or complex types, sequence fields, and enumeration variants into their existing
IR `documentation` fields. XML whitespace runs are collapsed to one space,
empty documentation nodes are omitted, and multiple nonempty documentation
nodes are joined with one blank line. This makes the value independent of XSD
indentation and line wrapping while retaining document boundaries.

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

The frontend resolves XSD extension chains once. The IR may preserve both `base_type` and effective fields so backends can choose composition, inheritance, traits/interfaces, or flattening without repeating schema resolution.

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
resolution of every modeled named-reference location, finite cardinality,
numeric and length consistency, and nonempty enumerations. Primitive references
need no declaration. Unbounded cardinality and unconstrained integers remain
valid IR even where an initial backend cannot yet represent them.

## Why this matters for SPARK

A normalized constraint model allows the Ada backend to make principled decisions about which XSD restrictions can become:

- Ada subtypes;
- discriminants;
- bounded container types;
- `Pre`/`Post` aspects;
- `Type_Invariant`/predicate aspects;
- generated validation functions.

Without a semantic IR, these decisions become tangled with XML parsing and namespace mechanics, making formal reasoning much harder.
