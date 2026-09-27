# Task 051 — Preserve OMS JSON member QNames in Schema IR

```text
Task 050:  generated Rust OMS JSON codecs, OAM namespace only
Task 051:  local element wire QNames preserved; qualified non-OAM codecs
```

Task 050 restricted generated codecs to the OAM namespace because
normalized Schema IR lost the namespace of LOCAL element declarations: the
frontend validated `elementFormDefault` and then discarded it, and rejected
local `@form`. OMSC-SPC-013 Rev B §6.1.2 names a particle member from the
**element declaration's** target namespace, so even an ordinary
single-namespace extension schema could not get a correct codec.

Task 051 closes that semantic gap. It is **wire provenance**, not a model API
change: host-language model identifiers, service API wrappers, and every
OAM codec are byte-identical to `main` (§6).

## 1. Evidence gate (recorded before production changes)

### 1.1 XML Schema 1.0: local element target namespace

Source: *XML Schema Part 1: Structures Second Edition*, W3C Recommendation
28 October 2004,
<https://www.w3.org/TR/2004/REC-xmlschema-1-20041028/structures.html>
(retrieved 2026-09-27; SHA-256 of the fetched HTML
`92bfd116316332b1153941436430afbdb2040e036d5fb2d21d4c8fba36a7765a`).

| # | Claim | Exact source text |
| --- | --- | --- |
| 1 | `elementFormDefault` defaults to `unqualified` | §3.15.2 *XML Representations of Schemas*, `<schema>` summary: `elementFormDefault = (qualified \| unqualified) : unqualified` |
| 2 | a local element may override it with `form` | §3.3.2 *XML Representation of Element Declaration Schema Components*, `<element>` summary: `form = (qualified \| unqualified)` (no default) |
| 3 | qualified local declaration → the enclosing schema's `targetNamespace` | §3.3.2, local element `{target namespace}`: "If `form` is present and its actual value is `qualified`, or if `form` is absent and the actual value of `elementFormDefault` on the `<schema>` ancestor is `qualified`, then the actual value of the `targetNamespace` [attribute] of the parent `<schema>` element information item, or *absent* if there is none, otherwise *absent*." |
| 4 | unqualified local declaration → *absent* | the same sentence's final clause: "otherwise *absent*" |

§3.1.1 defines *absent* as "a distinguished property value denoting
absence", distinct from any string. The top-level element case in §3.3.2 is
different: its `{target namespace}` is always the schema's
`targetNamespace`. That is `MessageDecl.name`, unchanged here.

The `<schema>` ancestor is the document that **declares** the element: an
included document's own `elementFormDefault` governs its own declarations,
and an imported document is a separate schema document with its own
`targetNamespace` and default.

### 1.2 OMS JSON particle member names

Source: `open-arsenal/oms` @ `726272bd0390982a759c91a9cf4e13b81c2b510b`,
OMSC-SPC-013 Rev B. The official
`docs_official/20_OMSC-SPC-013_RevB_LanguageAgnostic_CAL_Specification_DandD_v2_5.docx`
(SHA-256 `b1c3c07872570fb4f2c84fd819076b1b588148ef28225f5a8e23433efe94b1a7`)
was read from `word/document.xml`; it agrees verbatim with the unofficial
Markdown rendering (SHA-256
`5798da43991d5b554ed7575fa55c350aacece483d36841738e57db123f4ee98f`).

§6.1.2 *Particles*, element-declaration case:

> The string is given by the appropriate case among the following:
> 1. If {target namespace} is https://www.vdl.afrl.af.mil/programs/oam, then "{name}".
> 2. Otherwise "{{target namespace}}{name}".

and, when a type is used where another type was expected:

> the member whose string is "$type" and whose value is … If {complexType
> target namespace} is https://www.vdl.afrl.af.mil/programs/oam, then
> "{complexType name}." Otherwise "{{complexType target namespace}}{complexType name}."

