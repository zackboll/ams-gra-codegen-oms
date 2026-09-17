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

## 2026-09-17 probe

Before this task, both roots stopped at line 2 on
`xs:schema @elementFormDefault="qualified"`. The frontend now validates the XSD
enumeration values `qualified` and `unqualified`. This setting controls the
namespace qualification of locally declared elements in XML instances. The
normalized IR models local field wire names and targets language-native UCI JSON
types rather than XML instance serialization, so the validated setting is
deliberately discarded and produces no backend-visible IR change.

Unsupported-construct diagnostics now identify the source file and deterministic
`roxmltree` line and column. Re-running both roots advances to the next distinct
blocker:

```text
unsupported XSD construct: xs:schema @attributeFormDefault at <root>:2:1
```

In both releases the value is `unqualified`. That feature remains unsupported
and is intentionally left for the next compatibility task. No version-specific
difference was observed through this point; UCI 2.5 remains the Sleet
interoperability baseline, while UCI 2.6 is the forward-compatibility target.