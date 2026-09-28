# Task 050 — Generated Rust OMS JSON codecs

```text
Task 048:  typed operation surface (generated publish/subscribe façade)
Task 049:  Rust LA-CAL runtime and the generic global-element envelope
Task 050:  generated Rust payload-body OMS JSON codecs
```

Task 049 left a seam: `SleetRuntime<C>` accepts any `OmsJsonCodec<P>`
provider. Task 050 fills that seam with **generated** code. The primary
milestone is:

```text
real UCI 2.5 PositionReport
  -> generated Rust PositionReportMT
  -> generated OMS JSON codec            (service_codec.rs)
  -> Task 049 SleetRuntime
  -> unmodified pinned Sleet
  -> generated OMS JSON decoder
  -> typed PositionReportMT handler
```

There is no handwritten PositionReport codec anywhere. The Task 049
handwritten-codec tests are unchanged and still prove the seam accepts any
provider.

## 1. Why not Serde derives

OMS JSON is defined by XSD semantics, not by Rust representation. The
generated model deliberately uses validated scalar newtypes, remapped enum
identifiers, `BoundedVec`/`UnboundedVec`, Task 024 closed sums, flattened
inherited Records, Choice enums, and lexical `dateTime` carriers. Deriving
Serde would tie the wire format to those host-language choices and silently
produce wrong member names, enum values, Choice shapes, `$type` behavior, or
scalar spellings. **Generated model source is byte-identical** with and
without `--with-codec` (see §9). The codec is a separate generated file.

## 2. Authoritative evidence (gate before production changes)

Source: `open-arsenal/oms` @ `726272bd0390982a759c91a9cf4e13b81c2b510b`,
OMSC-SPC-013 Rev B (`docs_official/20_OMSC-SPC-013_RevB_LanguageAgnostic_CAL_Specification_DandD_v2_5.docx`,
SHA-256 `b1c3c07872570fb4f2c84fd819076b1b588148ef28225f5a8e23433efe94b1a7`;
the unofficial Markdown rendering used for reading has SHA-256
`5798da43991d5b554ed7575fa55c350aacece483d36841738e57db123f4ee98f`).

| Section | Rule the codec relies on |
| --- | --- |
| 6.1.1 Global Element Declarations | JSON text of a global element is an object with **exactly one** member. Key: `{name}` if the target namespace is `https://www.vdl.afrl.af.mil/programs/oam`, otherwise `{{namespace}}{name}`. (Runtime-owned since Task 049.) |
| 6.1.2 Particles | *sequence*: the text for each particle **in any order**; *choice*: the text for **one** of the particles. An element particle is a member keyed by `{name}` (OAM) or `{ns}name`. `maxOccurs` > 1 or unbounded → **array** of values. |
| 6.1.2 (derivation clause) | If a type is used where another type was expected, the object carries a member `"$type"` whose value is the concrete complexType name (bare for OAM, `{ns}name` otherwise). |
| 6.1.3 Complex Type Definitions | An object; members are those of the content model's particles. Examples show base-type content combined into the derived object (no nested base object), and `$type` examples for abstract bases. |
| 6.1.4 Simple Type Definitions | Mapped by primitive type only: `xs:boolean` → `true`/`false`; `xs:float`/`xs:double` → number **or** one of `"NaN"`, `"Infinity"`, `"-Infinity"`; `xs:decimal` → number; OAM `UniversallyUniqueIdentifierType` → RFC 4122 string; **otherwise → string**. |
| 6.1.5 Validation / 6.1.5.2 | A member string starting with `{` names `{namespace}local`; otherwise the namespace is OAM and the whole string is the local name. |

A member may be omitted when its element is absent (`minOccurs="0"` with zero
occurrences). Only 6.1.2's "text for each particle" produces members, so zero
occurrences produce none. OMS JSON has no `null`.

Integers are JSON **numbers** by 6.1.4 item 3. The XSD integer types
(`xs:long`, `xs:int`, `xs:short`, `xs:byte`, `xs:unsigned*`) are derived from
`xs:integer`, whose {primitive type definition} is `xs:decimal` (XML Schema
Part 2 §3.3.13). This agrees with the pinned Sleet validator
(`SimpleValueKind::Integer` requires a number) and the pinned PositionReport
vector.