The spec's glossary links `{target namespace}` to
<https://www.w3.org/TR/xmlschema-1/#e-target_namespace>, the XML Schema
element-declaration property of §1.1.

### 1.3 Unqualified members: no evidenced OMS JSON spelling

Both the official `.docx` text and the Markdown were searched for `absent`,
`unqualified`, `elementFormDefault`, `no namespace`, `no target namespace`,
and `{}`. **No rule** addresses an element declaration whose target
namespace is absent; the only `absent` is Table 6.1-1's `[base URI]` row.

Both candidate spellings are contradicted or unsupported:

* **bare `"Field"`**: §6.1.5.2 *Namespace Name and Local Name of a Member*
  reconstructs an unbraced member as namespace
  `https://www.vdl.afrl.af.mil/programs/oam`. A bare key therefore *means*
  OAM, not absent; emitting one would silently change the element's identity.
* **`"{}Field"`**: the "Otherwise" clause interpolates `{target namespace}`,
  but *absent* is not a string, and §6.1.5.2 would then reconstruct an empty
  namespace name. No text says to do this.

Decision: preserve absence faithfully in IR (`None`) and make generated codec
readiness **fail closed** on any stored unqualified member. Absence is never
mapped to OAM, to `{}Field`, or to the owning type's namespace.

## 2. IR representation

```rust
pub struct FieldDecl {
    pub name: String,                       // source local name (host naming)
    pub wire_namespace_uri: Option<String>, // element {target namespace}
    pub type_ref: TypeRef,                  // the element's TYPE (independent)
    // ... cardinality, nillable, constraints, documentation, source
}

pub struct FieldWireName<'a> {
    pub namespace_uri: Option<&'a str>,
    pub local_name: &'a str,
}

impl FieldDecl {
    pub fn wire_name(&self) -> FieldWireName<'_>;
}
```

* `Some(uri)`: a qualified local element in `uri`.
* `None`: an unqualified local element; its target namespace is absent.
  `Some("")` is a different value, and the frontend never produces it.
* `wire_name().local_name` **is** `FieldDecl.name` (borrowed), so no second
  copy of the local name exists to drift.
* The element's own namespace and its `type_ref` namespace are separate
  facts. `<xs:element name="Foreign" type="i:ImportedRecord"
  form="qualified"/>` in `urn:matrix` has wire namespace `urn:matrix` and
  type `{urn:matrix:imported}ImportedRecord`.
* No prefix spelling and no Clark string is stored. Clark notation is a
  codec-side formatting of this semantic pair.

### IR validation

`SchemaIr::validate` adds one rule for Record fields and Choice
alternatives: `Some(uri)` must name a `NamespaceDecl` of the schema set
(`ValidationError::UndeclaredFieldNamespace`). `None` is valid and needs no
declaration.

## 3. Frontend

* `ParsedSchemaDocument` retains its effective `element_form_default`
  (`ElementForm::{Qualified, Unqualified}`, default `Unqualified`).
  `attributeFormDefault` is still only validated.
* A `SchemaDocumentContext { target_namespace, element_form_default }` of
  the **declaring** document is passed to every local element parser:
  `xs:sequence`, `xs:choice`, and `complexContent/extension` sequences and
  choices. The type namespace is never used as a proxy.
* Local `@form` is accepted: `qualified` or `unqualified`; anything else is
  `InvalidInput` (`xs:element @form must be qualified or unqualified, got …
  at L:C`). Effective form = local `@form` if present, else the document
  default.
* Local `xs:element ref=` stays **unsupported**
  (`unsupported XSD construct: xs:element @ref at …`). The attribute
  allow-list rejects it before `name`/`type` are read.

### Qualification matrix (`crates/xsd-frontend/tests/member_qname.rs`)

