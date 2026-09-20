# UCI Schema Compatibility

Tested: **2026-09-18**.

This is an evidence log for frontend development, not a claim of full UCI
compatibility. UCI schema files are obtained and exercised externally; they are
not vendored in this repository.

## Public source

Both releases were obtained from the authoritative public [UCI Standard
repository](https://gitlab.com/open-arsenal/uci/standard):

| Target | Tag | Commit | Root schema | Required dependency | Fully normalizes? |
|---|---|---|---|---|---|
| UCI 2.5 / Sleet baseline | `v2.5` | `093610b7753944059360d3236770ab446d039556` | `UCI_MessageDefinitions_v2_5_0.xsd` | `UCI_SecurityMarkings_v2_5_0.xsd` via `xs:include` | Yes: 5,557 types, 722 messages |
| UCI 2.6 / current public release | `v2.6` | `78eb61b6112c8bffa40820c33124b57787fc5bd9` | `UCI_MessageDefinitions_v2_6_0.xsd` | `UCI_SecurityMarkings_v2_6_0.xsd` via `xs:include` | Yes: 5,570 types, 725 messages |

The separate `UCI_Versioning_v2_*_0.xsd` documents import the corresponding
message-definition schema. The compatibility probe used the message-definition
file as the root because it is the UCI message model consumed by this project.

The current frontend milestone is complete for these two authoritative roots.
The dated sections below retain the sequence of earlier evidence-driven
frontiers; they are historical probe results, not the current status. This does
not claim general-purpose support for every XML Schema construct, and backend
generation remains partial.

## 2026-09-17 probes

The first probe added support for schema-level
`elementFormDefault="qualified"`. The frontend recognizes and lexically validates
the XSD enumeration values `qualified` and `unqualified`, then deliberately
discards the value during normalization because it controls local element
qualification in XML instances rather than the language-native UCI JSON model.

Unsupported-construct diagnostics now identify the source file and deterministic
`roxmltree` line and column. Both roots then stopped at line 2 on:

```text
unsupported XSD construct: xs:schema @attributeFormDefault at <root>:2:1
```

In both releases the value is `unqualified`. The frontend now recognizes
schema-level `attributeFormDefault`, validates its value as either `qualified` or
`unqualified`, and deliberately discards it during normalization.
`attributeFormDefault` affects XML instance namespace qualification, while the
current backend-visible model targets language-native JSON/LA-CAL semantics.
Preserving this XML-representation-only setting in `SchemaIr` would therefore
violate the normalization boundary and provide no current backend value.

Re-running both roots advances beyond `attributeFormDefault` to the same next
blocker:

```text
UCI 2.5: FrontendError::UnsupportedConstruct
unsupported XSD construct: xs:annotation at UCI_MessageDefinitions_v2_5_0.xsd:3:2

UCI 2.6: FrontendError::UnsupportedConstruct
unsupported XSD construct: xs:annotation at UCI_MessageDefinitions_v2_6_0.xsd:3:2
```

No version-specific difference has been observed through this point. Support
for `xs:annotation` remains intentionally unimplemented for the next
evidence-driven compatibility task. UCI 2.5 remains the Sleet interoperability
baseline, while UCI 2.6 is the forward-compatibility target; neither schema
currently normalizes completely.

## Annotation inventory and policy

The annotation probe recursively inventoried each root and its security-marking
include (two files per release). UCI 2.5 contains 27,214 annotations and 27,943
documentation nodes; UCI 2.6 contains 27,263 annotations and 27,997
documentation nodes. Neither release contains `xs:appinfo`, documentation
attributes, or embedded XML markup. All annotations are the first element child
of their owner.

| Annotation owner | UCI 2.5 | UCI 2.6 |
|---|---:|---:|
| schema | 2 | 2 |
| element | 13,882 | 13,923 |
| complex type | 4,612 | 4,636 |
| simple type | 945 | 934 |
| enumeration facet | 7,766 | 7,767 |
| restriction | 6 | 0 |
| pattern facet | 1 | 1 |

Most annotations have one documentation child. UCI 2.5 has 727 annotations
with two and one annotation with three; UCI 2.6 has 730 with two and two with
three. Both releases contain plain single-line text, multiline text, and one
empty documentation node. The material shape difference is the six
restriction annotations found only in 2.5; 2.6 also puts a second multiline
disclaimer in its root schema annotation.

The frontend now parses an optional leading annotation separately from its
owner's semantic children. Plain documentation is normalized by trimming outer
whitespace and collapsing every XML whitespace run to one space. Empty nodes
are omitted, and multiple nonempty nodes are joined by one blank line. Type,
sequence-field, and enumeration-facet documentation reaches the existing IR
documentation fields. Validated schema and supported-restriction documentation
is deliberately discarded because those XSD syntax owners have no semantic IR
documentation field. At Task 008, pattern semantics remained unsupported, so
the frontend did not bypass the pattern owner merely to consume its annotation.
Task 014 later added semantic String pattern support. A supported String pattern
facet now validates an optional leading annotation and deliberately discards its
normalized documentation because `ConstraintSet` has no documentation owner.
Unsupported pattern families remain fail-closed. No raw XML metadata is exposed
to backends.

Because real UCI provides no evidence that `xs:appinfo` is safe to ignore, it
remains unsupported. Embedded markup, unknown annotation children, unexpected
annotation attributes, and annotations outside the leading position also fail
closed with source context.

The one pattern-facet annotation in each release is on
`NATO_SpecialWordsType` in the corresponding security-marking schema. Both
types restrict `xs:string`; the supported pattern is
`NATO:[a-zA-Z\-_]{1,256}`. In both releases the pattern has exactly one element
child: a leading, attribute-free `xs:annotation` containing exactly one
attribute-free, plain-text `xs:documentation` node with the text “North Atlantic
Treaty Organization Special Words.” The two occurrences are structurally and
semantically equivalent. Task 014 validates this evidence-backed shape but does
not add the documentation to semantic constraints. All accepted scalar facets
reject residual element children after their optional leading annotation.

After this annotation slice, both roots pass their original schema-level
annotation and stop at the next unsupported declaration:

```text
UCI 2.5: FrontendError::UnsupportedConstruct
unsupported XSD construct: xs:element at UCI_MessageDefinitions_v2_5_0.xsd:14:2

UCI 2.6: FrontendError::UnsupportedConstruct
unsupported XSD construct: xs:element at UCI_MessageDefinitions_v2_6_0.xsd:19:2
```

The construct is the same in both releases. The line differs because the 2.6
root annotation contains the additional disclaimer documentation. Global
`xs:element` support remains intentionally unimplemented.

## Global message element inventory and policy

The global-element probe used each message-definition root and its reachable
security-marking include. Only `xs:element` nodes whose direct parent is
`xs:schema` were counted.

| Release | Message definitions | Security markings | Total globals |
|---|---:|---:|---:|
| UCI 2.5 | 722 | 0 | 722 |
| UCI 2.6 | 725 | 0 | 725 |

Every global in both releases has the same shape: required unqualified `name`
and `type` attributes, the namespaced UCI/OAM `version` attribute, and one
leading annotation containing exactly two nonempty documentation nodes. All
payload QNames resolve to named types in the reachable schema set. None refers
directly to an XSD primitive, uses an anonymous simple or complex type, or has
`ref`, `abstract`, `nillable`, `substitutionGroup`, `default`, `fixed`, `block`,
or `final`. No payload type is shared by multiple real UCI globals. No element
anywhere in either reachable set uses `ref` or `substitutionGroup`, so these
globals provide no evidence of a reusable XML-element role.

The UCI Schema Style and Design Specification supplies the semantic
classification rule rather than a name heuristic. Its design principles say to
create distinct messages as top-level elements intended to be sent as a whole;
CERT SCH-000272 calls every declared global element a “Message” and requires its
associated MT and MDT declarations; CERT SCH-009994 limits global construction
to `name`, `type`, and `uci:version`. Therefore every global in these two
reachable authoritative sets maps to `MessageDecl`, while its referenced named
type remains a separate `TypeDecl` payload shape.

Global documentation cannot be discarded: all 722 UCI 2.5 and all 725 UCI 2.6
elements differ from their payload-type documentation. The globals contain the
message purpose and primitive classification, while each referenced MT says to
consult the associated global message annotation. `MessageDecl` therefore now
owns normalized optional documentation under the same whitespace and
multi-document rules as other semantic declarations.

The frontend supports only this observed global shape. A schema-level element
must carry unqualified `name` and `type` attributes and the UCI/OAM namespaced
`version` marker before it is classified as a message; a generic named and typed
global without that marker is not classified as `MessageDecl`. The version value
must be nonempty, is otherwise treated as opaque, and is discarded because
declaration version history has no current semantic IR consumer. The frontend
resolves message and payload QNames in the source document, preserves source
provenance, and retains schema-set discovery order followed by element source
order. Any other attribute or an anonymous type remains an explicit unsupported
construct. Message identities must be unique, payload references must resolve
after the complete schema set is assembled, and type and message symbol spaces
remain distinct.

UCI 2.6 adds exactly three globals relative to 2.5: `SystemSchedule`,
`SystemScheduleDataRequest`, and `SystemScheduleDataRequestStatus`. There are no
other global-element shape differences.

After normalizing all 722 or 725 global messages, both roots stop at the next
distinct unsupported construct:

```text
UCI 2.5: FrontendError::UnsupportedConstruct
unsupported XSD construct: xs:complexType @version at UCI_MessageDefinitions_v2_5_0.xsd:4639:2

UCI 2.6: FrontendError::UnsupportedConstruct
unsupported XSD construct: xs:complexType @version at UCI_MessageDefinitions_v2_6_0.xsd:4663:2
```

The construct is the same in both releases; only its source position differs.

## Declaration-version inventory and policy

The declaration-version probe traversed each message-definition root and its
reachable security-marking include. It matched attributes by expanded name,
`{https://www.vdl.afrl.af.mil/programs/oam}version`, rather than by lexical
prefix.

| Owner | UCI 2.5 message definitions | UCI 2.5 security markings | UCI 2.5 total | UCI 2.6 message definitions | UCI 2.6 security markings | UCI 2.6 total |
|---|---:|---:|---:|---:|---:|---:|
| global `xs:element` | 722 | 0 | 722 | 725 | 0 | 725 |
| `xs:complexType` | 4,607 | 5 | 4,612 | 4,631 | 5 | 4,636 |
| `xs:simpleType` | 927 | 18 | 945 | 915 | 19 | 934 |
| `xs:restriction` | 0 | 0 | 0 | 0 | 0 | 0 |
| `xs:enumeration` | 0 | 0 | 0 | 0 | 0 | 0 |
| `xs:attribute` | 0 | 0 | 0 | 0 | 0 | 0 |
| `xs:group` | 0 | 0 | 0 | 0 | 0 | 0 |
| `xs:attributeGroup` | 0 | 0 | 0 | 0 | 0 | 0 |
| **Total** | **6,256** | **23** | **6,279** | **6,271** | **24** | **6,295** |

These are the only observed owner kinds. Every named complex and simple type in
both reachable sets has exactly one OAM declaration version, as does every
global element; local elements do not. No declaration has multiple
version-like attributes, and no OAM value is empty or whitespace-only. The only
unqualified `version` attributes are the two `xs:schema` release versions in
each reachable set, so they are not declaration metadata.

All 6,279 UCI 2.5 and 6,295 UCI 2.6 declaration values have the observed lexical
shape `ddd.ddd.ddd.ddd`; representative values include `000.000.000.000`,
`001.000.000.000`, and `005.003.005.001`. There are 222 distinct values in 2.5
and 296 in 2.6, with many values inside each release. Among common named
declarations, 2,096 values differ between releases. The values therefore track
individual declarations and changes rather than duplicating the `002.5.0` or
`002.6.0` schema release. The authoritative `uci:VersionType` also permits an
optional lowercase engineering suffix on each component, although no suffix is
present in these reachable release schemas.

The UCI Standard Document, section 5.2, says UCI Message Versioning quantizes
developer impact between schema releases and tracks complex- and simple-type
changes at the same detail. Its four components represent direct structural,
indirect structural, direct optional, and indirect optional change history; a
message version is inherited from its associated MT complex type. The UCI
Schema Style and Design Specification likewise says `uci:version` identifies
messages and types, CERT SCH-002406 ties it to `uci:VersionType`, and CERTs
SCH-009994/SCH-009995 allow it on global elements and complex types. This is
declaration change-history/provenance metadata. It does not itself alter XSD
constraints, declaration identity, UCI JSON, CAL/OWP wire behavior, or generated
runtime behavior.

The frontend consequently applies one namespace-aware declaration-version
policy to messages, complex types, and simple types: a present OAM version must
be nonempty, remains lexically opaque, and is discarded at the semantic IR
boundary. No numeric grammar is duplicated from the certification schema.
Global-message classification still requires the marker; generic supported
complex and simple types may omit it. Thus authoritative UCI declarations are
accepted without narrowing the generic XSD subset, and `SchemaIr.schema_version`
remains the separate root-schema release value. No IR or backend structure was
added.

After this shared type-declaration support, both roots pass their original
`xs:complexType @version` blocker and all `xs:simpleType @version` metadata, then
stop at the next distinct unsupported construct:

```text
UCI 2.5: FrontendError::UnsupportedConstruct
unsupported XSD construct: xs:double at UCI_MessageDefinitions_v2_5_0.xsd:4703:4

UCI 2.6: FrontendError::UnsupportedConstruct
unsupported XSD construct: xs:double at UCI_MessageDefinitions_v2_6_0.xsd:4727:4
```

Both failures are the `type="xs:double"` primitive of the first local sequence
element encountered after declaration-version processing. The releases do not
diverge except in source position. Support for this next primitive blocker is
implemented by the floating-primitive slice below.

## Built-in primitive inventory and floating-point policy

The Task 011 probe recursively inspected both reachable schema documents per
release and resolved every QName-bearing `type`, `base`, `ref`, `itemType`,
`memberTypes`, and `substitutionGroup` value through the namespace bindings at
its use site. The table therefore does not assume the lexical prefix `xs`.
Only QNames resolving to `http://www.w3.org/2001/XMLSchema` are counted.

| Built-in | UCI 2.5 local field | UCI 2.5 restriction base | UCI 2.6 local field | UCI 2.6 restriction base |
|---|---:|---:|---:|---:|
| `boolean` | 409 | 0 | 349 | 0 |
| `byte` | 3 | 0 | 3 | 0 |
| `dateTime` | 4 | 1 | 0 | 1 |
| `double` | 280 | 25 | 280 | 25 |
| `duration` | 9 | 1 | 0 | 1 |
| `float` | 56 | 4 | 56 | 4 |
| `hexBinary` | 5 | 3 | 0 | 4 |
| `int` | 59 | 4 | 61 | 4 |
| `long` | 14 | 0 | 14 | 1 |
| `short` | 4 | 0 | 4 | 0 |
| `string` | 0 | 850 | 0 | 835 |
| `time` | 0 | 1 | 0 | 1 |
| `unsignedByte` | 37 | 17 | 37 | 17 |
| `unsignedInt` | 317 | 3 | 317 | 3 |
| `unsignedShort` | 32 | 13 | 33 | 13 |

No other XSD built-in names occur in the reachable sets. In particular, neither
release references `decimal`, `integer`, `unsignedLong`, binary variants other
than `hexBinary`, `date`, `base64Binary`, `anyURI`, or `QName`. There are no
built-in references in global `xs:element @type`, so none is directly a message
payload. No built-in QName occurs in any observed context other than a local
element `@type` or restriction `@base`.

Task 011 preserves this evidence-backed boundary explicitly. Primitive
expansion applies to local semantic fields and general type references; it does
not make primitive payloads legal UCI messages. A global element carrying UCI
message version metadata must resolve its payload to a named schema type before
it can become a `MessageDecl`.

All counts above are in `UCI_MessageDefinitions_v2_*_0.xsd` except `boolean`
local fields (407 message-definition + 2 security-marking in 2.5; 347 + 2 in
2.6), `dateTime` local fields (2 + 2 in 2.5), and `string` restriction bases
(832 + 18 in 2.5; 817 + 18 in 2.6). The remaining rows have no
security-marking references. The releases differ in several non-floating totals,
but their floating profiles are identical: 60 `float` and 305 `double`
references each, split identically between direct fields and restriction bases.

The first `double` field in both releases is `AnAn` in
`AccelerationAccelerationCovarianceType`: required exactly once, non-nillable,
and documented as a North-North acceleration covariance. It is a direct local
element `@type` at line 4703 in 2.5 and line 4727 in 2.6. The same covariance
record immediately repeats the direct `double` pattern for five related
components, with later components optional. The first direct `float` field is
the optional, non-nillable `NorthSouthVelocity` in
`ADS_B_KinematicsContributionType` (line 7939 in 2.5; 7971 in 2.6). Floating
restriction bases also occur, first on `AccelerationType` for `double` and
`IFF_BarometricPressureType` for `float`.

This is not an isolated `double` use: both binary floating widths materially
occur in equivalent structural roles. The IR and direct-field QName resolver
therefore add the coherent pair:

```text
xs:float  -> PrimitiveKind::Float32
xs:double -> PrimitiveKind::Float64
```

These are binary floating-point value spaces and remain distinct from
`PrimitiveKind::Decimal`, which denotes decimal arithmetic. No integer range is
attached to either floating kind, and the model does not erase possible future
lexical values such as `NaN`, `INF`, `-INF`, or negative zero. Floating
restriction facets remain fail-closed rather than being approximated.

After direct `float` and `double` support, both authoritative roots advance to
the same next distinct blocker:

```text
UCI 2.5: FrontendError::UnsupportedConstruct
unsupported XSD construct: xs:choice at UCI_MessageDefinitions_v2_5_0.xsd:4739:3

UCI 2.6: FrontendError::UnsupportedConstruct
unsupported XSD construct: xs:choice at UCI_MessageDefinitions_v2_6_0.xsd:4763:3
```

Task 011 stops at this blocker without adding choice support.

## Task 012: choice, scalar, integer-range, and unbounded tranche

Both complete reachable schema sets (the message-definition root plus its
security-marking include) were inventoried before implementation. UCI 2.5 has
420 `xs:choice` nodes: 416 in message definitions and 4 in security markings.
UCI 2.6 has 424: 420 and 4 respectively. In 2.5, 419 choices are the sole
semantic child of a named `xs:complexType`; in 2.6, 423 are. The one remaining
choice in each release is under `xs:extension`/`xs:complexContent` and remains
outside this tranche. No choices occur inside sequences, other choices, or
groups.

Every choice has no attributes and therefore default 1..1 group cardinality.
No choice group is unbounded, and no choice-level annotation occurs. All 1,468
UCI 2.5 alternatives and all 1,482 UCI 2.6 alternatives are source-ordered local
`xs:element` children with required `name` and `type`; none uses `ref`, an
anonymous type, nested sequence/choice, or `xs:any`. Every alternative has one
documentation annotation. Alternative types split 1,377 named/91 primitive in
2.5 and 1,393 named/89 primitive in 2.6.

| Alternative attributes | UCI 2.5 | UCI 2.6 |
|---|---:|---:|
| `name type` | 1,375 | 1,388 |
| `name type maxOccurs` | 89 | 90 |
| `name type minOccurs maxOccurs` | 4 | 4 |

Choice sizes are 2 alternatives (238/237 choices in 2.5/2.6), 3 (68/72), 4
(40/41), 5 (22/22), 6 (15/15), 7 (7/7), 8 (6/6), 9 (7/7), 10 (3/3), 11
(3/3), 12 (1/1), 13 (2/2), 15 (1/1), 18 (1/1), 20 (1/1), 21 (1/1), 23
(3/3), and 24 (1/1).

The frontend maps the 419/423 supported named complex types to
`TypeKind::Choice` without changing the IR shape or flattening mutual
exclusivity into records. Non-default synthetic choice-group cardinality remains
unsupported. Sequence fields and choice alternatives share one local-element
parser, preserving documentation, resolved QName, occurrence bounds,
nillability, intrinsic primitive constraints, and source provenance.

Direct scalar mappings added by this tranche are `boolean -> Boolean`;
`byte`/`short`/`int`/`long -> SignedInteger`;
`unsignedByte`/`unsignedShort`/`unsignedInt -> UnsignedInteger`;
`dateTime -> DateTime`; and `hexBinary -> Binary`. Existing mappings for
`integer`, `string`, `float`, and `double` remain. Prefix spelling is irrelevant;
the XML Schema namespace URI controls resolution. `duration` and `time` are not
mapped to `DateTime`.

Fixed-width integer fields and restriction bases retain exact intrinsic ranges
in `ConstraintSet`: byte -128..127, short -32768..32767, int
-2147483648..2147483647, long -9223372036854775808..9223372036854775807,
unsignedByte 0..255, unsignedShort 0..65535, and unsignedInt 0..4294967295.
Explicit inclusive/exclusive range facets are intersected with intrinsic bounds
without converting exclusivity through arithmetic; contradictory intersections
fail semantic validation.

Integer restriction-base counts are 17 `unsignedByte`, 13 `unsignedShort`, 4
`int`, and 3 `unsignedInt` in 2.5. UCI 2.6 has the same counts plus one `long`.

| Integer restriction facet | UCI 2.5 | UCI 2.6 |
|---|---:|---:|
| `minInclusive` | 10 | 11 |
| `maxInclusive` | 30 | 30 |
| `minExclusive` | 0 | 0 |
| `maxExclusive` | 0 | 0 |
| `pattern` | 1 | 0 |
| all other facets | 0 | 0 |

Numeric range facets are modeled coherently, including synthetic exclusive
coverage. The one 2.5 `pattern` on an `int` restriction remains fail-closed
because pattern-only scalar semantics need a different model. Floating,
`dateTime`, and `hexBinary` restrictions remain outside this tranche.

`maxOccurs="unbounded"` is material: UCI 2.5 has 1,933 uses (1,927 message
definitions, 6 security markings), comprising 1,849 sequence fields and 84
choice alternatives. UCI 2.6 has 1,952 (1,946 and 6), comprising 1,867 sequence
fields and 85 choice alternatives. Neither has an unbounded choice group.
Minimum occurrences across those uses are 0/1/2/3: 1,390/531/9/3 in 2.5 and
1,406/534/9/3 in 2.6. They normalize to `max_occurs = None`; no finite maximum
is invented.

The larger probe loop passed the previous `xs:choice` blocker and the direct
scalar and unbounded field uses encountered before the next structural boundary.
Both releases then stop outside Task 012:

```text
UCI 2.5: unsupported XSD construct: xs:complexContent at UCI_MessageDefinitions_v2_5_0.xsd:4822:3
UCI 2.6: unsupported XSD construct: xs:complexContent at UCI_MessageDefinitions_v2_6_0.xsd:4846:3
```

Task 012 stops there without implementing extension/inheritance. UCI 2.6 has
four additional choices, 14 additional alternatives, 19 additional unbounded
fields, and one additional `long` restriction; its integer restrictions omit
the 2.5 pattern facet. Primitive global message payloads remain invalid because
`MessageDecl.payload_type` must be named. Frontend semantic recognition also
does not claim backend support: existing generators explicitly reject choices
and unsupported scalar/container semantics rather than lowering them lossily.

## Complex-type inheritance inventory and policy

Task 013 recursively inventoried each message-definition root and its reachable
security-marking include before implementation. All observed `xs:complexContent`
nodes are in the message-definition document; none occur in the included
security-marking document.

| Inheritance property | UCI 2.5 | UCI 2.6 |
|---|---:|---:|
| `xs:complexContent` | 2,026 | 2,036 |
| `xs:extension` | 2,026 | 2,036 |
| `complexContent/xs:restriction` | 0 | 0 |
| extension-local sequence | 1,619 | 1,630 |
| extension-local choice | 1 | 1 |
| extension with no local compositor | 406 | 405 |
| inheritance edges | 2,026 | 2,036 |
| distinct referenced base types | 271 | 272 |
| roots referenced by derived declarations | 197 | 198 |
| bases declared before/after child | 1,134 / 892 | 1,142 / 894 |
| bases that are themselves derived | 331 | 334 |
| maximum inheritance depth (edges) | 5 | 5 |
| cycles / unresolved bases | 0 / 0 | 0 / 0 |
| inherited-name redeclarations | 0 | 0 |

The longest representative chain is identical in both releases:
`AirRecordMDT -> AirRecordDRL -> AirRecordDRLE -> RecordDRLE ->
DataRecordListBaseType -> DataRecordBaseType`. Multiple independent
hierarchies are present. Every base resolves to a named `xs:complexType` in the
UCI namespace. There are no cross-file or cross-namespace inheritance edges.

All complex-content parents have `name` and UCI `version`; the 18 abstract
derived parents per release additionally have `abstract="true"`. Every
`complexContent` has no attributes, exactly one extension child, no `mixed`,
and no annotation. Every extension has only its required `base` attribute and
has no annotation. No extension has attributes, attribute groups, groups,
children after its compositor, nested compositors, `xs:any`, or anonymous local
types. The sole local choice in each release is `QueryType` extending
`QueryPET`. Consequently the supported grammar deliberately rejects all
unobserved complex-content attributes and children; complex restriction is not
treated as extension.

Each release contains 70 abstract complex types, all using lexical value
`true` (no `false`, `1`, or `0` in real UCI). Eighteen abstract types directly
extend another type and 18 have direct sequence content; the remaining direct
content is empty. Fifty-seven abstract types participate in an observed
inheritance edge and 13 currently have no derived child. Concrete declarations
extend abstract bases 1,254 times in 2.5 and 1,257 times in 2.6. No global UCI
message directly references an abstract type. All 722/725 global messages in
2.5/2.6 reference concrete types that themselves participate in inheritance.

The frontend preserves XSD boolean forms `true`/`1` and `false`/`0` in
`TypeDecl.is_abstract` and rejects malformed values. It normalizes each
extension as an immediate named `base_type` plus only its locally declared
`Record` or `Choice` content. Empty extension content is an empty local record,
meaning no locally declared content. It never copies inherited fields or
alternatives into the derived kind. Type and field documentation remain owned
by their corresponding declarations; the observed intermediate inheritance
syntax contains no documentation to discard.

Semantic IR validation requires structural bases to be named structural
declarations and rejects deterministic self or multi-node inheritance cycles.
This check is limited to base edges and does not conflate field-reference
recursion with inheritance. Primitive `base_type` ancestry remains valid for
named simple restrictions. The shared dependency planner already observes base
references and is tested with forward and multi-level source order. Ada, Rust,
and C++ explicitly reject inherited records and choices before generation, so
no backend can silently emit only local content; their supported simple
restriction ancestry remains unaffected.

The iterative probes passed `xs:complexContent`, `xs:extension`, named complex
base resolution, abstract metadata, local sequence/choice content, empty local
extensions, forward bases, and multi-level chains. They then reached different
outside-tranche blockers:

```text
UCI 2.5: unsupported XSD construct: xs:duration at
UCI_MessageDefinitions_v2_5_0.xsd:39334:4

UCI 2.6: unsupported XSD construct: xs:restriction base type at
UCI_MessageDefinitions_v2_6_0.xsd:110193:3
```

The 2.5 blocker is the `IntegratorStepSize` field with type `xs:duration`. The
2.6 blocker is `AA_CodeType`, a simple restriction of `xs:hexBinary` with an
`xs:length` facet. Both belong to later scalar/restriction work, not complex
inheritance, and are intentionally not implemented here. The inheritance
shapes are materially the same between releases; 2.6 adds ten extension edges,
11 sequence extensions, one distinct base, and removes one empty extension.

## Task 014 scalar restrictions

The complete recursively reachable message-definition sets (message schema plus
security-marking include) contain 945 named simple restrictions in UCI 2.5 and
934 in UCI 2.6. Every restriction has a declaration name and owner annotation.
Six 2.5 restriction nodes and no 2.6 restriction nodes have their own
annotations. Resolved-base counts are:

| Resolved base | UCI 2.5 | UCI 2.6 |
|---|---:|---:|
| `xs:string` | 850 | 835 |
| `xs:double` / `xs:float` | 25 / 4 | 25 / 4 |
| `xs:int` / `xs:long` | 4 / 0 | 4 / 1 |
| `xs:unsignedByte` / `xs:unsignedInt` / `xs:unsignedShort` | 17 / 3 / 13 | 17 / 3 / 13 |
| `xs:hexBinary` | 3 | 4 |
| `xs:dateTime` / `xs:time` / `xs:duration` | 1 / 1 / 1 | 1 / 1 / 1 |
| `uci:BytePositiveType` / `uci:DecibelType` | 4 / 1 | 4 / 1 |
| `uci:DoubleNonNegativeType` / `uci:DoublePositiveType` | 13 / 1 | 13 / 1 |
| `uci:HexBinaryType` | 0 | 1 |
| `uci:IntPositiveType` / `uci:LongNonNegativeType` | 1 / 0 | 1 / 1 |
| `uci:ShortPositiveType` / `uci:VisibleString32Type` | 2 / 1 | 2 / 1 |

Complete facet totals by owner base are:

| Base / facet | UCI 2.5 | UCI 2.6 |
|---|---:|---:|
| string enumeration | 7,766 | 7,767 |
| string length / minLength / maxLength | 65 / 57 / 60 | 65 / 57 / 60 |
| string pattern / whiteSpace | 140 / 2 | 140 / 0 |
| hexBinary length | 3 | 3 |
| dateTime pattern / time pattern | 1 / 1 | 1 / 1 |
| int minInclusive / maxInclusive / pattern | 4 / 4 / 1 | 4 / 4 / 0 |
| long minInclusive | 0 | 1 |
| unsignedByte minInclusive / maxInclusive | 4 / 16 | 4 / 16 |
| unsignedInt minInclusive / maxInclusive | 1 / 1 | 1 / 1 |
| unsignedShort minInclusive / maxInclusive | 1 / 9 | 1 / 9 |
| float minInclusive / maxInclusive | 4 / 3 | 4 / 3 |
| double minInclusive / maxInclusive / minExclusive | 10 / 7 / 1 | 10 / 7 / 1 |
| named-base minInclusive / maxInclusive / length | 1 / 9 / 0 | 1 / 9 / 1 |

No `totalDigits`, `fractionDigits`, or other restriction facets occur. Exact
built-in facet-set shapes are identical except where noted: temporal is
`dateTime(pattern)`, `time(pattern)`, and `duration(zero)`; floating is
`float(minInclusive)` or `float(minInclusive,maxInclusive)`, and `double(zero)`,
`double(minExclusive)`, `double(minInclusive)`, or
`double(minInclusive,maxInclusive)`; binary is `hexBinary(length)`, plus one
`hexBinary(zero)` in 2.6; signed integers use range combinations, with one 2.5
`int(minInclusive,maxInclusive,pattern)` and one 2.6 `long(minInclusive)`;
unsigned integers use zero, one-sided, or inclusive-range shapes.

String restrictions comprise enumeration-only declarations and these exact
non-enumeration ordered shapes: `length+pattern` (59),
`length+pattern×2` (4), `length+pattern×5` (1),
`length+pattern×8` (1), `maxLength+minLength+pattern` (7),
`maxLength+pattern` (3), and `minLength+maxLength+pattern` (48 in 2.5,
50 in 2.6). UCI 2.5 additionally has two
`minLength+maxLength+whiteSpace+pattern` declarations, which remain
unsupported. Enumeration-only multiplicities in 2.5 are
`1:5, 2:141, 3:136, 4:107, 5:62, 6:59, 7:23, 8:31, 9:14, 10:15, 11:20,
12:10, 13:7, 14:6, 15:4, 16:8, 17:7, 18:3, 19:7, 20:5, 21:4, 22:4,
23:1, 24:5, 25:1, 27:1, 28:2, 29:1, 30:1, 31:5, 32:4, 33:1, 35:3,
36:1, 38:1, 39:1, 41:1, 43:1, 47:1, 48:1, 54:1, 57:1, 61:2, 62:1,
64:1, 83:1, 92:1, 93:1, 280:1, 340:3, 341:1, 722:1`.
The 2.6 multiplicities are
`1:2, 2:139, 3:134, 4:102, 5:61, 6:58, 7:21, 8:31, 9:14, 10:15,
11:18, 12:10, 13:8, 14:6, 15:4, 16:9, 17:7, 18:3, 19:7, 20:5, 21:4,
22:4, 23:1, 24:5, 25:1, 27:1, 28:2, 29:1, 30:1, 31:5, 32:4, 33:1,
35:3, 36:1, 38:1, 39:1, 41:1, 43:1, 47:1, 48:1, 49:1, 54:1, 57:1,
61:2, 62:1, 64:1, 83:1, 92:1, 93:1, 280:1, 340:3, 341:1, 725:1`.

Named UCI restriction bases are present: 23 declarations in 2.5 and 25 in
2.6. Their exact shapes are `BytePositiveType(maxInclusive)` (4),
`DecibelType(minInclusive)` (1), `DoubleNonNegativeType(zero)` (11),
`DoubleNonNegativeType(maxInclusive)` (2), `DoublePositiveType(zero)` (1),
`IntPositiveType(maxInclusive)` (1), `ShortPositiveType(maxInclusive)` (2),
and `VisibleString32Type(zero)` (1). UCI 2.6 additionally has
`HexBinaryType(length)` (1) and `LongNonNegativeType(zero)` (1). Supporting
these requires named restriction constraint inheritance/intersection and is
outside this tranche.

Direct temporal references reconfirm earlier evidence: UCI 2.5 has four
`xs:dateTime`, nine `xs:duration`, and zero `xs:time` fields; UCI 2.6 has none.
Each release has one restriction of each temporal base. `DurationType` has zero
facets and normalizes safely. `DateTimeType` and `TimeType` each use lexical
pattern `.+Z`; those restrictions remain fail-closed because lexical timezone
spelling does not survive semantic normalization. Direct fields map to distinct
`DateTime`, `Time`, and `Duration` primitive kinds without selecting runtime
representations.

The frontend now parses supported built-in restrictions through one
base-sensitive restriction path. String enumerations retain source order,
documentation, and wire values. Semantic string length and same-restriction
pattern alternatives are preserved in source order; duplicate singular facets
and malformed non-negative length values fail deterministically. `hexBinary`
exact/minimum/maximum length
constraints are represented in octets, not lexical hexadecimal characters.
Binary, integer, and temporal lexical patterns remain unsupported. Zero-facet
built-in aliases, including observed floating aliases, normalize without new
constraints; floating ranges remain unsupported. All named restrictions retain
their immediate primitive `base_type`.

Ada, Rust, and C++ still reject `Time`, `Duration`, constrained `String`, and
constrained `Binary` before rendering. This prevents partial output and silent
constraint loss; backend generation support was intentionally not added.

**Task 025 update:** unconstrained `Binary` (`ConstraintSet::default()`) is now
baseline-supported by all three backends as an owned octet sequence — Rust
`Vec<u8>`, C++ `std::vector<std::uint8_t>`, and Ada
`Interfaces.Unsigned_8`-element `Ada.Containers.Vectors.Vector`. `Binary`
length/minLength/maxLength constraints remain unsupported and fail closed
exactly as documented above; the constraint units reiterated above (octets,
never hexadecimal lexical characters) remain the authoritative semantics for
any future constrained-Binary tranche.

The iterative progression was:

```text
UCI 2.5:
  xs:duration direct field at 39334:4
  -> passed direct duration and supported scalar restrictions
  -> STOP: xs:double minInclusive in AltitudeBarometricType at 111189:4

UCI 2.6:
  xs:hexBinary + xs:length in AA_CodeType at 110193:3
  -> passed binary length and supported scalar restrictions
  -> STOP: xs:double minInclusive in AltitudeBarometricType at 111578:4
```

Both final blockers require floating-point constraint semantics not representable
by the integer-valued `ConstraintSet` range fields. They are outside Task 014;
no approximation or Task 015 implementation was added. The releases began this
tranche at divergent blockers and converged on the same semantic blocker at
release-specific source locations.

More precisely, both blockers are the semantically equivalent
`AltitudeBarometricType` declaration. Each restricts the resolved base
`{http://www.w3.org/2001/XMLSchema}double` with exactly one facet,
`minInclusive="-6378237"`, after documentation describing barometric altitude
in meters and the EGM96 geoid-derived minimum. The restriction begins at line
111,189 in 2.5 and line 111,578 in 2.6 according to the XML parser's source
position; the unsupported facet diagnostic is at column 4 on those lines. Each
authoritative schema set contains three declarations
with the exact `xs:double + minInclusive` shape: `AltitudeBarometricType`,
`AltitudeType` (also `-6378237`), and `DoubleNonNegativeType` (`0.0`).

Representing these bounds faithfully requires a language-neutral
floating-point constraint model that preserves XSD floating values and ordering
without parsing into `i128`, rounding, truncating, mapping to decimal, or hiding
the bound in an opaque string. That is the recommended next-task semantic
problem and is intentionally not implemented in Task 014.

## Task 015 floating and named-restriction evidence

The complete reachable two-document sets were inventoried before implementation.
Each release has the same direct built-in floating range-facet counts:

| Primitive | minInclusive | maxInclusive | minExclusive | maxExclusive |
|---|---:|---:|---:|---:|
| `xs:float` / Float32 | 4 | 3 | 0 | 0 |
| `xs:double` / Float64 | 10 | 7 | 1 | 0 |

The Float32 bound lexicals are `-1.0`, `0.0`, `1.0`, `800`, and `1210`. The
Float64 lexicals, including local facets on named floating restrictions, are
`-6378237`, `-3.141592653589793238462`, `-1.570796326794896619232`, `-1`, `0`,
`0.0`, `1`, `1.0`, `1.570796326794896619232`, `3.141592653589793238462`,
`6.283185307179586476926`, `9`, and `400`. Neither release uses exponent
notation, an explicit positive sign, negative zero, `INF`, `-INF`, or `NaN` in
floating range facets.

Constrained Float32 declarations are `IFF_BarometricPressureType`,
`SpoilFactorType`, `UnitBallFloatType`, and `UnitIntervalFloatType`. Directly
constrained Float64 declarations are `AltitudeBarometricType`, `AltitudeType`,
the five angle range types, `DoubleNonNegativeType`, `DoublePositiveType`,
`UnitBallDoubleType`, and `UnitIntervalDoubleType`. Named chains add
`DecibelNonNegativeType(minInclusive=0)`,
`GeomagneticApIndexType(maxInclusive=400)`, and
`GeomagneticKpIndexType(maxInclusive=9)`.

UCI 2.5 has 23 named bases: 15 ultimately Float64, seven UnsignedInteger, and
one String; six are forward references. UCI 2.6 has 25: the same distribution
plus one SignedInteger (`BytesType -> LongNonNegativeType`) and one Binary
(`IFF_RegisterType -> HexBinaryType`), with seven forward references. Maximum
depth including the primitive-root level is two. Other local named-chain shapes
are integer `maxInclusive`, binary `length`, or zero facets. Neither graph has
cycles, unresolved bases, named enumeration bases, family changes, inherited
base patterns, or derived patterns.

Task 015 stores floating bounds as width-specific IEEE semantic values. Pending
named restrictions resolve after all documents load while preserving immediate
`base_type`, ultimate primitive `kind`, declaration order, and effective numeric
and length intersections. Contradictions, mixed domains, cycles, structural
bases, non-finite ranges, and pattern-on-pattern inheritance fail closed.

At Task 015 the backends still rejected every constrained floating declaration
before rendering. That backend limitation was lifted in Task 033: named
`Float32`/`Float64` declarations whose effective constraints are numeric range
facets only are now backend-renderable, and the effective bounds Task 015
resolves here are exactly what the backends consume — they do not re-walk raw
XSD restriction chains. Because every constrained floating declaration in both
pinned releases is bound-only (zero lexical and zero length facets on floating
types), this covers the authoritative floating tranche in full.

Nothing about frontend behaviour changed in Task 033: the parsing, storage,
inheritance resolution, and fail-closed rules recorded above are Task 015's and
remain byte-for-byte in force.

The independent iterative probe progression was:

```text
UCI 2.5:
  AltitudeBarometricType xs:double minInclusive at 111189:4
  -> passed all floating ranges and named simple-restriction chains
  -> STOP: DateTimeType xs:dateTime pattern=".+Z" at 117043:4

UCI 2.6:
  AltitudeBarometricType xs:double minInclusive at 111578:4
  -> passed all floating ranges and named simple-restriction chains,
     including HexBinaryType length and LongNonNegativeType inheritance
  -> STOP: TimeType xs:time pattern=".+Z" at 145408:4
```

Both final blockers are temporal lexical restrictions with no surrounding
facets. They require timezone/runtime lexical semantics outside Task 015; the
frontend intentionally neither preserves them as value-space regexes nor starts
Task 016. The release divergence is the first temporal owner reached:
`DateTimeType` in 2.5 versus `TimeType` in 2.6.

## Task 016 lexical restriction evidence

The complete reachable two-document sets were inventoried again before coding.
Pattern counts by ultimate primitive are:

| Primitive | UCI 2.5 | UCI 2.6 |
|---|---:|---:|
| String | 140 | 140 |
| SignedInteger | 1 | 0 |
| DateTime | 1 | 1 |
| Time | 1 | 1 |
| UnsignedInteger, Binary, Duration, Float32, Float64, other | 0 | 0 |
| **Total** | **143** | **142** |

There are 128 patterned owners in 2.5 and 127 in 2.6. In each release the
restriction-level group sizes are 122/121 owners with one pattern, four with
two, one with five, and one with eight. All pattern restrictions directly use a
built-in base (restriction-chain depth one); no authoritative named chain has a
patterned base. The only pattern-facet annotation is the supported leading
documentation annotation on `NATO_SpecialWordsType` in each security-marking
document; it has no appinfo or markup and the exact expression
`NATO:[a-zA-Z\-_]{1,256}`.

Both temporal declarations use exactly `.+Z` and no neighboring facets.
`DateTimeType` is at message-definition line 117,043 in 2.5 and security-marking
line 1,208 in 2.6; `TimeType` is at message-definition line 145,157 in 2.5 and
145,408 in 2.6. Each owner documentation says the W3C primitive is retained
with the further restriction that only the “Zulu” timezone be used. The IR
preserves the XML Schema expression lexically; it does not turn `Z` into an
instant, offset, parser, or value-space range.

The sole integer case is UCI 2.5
`USMTF_SerialNumberOfQualifierType` at message-definition line 145,862. It
restricts `xs:int` with `minInclusive=1`, `maxInclusive=999`, and
`pattern="[0-9]{1,3}"`; owner documentation identifies a serial number from
MIL-STD-6040. The pattern constrains spelling independently of the numeric
interval (for example signs and leading-zero spellings), so both constraints are
retained. The 2.6 declaration is version `000.001.000.000` and keeps the same
numeric bounds but removes the pattern.

UCI 2.5 has exactly two `whiteSpace` facets and 2.6 has none. The 2.5 owners are
`WhitespaceVisibleString1024Type` and `WhitespaceVisibleString4096Type` in the
security-marking schema at lines 8,572 and 8,583. Both directly restrict
`xs:string`, use `collapse`, and combine `minLength=0`, the corresponding
`maxLength`, and `[ -~\n\r]{0,N}`. They have equivalent structure, no facet
annotation, and no named-base participation. No other primitive family has a
`whiteSpace` facet. The equivalent 2.6 declarations omit `whiteSpace` and use
pattern lower bounds of one.

The authoritative W3C XML Schema Datatypes rules were checked before design.
Multiple pattern elements in one restriction contribute alternatives to one
pattern facet (a value must match at least one); pattern facets accumulated over
successive restrictions all apply. The IR therefore stores one source-ordered
`PatternGroup` per restriction level and appends local groups after inherited
groups. `PatternExpression` explicitly records the `XmlSchema` dialect. Named
effective constraints retain the immediate named base, and IR validation
requires the base groups to be an exact inherited prefix.

W3C whitespace processing defines `preserve`, `replace`, and `collapse`, is
performed before other facets, and permits derivation only in the tightening
order `preserve < replace < collapse`. The typed `WhiteSpacePolicy` is effective
on named restrictions; frontend and IR validation reject weakening. Whitespace,
length, and pattern constraints are all retained simultaneously. Task 016 only
enables whitespace on the observed String family and patterns on observed
String, SignedInteger, DateTime, and Time families.

A Task 016 corrective pass made the primitive intrinsic policy explicit in the
IR contract without materializing it into every `ConstraintSet`: String starts
at restrictable `preserve`, while the other represented primitives use fixed
`collapse`. An absent explicit facet still inherits that baseline. This did not
change the normalized UCI 2.5 or 2.6 counts or results below.

Ada, Rust, and C++ have no XML Schema regex or whitespace runtime. Each backend
rejects any declaration carrying lexical constraints before rendering, including
String, SignedInteger, DateTime, and Time patterns and String whitespace. No
regex crate, translation, generated validator, temporal parser, serializer, or
timezone representation was added.

The optimized independent progression was:

```text
UCI 2.5:
  DateTimeType pattern=".+Z" at 117043:4
  -> TimeType pattern=".+Z" at 145157:4
  -> USMTF_SerialNumberOfQualifierType pattern="[0-9]{1,3}" at 145862:4
  -> both WhitespaceVisibleString* collapse facets in the included schema
  -> schema valid: 1 namespace, 5,557 types, 722 messages

UCI 2.6:
  TimeType pattern=".+Z" at 145408:4
  -> DateTimeType pattern=".+Z" in the included schema at 1208:4
  -> schema valid: 1 namespace, 5,570 types, 725 messages
```

Neither release has a final outside-Task-016 blocker: both complete reachable
schema sets normalize. Release divergence is limited to encounter order, the
2.5-only integer pattern and whitespace facets, and declaration locations/counts.
Full frontend normalization does not imply backend generation support; Task 017
and runtime lexical enforcement remain intentionally unstarted.