### Namespace boundary (evidence for §7)

The authoritative UCI 2.5 root declares
`targetNamespace="https://www.vdl.afrl.af.mil/programs/oam"` and
`elementFormDefault="qualified"`, and contains no `form=` attribute. Every
local element in the selected closure is therefore in the OAM namespace, and
its member key is the bare `FieldDecl.name`. Schema IR keeps `FieldDecl.name`
but not the local element's QName/form (`elementFormDefault` is validated and
discarded by the frontend), so member namespaces cannot be proven for any
other namespace.

> **Superseded by [Task 051](task-051-member-qname-provenance.md).** Schema
> IR now keeps each local element's effective target namespace
> (`FieldDecl.wire_namespace_uri`). The blanket non-OAM rejection is replaced
> by member-level semantics: qualified non-OAM members use `{ns}local`, and
> unqualified members fail closed. OAM codec output is byte-identical.

### Binary boundary

OAC-SPC-001 Rev E (UCI 2.5 schema style spec, CERT SCH-000752) permits
`xs:hexBinary`. `PrimitiveKind::Binary` stores semantic octets and does not
retain whether the XSD primitive was `hexBinary` or `base64Binary`. Those have
different lexical forms, so no encoding is chosen.

> **Superseded by [Task 052](task-052-hexbinary-provenance-codec.md).** Schema
> IR now keeps Binary lexical provenance separately from the value kind
> (`TypeRef.binary_encoding`, resolved through named restrictions). A Binary
> with `xs:hexBinary` provenance is codec READY and encoded as a canonical
> uppercase hex JSON string; unknown provenance and `base64Binary` still fail
> closed. `codec-binary.xsd` is now model READY **and** codec READY.

## 3. Pinned PositionReport JSON vector

| | |
| --- | --- |
| Source repository | `open-arsenal/ams-gra-hello-world-sk-infra-sleet` (Apache-2.0) |
| Revision | `e38f61d8ce0d75c8508434a52f2ed77c69cf6a3b` (same pin as Task 049) |
| Source path | `tests/fixtures/uci/v2_5/PositionReport.json` |
| SHA-256 | `5993d4237a7ea7bc1f38ee2d1a03d9baf35f6b6cc9738ba19a25e31ab5fb148b` |
| Copy here | `tests/fixtures/oms-json/sleet-e38f61d8/PositionReport.json` (byte-identical, same SHA-256) |

The copy is not authored by this repository and has not been edited. It is
reproduced under the source's Apache-2.0 license (this repository is also
Apache-2.0) so offline tests are deterministic. To verify:
`git -C <sleet> show e38f61d8:tests/fixtures/uci/v2_5/PositionReport.json | sha256sum`.

## 4. Real PositionReport selected-closure inventory

UCI 2.5 root: `gitlab.com/open-arsenal/uci/standard` tag `v2.5` =
`093610b7753944059360d3236770ab446d039556`,
`03_OAC-STD-002_RevE_UCI_Schema_v2_5/UCI_MessageDefinitions_v2_5_0.xsd`,
SHA-256 `ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27`.
Contract: `crates/service-contract/tests/fixtures/upstream-minimal.yaml`.
World: `closed-schema`. The inventory was produced before any production
change by a throwaway program over the shared helpers
(`project_service_generation_schema`, `plan_type_emissions`,
`effective_record_fields`, `effective_choice_alternatives`,
`field_storage_semantics`, `string_profile`, `temporal_profile`,
`direct_temporal_profile`).