| Case | Declaring document default | Local `form` | Result |
| --- | --- | --- | --- |
| A | none | none | `None` |
| B | `unqualified` (imported doc) | none | `None` |
| C | `qualified` (included doc) | none | `Some(targetNamespace)` |
| D | none | `qualified` | `Some(targetNamespace)` |
| E | `qualified` | `unqualified` | `None` |
| F | Choice alternatives under both defaults | mixed | same rule as sequence |
| G | extension members, both directions | — | the declaring document's rule |
| H | included doc with its own default | — | the included doc's default, not the root's |
| I | two imported docs with different defaults | — | each its own |

## 4. Codec semantics

`codegen-core` has one semantic formatter each:

```text
oms_json_member_name(namespace, local)   Record keys AND Choice keys
oms_json_type_name(namespace, local)     "$type" values (and a Record's own $type check)
    OAM   -> local
    other -> "{" + namespace + "}" + local
```

* Member keys come from `FieldDecl::wire_name()` of the stored field or
  alternative, never from the owning Record, the message, the payload root,
  or the Rust spelling.
* `$type` comes from the concrete **TypeDecl**'s QName. Decode matches the
  exact same formatted value.
* Generated error paths embed the same key; `{`/`}` are escaped inside the
  generated `format!` literal. A bare OAM key needs no escaping, so OAM
  output is unchanged.
* Readiness: the blanket `NonOamNamespace` rejection is removed. A stored
  member whose wire namespace is `None` fails with
  `ServiceCodecError::UnqualifiedMember`:
  `service codec boundary: Payload.Field has an unqualified local element with
  no evidenced OMS JSON member-name mapping`. Task 026 absent-only members
  have no storage, are never encoded, and are not checked.
* Binary, Decimal/Time/Duration, nillable, optional Choice alternatives,
  alias/list, and Ada/C++: every Task 050 boundary is unchanged.
* The **backend** single-namespace preflight is unchanged:
  `BackendPreflightError::MultipleNamespaces` still rejects a projection
  spanning two namespaces, in every language, with or without `--with-codec`.
* Global message identity is unchanged: the runtime's
  `oms_global_element_name` formats `MessageDecl.name` exactly as in Task 049.

### Exact generated results

| Proof | Result |
| --- | --- |
| Record (`runtime-test.xsd`, `urn:test`) | payload `{"{urn:test}Count":7}`; with the runtime envelope `{"{urn:test}MessageB":{"{urn:test}Count":7}}` |
| Choice (`codec-choice.xsd`, `urn:choice`) | `{"{urn:choice}Pick":{"{urn:choice}Alpha":5}}`; rejects bare `Alpha`, `{urn:other}Alpha`, two alternatives, none, `alpha`, `$type:"Selection"` |
| Inheritance (`codec-inherit.xsd`) | `{"{urn:inherit}BaseValue":3,"{urn:inherit}ExtraValue":"extra"}`, flat; rejects a nested base object and bare `BaseValue` |
| `$type` (`codec-shape.xsd`) | `{"$type":"{urn:shape}BoxShape","{urn:shape}Tag":"t","{urn:shape}Width":4}`; rejects `BoxShape`, `{urn:other}BoxShape`, unknown and abstract types, a missing `$type` (no field-based selection), and a member of the other concrete type |
| Strict Record negatives | bare `Count`; `{urn:other}Count`; OAM-Clark `Count`; unknown `{urn:test}Extra`; missing `{urn:test}Count`; both Clark and bare present; bare owner `$type` |

### Mock OWP (the required runtime proof)

`generated_codec_mock_owp.rs::task051_non_oam_generated_codec_round_trips_through_mock_owp`:
generated typed payload -> generated `service_codec.rs` -> generated publish
façade -> `SleetRuntime<ServiceCodec>` -> pinned sleet-client -> mock OWP.

