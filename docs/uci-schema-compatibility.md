# UCI Schema Compatibility

Tested: **2026-09-17**.

This is an evidence log for frontend development, not a claim of full UCI
compatibility. UCI schema files are obtained and exercised externally; they are
not vendored in this repository.

## Public source

Both releases were obtained from the authoritative public [UCI Standard
repository](https://gitlab.com/open-arsenal/uci/standard):

| Target | Tag | Commit | Root schema | Required dependency | Fully normalizes? |
|---|---|---|---|---|---|
| UCI 2.5 / Sleet baseline | `v2.5` | `093610b7753944059360d3236770ab446d039556` | `UCI_MessageDefinitions_v2_5_0.xsd` | `UCI_SecurityMarkings_v2_5_0.xsd` via `xs:include` | No |
| UCI 2.6 / current public release | `v2.6` | `78eb61b6112c8bffa40820c33124b57787fc5bd9` | `UCI_MessageDefinitions_v2_6_0.xsd` | `UCI_SecurityMarkings_v2_6_0.xsd` via `xs:include` | No |

The separate `UCI_Versioning_v2_*_0.xsd` documents import the corresponding
message-definition schema. The compatibility probe used the message-definition
file as the root because it is the UCI message model consumed by this project.

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
documentation field. The observed pattern facet remains unsupported in its own
right; Task 008 does not bypass that owner to consume its annotation. No raw XML
metadata is exposed to backends.

Because real UCI provides no evidence that `xs:appinfo` is safe to ignore, it
remains unsupported. Embedded markup, unknown annotation children, unexpected
annotation attributes, and annotations outside the leading position also fail
closed with source context.

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

The frontend supports only this observed global shape. It resolves message and
payload QNames in the source document, preserves source provenance, and retains
schema-set discovery order followed by element source order. The observed
UCI/OAM `version` attribute is accepted and discarded because declaration
version history has no current semantic IR consumer; any other attribute or an
anonymous type remains an explicit unsupported construct. Message identities
must be unique, payload references must resolve after the complete schema set is
assembled, and type and message symbol spaces remain distinct.

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
Support for the next blocker is intentionally outside this slice.