| Measure | Result |
| --- | --- |
| Selected messages | 1: `{oam}PositionReport -> PositionReportMT` |
| Projected declarations | 60 (60 contract-selected, 0 generated support) |
| Declaration namespaces | only `https://www.vdl.afrl.af.mil/programs/oam` |
| Planned emissions | 60 = 59 emitted declarations + 1 abstract Record kept only as inheritance metadata (`MessageType`) |
| Abstract-value wrappers / `$type` positions | 0 |
| TypeKind | Record 18 (+1 abstract), Enumeration 20, Choice 4, Primitive 17 (Float64 9, String 6, UnsignedInteger 1, DateTime 1), Alias 0, List 0 |
| Named String profiles | `UCI_SchemaVersionStringType` (UciSchemaVersion), `UniversallyUniqueIdentifierType` (UUID), `VisibleString256Type` (VisibleAscii 1..256), `NATO_SpecialWordsType`, `WhitespaceVisibleString1024Type`/`4096Type` (WhitespaceVisible collapse) |
| Named DateTime | `DateTimeType` (DateTimeZulu) |
| Direct DateTime | `SecurityInformationType.DeclassDate`, `.CUI_DecontrolDate` (0..1, `XmlSchemaDateTime`) |
| Named numeric carriers | Float64 9 (5 bounded: Altitude, AngleHalf, Angle, Distance, DoubleNonNegative; 4 unbounded); `Link16_PositionQualityType` u64 0..15 |
| Direct numeric/bool | `MissionID_Type.Version`, `VersionedID_Type.Version` (u32 domain, 0..1), `PositionReportMDT.SimulationTargetNumber` (i64, 0..1), `Joint`/`HasApproximateMarkings` (bool, 0..1) |
| Member shapes | RequiredOne 40, OptionalOne 48, Bounded{0,N} 10, Unbounded{0} 5, Unbounded{1} 1; Choice alternatives: RequiredOne 8 |
| Repeated fields | 16, all on `SecurityInformationType` (e.g. `OwnerProducer` Unbounded{1}, `SCI_Controls` 0..20, `CUI_Basic` 0..93) |
| Inherited Records | `PositionReportMT` (<- abstract `MessageType`), `SystemID_Type`, `ServiceID_Type`, `VersionedID_Type` (<- `ID_Type`), `MissionID_Type` (<- `VersionedID_Type`) |
| Choices | `OwnerProducerChoiceType`, `ReleasableToChoiceType`, `FGI_SourceOpenChoiceType`, `FGI_SourceProtectedChoiceType` (two single-occurrence alternatives each) |
| Binary / Decimal / Time / Duration | none |
| Nillable members / AbsentOnly members | 0 / 0 |

Scope gate: every construct is encodable from the current IR without a guess,
so Task 050 proceeded.

**Rust representations and accessors used** (all already public on the
model; none added): Record `pub` fields; `Option<T>`; `BoundedVec`/
`UnboundedVec` `new(Vec) -> Option` and `as_slice()`; `BoundedI64/U64`
`new -> Option` and `get()`; named integer `new -> Option` and `get()`;
named float `new -> Self` (unbounded) or `new -> Option` (bounded) and
`get()`; named Boolean `new`/`get`; String profiles and DateTime Zulu
`new(&str) -> Option` and `as_str()`; `XmlSchemaDateTime::new`/`as_str`;
enum and Choice variants; Task 024 abstract-value enum variants.

### Result

| Dimension | Value |
| --- | --- |
| Model readiness (unchanged, historical) | 60/60 selected types READY |
| Rust codec readiness (`--with-codec`) | `codec renderable emitted declarations: 59/59`, `codec status: READY` |

The codec denominator is the **emitted** declarations (59). The 60th
projected declaration is abstract `MessageType`, which has no Rust type; its
fields are flattened into `PositionReportMT`.

## 5. CLI and readiness semantics

`--with-codec` is a value-less flag on `service-check` and
`service-generate`. Repeating it is a usage error.

* **Without it**, every report line and generated file is byte-identical to
  main (§9).
* **With it**, the shared codec preflight (`analyze_service_codec` in
  codegen-core) runs on the same projected schema and `ServiceApiModel`
  that `service-generate` uses, and joins the same pre-generation verdict.
  READY means `service-generate --with-codec` emits model + service API +
  codec with no predictable failure. The report appends:

  ```text
  codec renderable emitted declarations: X/Y
  codec status: READY | NOT READY

  service codec boundary: <reason>          (only when NOT READY)
  ```

  `Y` counts every `TypeEmission` that owns a generated Rust type (including
  generated support declarations and Task 024 wrappers; excluding abstract
  Records kept only as metadata). A codec blocker is never listed as an
  unsupported UCI type, and it changes no model count.