```text
SUB sub-1 {urn:test}MessageA input-topic
PUB output-topic {"{urn:test}MessageB":{"{urn:test}Count":7}}
MSG sub-1 {"{urn:test}MessageA":{"{urn:test}Count":42}}   -> handler: Count = 42
MSG sub-1 {"{urn:test}MessageA":{"Count":43}}              -> SubscriptionDecodeError
                                                    "SharedPayload: unknown member \"Count\""
```

No handwritten codec is involved. Task 049's `mock_owp.rs` still drives the
same schema with its handwritten codec (`runtime_test`), unchanged.

## 5. Real pinned Sleet: non-OAM compatibility probe

| | |
| --- | --- |
| Sleet | `open-arsenal/ams-gra-hello-world-sk-infra-sleet` @ `e38f61d8ce0d75c8508434a52f2ed77c69cf6a3b`, unmodified |
| Schema | `tests/fixtures/service-generate/runtime-test.xsd` |
| SUB | `SUB sub-1 {urn:test}MessageA input-topic` |
| JSON | `{"{urn:test}MessageB":{"{urn:test}Count":7}}` |
| Result | **rejected**: `-ERR Illegal-Argument` `unknown message name {urn:test}MessageA`, and `-ERR Invalid-Message` `schema validation failed: UnknownGlobalElement { name: "{urn:test}MessageB" }` |

Cause: pinned Sleet's schema parser registers global elements and particle
members by bare `name` (`sleet/src/schema/parser.rs::push_element_start`),
and `validate_oms_json` looks the root key up directly. It never applies the
§6.1.1/§6.1.2 Clark form for a non-OAM target namespace, so it rejects the
spec-correct **global** name before any payload member is examined. This is
a pinned-Sleet implementation limitation, not a codec defect: the generated
codec follows OMSC-SPC-013 and is not changed to match it. The probe
(`generated_codec_sleet_non_oam.rs`) asserts exactly this outcome and prints
`SLEET NON-OAM PROBE: RECORDED`; `scripts/run-real-sleet-test.sh` runs it.
Sleet is not patched. The OAM real-Sleet tests (Task 049 handwritten, Task
050 generated `codec-oam`, real UCI PositionReport) are unchanged and pass.

## 6. Byte-identity evidence

A before/after matrix ran the `main` binary (`7925ba0`) and the Task 051
binary side by side. It covered every committed service fixture pair (111
pre-existing plus the new ones) × {rust, ada, cpp} × {closed-schema,
open-extensions} × {default, `--with-codec`}, running `service-check` and
`service-generate`. It also ran plain `generate` over every committed XSD
fixture plus the pinned UCI 2.5 and UCI 2.6 roots × 3 languages × 2 worlds.
Only each report's echoed output directory was normalized.

| Surface (pre-existing inputs) | Result |
| --- | --- |
| Every default-mode file: every model source (`generate` and `service-generate`), every `service_api.rs` / `.hpp` / `.ads`, every report, stderr, exit code | **7,886 / 7,886 byte-identical** |
| Every Ada/C++ `--with-codec` file | **2,648 / 2,648 byte-identical** (still `not implemented for Ada` / `C++`) |
| Every `service_codec.rs` generated on both sides: codec-oam, runtime-oam, the synthetic position-report, and the **real UCI 2.5 PositionReport**, each in both worlds | **7 / 7 byte-identical** |
| Model file inside `--with-codec` output | byte-identical to default generation |
| Rust `--with-codec` cells | 220 total: **162 byte-identical** (every OAM, Binary, and not-measured case); **58 intentionally changed** |

All 58 changed cells are non-OAM Rust `--with-codec` services that Task 050
rejected only with `declaration {...} is outside the OAM namespace`:

* **56** move from codec NOT READY to `codec status: READY`;
* **2** (`optional-primitive`, both worlds) stay NOT READY. They now report
  their real blocker, `PrimitivePayload.Maybe_Binary is Binary`, which Task
  050's first-failure order had hidden behind the namespace check.

Nothing else changed. All 37 closed-schema Rust `service_codec.rs` files from
the after-matrix (existing fixtures plus the new ones) compile together
against `runtime-rust` under stable and under `cargo +1.95.0`.

## 7. UCI inventories

Pinned UCI 2.5 (tag `v2.5`, `093610b7753944059360d3236770ab446d039556`, root
SHA-256 `ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27`) and
UCI 2.6 (tag `v2.6`, `78eb61b6112c8bffa40820c33124b57787fc5bd9`, root
SHA-256 `af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b`)
declare `elementFormDefault="qualified"` in all three documents and contain
no local `form=` and no local `ref=`. Counts are from the normalized IR.

| | Types | Record fields | Choice alternatives | Qualified | Unqualified | Namespaces | Unexpected |
| --- | --- | --- | --- | --- | --- | --- | --- |
| UCI 2.5 full set | 5,557 | 11,692 | 1,468 | 13,160 | 0 | OAM only | 0 |
| UCI 2.5 PositionReport selected closure | 60 | 95 | 8 | 103 | 0 | OAM only | 0 |
| UCI 2.6 full set | 5,570 | 11,716 | 1,482 | 13,198 | 0 | OAM only | 0 |

"Unexpected" counts members whose wire namespace differs from the owning
type's namespace, or is absent. PositionReport stays model 60/60 READY and
codec 59/59 READY. Its model, `service_api.rs`, and `service_codec.rs` are
byte-identical to `main`; the pinned vector still round-trips; and the real
Sleet round trip still passes. UCI 2.6 was checked for frontend provenance
only, with no codec milestone.

## 8. Tests

| Test | Proves |
| --- | --- |
| `ir` unit (+3) | accessor borrows the one local name; element vs type namespace independent; undeclared wire namespace rejected for Record and Choice; absence valid |
| `xsd-frontend` unit (+2) | parsed document retains its effective default; `attributeFormDefault` is not conflated |
| `xsd-frontend/tests/member_qname.rs` (10) | matrix A–I; no prefix/Clark strings in IR; determinism; local `ref=` rejected; invalid local `form` |
| `codegen-core` `service_codec::tests` (+3, 1 renamed) | formatter rule (only the exact OAM URI is bare); qualified non-OAM ready; unqualified Record/Choice members fail closed |
| `cli/tests/service_codec.rs` (+3, 1 replaced) | runtime-test READY with `{urn:test}Count`; unqualified control; local-form controls A/B; multi-namespace boundary unchanged in all languages |
| `cli/tests/member_qname_projection.rs` (4) | selected projection, effective Record/Choice members (including mixed provenance), abstract-value projection, private overlays |
| `backend-rust/tests/service_codec.rs` (+1, 1 replaced) | direct renderer emits Clark keys/`$type`; unqualified fails closed |
| `schema-docs` browser (+1) | wire member shown separately from name and type |
| `generated_codec_qname.rs` (8) | Record, complete document, strict negatives, Choice, inheritance, `$type` |
| `generated_codec_mock_owp.rs` (+1) | generated non-OAM codec through mock OWP |
| `generated_codec_sleet_non_oam.rs` (1) | real-Sleet compatibility probe (asserted, recorded rejection) |

CI gains a `Member QName provenance (Task 051, must execute)` step with 25
exactly-one-test gates. The Task 050 gate
`task050_non_oam_payload_fails_codec_preflight` is replaced by
`task051_qualified_non_oam_payload_is_codec_ready`. Every other GNAT, Task
049, Task 050, Rust 1.95, and real pinned-Sleet gate is kept.

## 9. Still open

* multi-namespace backend generation;
* unqualified member OMS JSON semantics (no authoritative rule yet);
* `xs:element ref` support;
* hexBinary/base64Binary provenance (Binary codec);
* Ada/C++ codecs and runtimes (the IR wire namespace is language-neutral and
  ready for them);
* a Sleet that accepts non-OAM Clark names (external; not patched here).