* If the **model** is NOT READY, the codec line reads
  `codec: not measured (selected model is NOT READY)`, so one cause is never
  reported twice.
* **Zero OMS exchanges**: `codec: not required (no OMS message exchange)`.
  The service stays READY and no codec file is written; wrapper output is
  byte-identical.
* **Ada / C++** with OMS exchanges: NOT READY with
  `service codec boundary: generated OMS JSON codec is not implemented for
  Ada` (or `C++`). No placeholder file is written.
* **Failure writes nothing.** Readiness is checked before projection or any
  backend call, so no directory, model, wrapper, or codec is written. The
  combined file list is still validated by the existing single writer as
  defence in depth.

The `service-generate` summary gains `generated service codec files: N` and
`generated K file(s) including the codec` only under `--with-codec`.

**Artifact layout.** `service_codec.rs` is checked case-insensitively against
the model file (derived by `rust_model_file_name`) and `service_api.rs`. Its
module `service_codec` is checked against the wrapper's `model` and
`service_api` modules. Any clash is a `service codec boundary`.

## 6. Generated code

```text
service_api.rs        (crate root)
  #[path = "oam.rs"]            pub mod model;
  #[path = "service_codec.rs"]  pub mod service_codec;   // only with --with-codec
  pub mod service_api { ... }
```

`service_codec.rs` public surface:

```rust
#[derive(Debug, Default, Clone, Copy)]
pub struct ServiceCodec;

impl OmsJsonCodec<super::model::P> for ServiceCodec { ... }   // once per UNIQUE payload
```

Two messages that share one payload type produce one impl, so no
application registers codecs by hand:
`SleetRuntime::connect(config, service_api::service_codec::ServiceCodec)`.

**Helper naming.** Recursive helpers are private `encode_tNNN` /
`decode_tNNN` functions. `NNN` is the stable emission index, and a comment
names the source declaration. They never derive from schema names, so they
add no public API, cannot hit keywords or normalization collisions, and need
no name preflight. Every Rust spelling that refers to the model comes from
the backend's own `upper_camel`/`snake_case` and
`generated_enum_variant_name(BackendLanguage::Rust, ..)`, the same functions
that render the model. There is no second casing implementation.

> **Task 054:** Record field and Choice variant host names now come from the
> shared `generated_record_field_name` / `generated_choice_alternative_name`
> (a reserved member is escaped, e.g. `field_type`, `AlternativeSelf`). Wire
> keys are unchanged: they still come from `FieldDecl::wire_name()`, so
> `field_type` encodes as `"Type"`, and an escaped host name is never
> accepted as a wire alias. See
> [Task 054](task-054-member-identifier-remapping.md).

**Inputs.** `generate_service_codec(model: &ServiceApiModel, schema: &SchemaIr,
world: GenerationWorld)` consumes the lowered model, the projected schema,
and the world only. It reuses `plan_type_emissions`,
`effective_record_fields`, `effective_choice_alternatives`,
`field_storage_semantics`, the Task 024 projection carried by
`TypeEmission::AbstractValue`, and `generated_enum_variant_name`. It re-runs
the codec preflight, so a direct caller that skipped readiness still fails
closed.

## 7. Wire mapping implemented

| Construct | Encode | Decode (strict) |
| --- | --- | --- |
| Boolean | JSON `true`/`false` | JSON boolean only |
| Signed/unsigned integer (named or direct, checked) | JSON number from `get()` | integer number in i64/u64, then the generated `new`; `None` -> `CodecError` |
| Float32/Float64 | finite -> number (f32 via its shortest spelling); NaN -> `"NaN"`, +inf -> `"Infinity"`, -inf -> `"-Infinity"` | number (finite in range) or exactly those three strings; `"inf"`, `"+Infinity"`, `"nan"`, ... rejected; a bounded wrapper's `new` decides if a special value is legal |
| String | JSON string | JSON string |
| String profiles (UUID, schema version, visible ASCII, NATO, whitespace-visible) | stored `as_str()` | generated `new(&str)` |
| Named DateTime Zulu / direct `XmlSchemaDateTime` | stored lexical `as_str()` (never normalized by the codec) | generated `new(&str)` |
| Enumeration | exact `EnumVariant.wire_value` | exact wire string -> variant named by `generated_enum_variant_name` |
| Concrete Record | object keyed by the XSD `FieldDecl.name` over `effective_record_fields` (inherited fields flattened, no nested base, no `$type`) | object; unknown member, missing required member, and `null` all rejected; any member order |
| 0..1 | `Some` -> member; `None` -> omitted | absent -> `None` |
| Repeated (`maxOccurs` > 1) | array from `as_slice()`; zero occurrences -> omitted | array (absent = empty), each element decoded, then `BoundedVec::new`/`UnboundedVec::new` decides cardinality |
| Task 026 AbsentOnly | nothing | the member is unknown -> rejected |
| Choice | object with exactly one member: the XSD alternative name (array if the alternative repeats) | exactly one recognized member; zero, two, or unknown rejected |
| Closed abstract value (Task 024) | the concrete object plus `"$type": "<concrete local name>"` | requires `$type`; exact known concrete name -> that descendant's decoder -> the matching enum variant; missing/unknown/non-string `$type` rejected; fields are never inspected to pick a descendant |

A concrete Record decoder accepts a `$type` member only when it names the
Record itself, which is harmless under 6.1.3. Any other value is rejected,
because decoding a derived type as its base would drop content.

No bound, facet, or cardinality appears in `service_codec.rs`. The model's
constructors are the only validation authority. (The test
`codec_references_the_model_renderer_spellings` checks, for example, that
`Level`'s `-10` bound is not in the codec.)

**Diagnostics.** Every decode failure is a `CodecError` whose message starts
with the semantic path, e.g. `CodecPayload.Identity.UUID: value rejected by
generated UniversallyUniqueIdentifierType::new` or
`CodecPayload.Source.Levels[0]: ...`. `CodecError`'s public type is unchanged.

**Duplicate members.** `decode_payload` receives a parsed `serde_json::Value`,
and the parser has already collapsed duplicate object keys. Task 050 does
**not** detect duplicate member names in raw JSON text. In live operation,
Sleet validates OMS JSON against the XSD before routing. Moving raw-text
validation earlier is possible later hardening work.

## 8. Explicit codec boundaries (fail closed, `service codec boundary:`)

| Boundary | Why |
| --- | --- |
| Any codec-emitted declaration, referenced type, or `$type` descendant outside the OAM namespace | Schema IR has no local element QName/form, so `{ns}local` member keys and `$type` values cannot be proven. A future task can strengthen IR field/member QName semantics. This is a **codec** limit; the model and Task 049's non-OAM handwritten-codec tests are unaffected (generation is opt-in). **Superseded by [Task 051](task-051-member-qname-provenance.md):** only an *unqualified* stored member now fails closed. |
| `PrimitiveKind::Binary` | hexBinary vs base64Binary lexical provenance is not retained. Control: `codec-binary.xsd` is model READY, codec NOT READY, and nothing is written. **Superseded by [Task 052](task-052-hexbinary-provenance-codec.md):** HexBinary-provenanced Binary is READY; only unknown provenance or base64Binary fails closed. |
| `Decimal`, `Time`, `Duration` | Not evidenced in scope. (The model backend fails first for most of them; the codec reports them separately if the model can render them.) |
| Nillable members, optional Choice alternatives, Alias/List declarations | Not represented by the codec (none occur in the PositionReport closure). |
| Ada, C++ | No generated codec exists. |

The model backend was not expanded. A primitive the model cannot render stays
a model blocker first.

## 9. Default-output compatibility gate

The release binaries of `main` (`629266ba`) and this branch were compared
without `--with-codec`, using the Task 049 comparison script over every
`tests/fixtures/service-generate/*.yaml` pairing, the `position-report.xsd`
and `service-status.xsd` upstream pairings, and the real UCI 2.5 PositionReport,
for rust/cpp/ada:

```text
service-generate: runs=108 identical_files=124 wrapper_diffs=0 model_diffs=0
service-check stdout+exit (service-generate and service-check fixtures): 105 runs, 0 diffs
```

Model files, Rust/Ada/C++ wrappers, zero-OMS output, and default reports are
all byte-identical. The Task 049 facade-tests fixtures are still generated
without the flag.

## 10. Tests

| Test | What it proves |
| --- | --- |
| `runtime-rust-facade-tests/tests/generated_codec.rs` (7) | Synthetic OAM `codec-oam` fixture generated by the real `service-generate --with-codec`: full round trip to the exact expected JSON and back; enum wire values incl. Task 044 remaps (`5G`->`Value5G`, `SOME_VALUE`->`SOMEVALUE`, `Self`->`ValueSelf`); Choice; closed abstract `$type`; float edge corpus (±0.0, finite, f64 MIN/MAX, bounded -180/180, NaN, ±Infinity; rejects `inf`, `+Infinity`, `nan`, ...); optional/empty/max/unbounded repetition; any member order; a 36-case strict negative corpus with path-prefixed `CodecError`s |
| `generated_codec_mock_owp.rs` | generated facade + generated codec -> mock OWP: exact PUB document, typed MSG, invalid MSG -> decode event, never a handler call |
| `generated_codec_sleet.rs` | generated typed value -> generated codec -> **unmodified pinned Sleet** (which validates the document against `codec-oam.xsd`) -> generated codec -> typed handler; same-payload `CodecEcho` is not routed to the `CodecNotice` subscription |
| `real_uci_position_report.rs` (2) | **real UCI 2.5 PositionReport**: pinned vector -> generated `PositionReportMT` (13 representative values asserted) -> re-encode -> parsed-JSON equality with the vector; and the typed round trip through unmodified Sleet loaded with the same pinned UCI root |
| `cli/tests/service_codec.rs` (6) | controls A-D (OAM ready + generated, Binary, non-OAM, zero-OMS), Ada/C++ boundary, flag parsing/help |
| `backend-rust/tests/service_codec.rs` (3) | direct renderer fails closed (Binary, non-OAM); codec spellings are exactly the model renderer's; no bound duplicated; deterministic |
| `codegen-core` `service_codec::tests` (2) | every `PrimitiveKind` has an explicit codec decision; non-OAM and nillable fail closed |

Task 049's handwritten-codec tests (`mock_owp`, fairness, lifecycle, errors,
dispatch, `real_sleet`) are unchanged and do not use the generated codec.

## 11. Real UCI in CI

The pinned public UCI source is fetched reliably: the repository already
fetches the same tag for Pages. `scripts/fetch-pinned-uci-2.5.sh` fetches
tag `v2.5` from `gitlab.com/open-arsenal/uci/standard` **outside the
repository**, requires commit `093610b7...`, requires the root SHA-256
`ac943049...`, and prints the root path. `runtime-rust-facade-tests/build.rs`
verifies the SHA-256 **again** before using `AMS_GRA_UCI_2_5_ROOT`. A
mismatch is a hard build failure, not a skip. With the root set, the build
script generates the real PositionReport model + API + codec with
`--with-codec` (using the test-only `position-report-loop.yaml`: the real
global `PositionReport` as one input and one output exchange on
`mission.position-report`) and enables `cfg(ams_gra_real_uci)`. Without it,
the two real-UCI tests compile as explicit `SKIPPED` stubs.

CI runs the real PositionReport decode/re-encode and the real Sleet round
trip as named steps. `scripts/run-real-sleet-test.sh` requires each test to
print its `PASSED` line. No local UCI tree or generated output is committed.

## 12. Architecture after Task 050

```text
Service Contract
      |
      v
generated service_api.rs
      +---- generated model
      +---- generated service_codec.rs
                   | OmsJsonCodec<P>
                   v
           SleetRuntime<ServiceCodec>
                   | generic global envelope
              sleet-client
                   | OWP/WebSocket
                 Sleet  (unchanged)
```

The generated codec owns payload fields, enum wire values, sequence/choice
representation, inheritance, and checked model construction. The runtime owns
the global message QName envelope, PUB/SUB/UNSUB/MSG, SIDs, dispatch, and
the connection.

## 13. Still open

Ada runtime; C++ runtime; Ada/C++ generated codecs; generic
extension-namespace member QName support; binary lexical provenance
(hexBinary done in Task 052);
reconnect; Phase 4 SPARK work. Only the **Rust** codec slice is complete.
