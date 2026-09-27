# Task 052 — Preserve hexBinary provenance and generate Rust Binary codecs

```text
Task 025:  Binary lowered as SEMANTIC OCTETS (Vec<u8> / vector<uint8_t> /
           Unsigned_8 vector); the XML lexical family was discarded.
Task 050:  the Rust OMS JSON codec refused every Binary value, because
           PrimitiveKind::Binary could not say hexBinary vs base64Binary.
Task 051:  local element QName provenance restored at the IR boundary.
Task 052:  Binary LEXICAL provenance restored at the IR boundary; Rust
           generated codecs encode/decode xs:hexBinary.
```

The generated MODEL is unchanged: it still owns bytes. Only the opt-in
`--with-codec` output changes, and only where a codec surface contains a
Binary value with explicit `xs:hexBinary` provenance.

## 1. Evidence gate (recorded before production changes)

### 1.1 XML Schema Part 2: Datatypes Second Edition

Source: W3C Recommendation 28 October 2004,
<http://www.w3.org/TR/2004/REC-xmlschema-2-20041028/> (fetched 2026-09-27;
SHA-256 of the fetched HTML
`980de872aa2c50013d5202176eb85eef3f183a7df635fdcb6b4aa34c1c12eb72`).

| Rule | Section | Quoted / established |
| --- | --- | --- |
| Value space | 3.2.15 | "The ·value space· of **hexBinary** is the set of finite-length sequences of binary octets." |
| Lexical representation | 3.2.15.1 | "each binary octet is encoded as a character tuple, consisting of two hexadecimal digits (`[0-9a-fA-F]`) representing the octet code." |
| Case-insensitive digits | 3.2.15.1 | The digit class is `[0-9a-fA-F]`: both cases are lexical. |
| Canonical representation | 3.2.15.2 | "Specifically, the lower case hexadecimal digits (`[a-f]`) are not allowed." Canonical is **uppercase**. |
| whiteSpace fixed collapse | 4.3.6 | "For all ·atomic· datatypes other than string (and types ·derived· by ·restriction· from it) the value of whiteSpace is **collapse** and cannot be changed by a schema author." |
| collapse | 4.3.6 | replace: "#x9 (tab), #xA (line feed) and #xD (carriage return) are replaced with #x20 (space)"; collapse: "contiguous sequences of #x20's are collapsed to a single #x20, and leading and trailing #x20's are removed." |
| Length units | 4.3.1 | "For **hexBinary** and **base64Binary** and datatypes ·derived· from them, length is measured in **octets** (8 bits) of binary data." (also minLength/maxLength, 4.3.2/4.3.3) |
| base64Binary | 3.2.16 | Same value space ("finite-length sequences of binary octets"), different lexical family (RFC 2045 Base64 alphabet). |

Consequences:

* `hexBinary` and `base64Binary` share one VALUE space, so
  `PrimitiveKind::Binary` stays one semantic kind; the difference is purely
  lexical and is kept as separate provenance.
* The decoder applies collapse first. An internal space survives collapse
  (as one `#x20`) and is not a hexadecimal digit, so `"AA BB"` and
  `"AA\tBB"` are invalid while `"  0A  "` and `"\t0A\r\n"` are valid.
* Length facets count octets (already true since Task 016), never lexical
  characters.

### 1.2 OMSC-SPC-013 Rev B (OMS JSON)

Source: `open-arsenal/oms` @ `726272bd0390982a759c91a9cf4e13b81c2b510b`,
`docs_official/20_OMSC-SPC-013_RevB_LanguageAgnostic_CAL_Specification_DandD_v2_5.docx`
(SHA-256 `b1c3c07872570fb4f2c84fd819076b1b588148ef28225f5a8e23433efe94b1a7`),
read through the unofficial Markdown rendering (SHA-256
`5798da43991d5b554ed7575fa55c350aacece483d36841738e57db123f4ee98f`): the same
pins as Tasks 050/051.

* **§6.1.3 Complex Type Definitions, example.** `MyDerivedType` extends
  `MyBaseType` with `<xs:element name="C" type="xs:hexBinary"/>` and "may
  result in the following JSON text": `{ "A": 3.14, "B": "PI", "C": "ee" }`.
  A hexBinary value is a JSON **string**, and lowercase `ee` is a valid
  spelling of it.
* **§6.1.4 Simple Type Definitions.** "mapped based only on its {primitive
  type definition}": `xs:boolean` → true/false; `xs:float`/`xs:double` →
  number or `"NaN"`/`"Infinity"`/`"-Infinity"`; `xs:decimal` → number; OAM
  `UniversallyUniqueIdentifierType` → RFC 4122 string; **"Otherwise
  string."** `xs:hexBinary` is none of the special cases, so it is the
  *otherwise → string* case.

### 1.3 Canonical encoder vs. the `"ee"` example

`"ee"` shows lowercase is an accepted lexical form. OMSC-SPC-013 does not
mandate one spelling among the lexical forms, so the generator
deterministically emits the **XML Schema canonical** form (3.2.15.2):
uppercase, exactly two digits per octet, no whitespace, separator, or prefix.
The decoder accepts every lexical form (either case, collapsible surrounding
whitespace). Encoder determinism and decoder generality are deliberately
different.

## 2. UCI Binary inventory (pinned releases)

Raw XSD scan (`lxml`, evidence only, not committed) of UCI 2.5
`093610b7753944059360d3236770ab446d039556` (root SHA-256 `ac943049...bf27`)
and UCI 2.6 `78eb61b6112c8bffa40820c33124b57787fc5bd9` (root SHA-256
`af54ce72...955b`). Neither release references `xs:base64Binary` in any of
its three documents.

### UCI 2.5

| Occurrence | Kind | Line | Base chain | Constraints | Cardinality / use |
| --- | --- | --- | --- | --- | --- |
| `AtomicValueType.HexBinaryValue` | direct, **Choice** alternative | 12893 | `xs:hexBinary` | none | 1..1 |
| `IFF_ModeSelectionType.EHS_BDS_Registers` | direct | 47074 | `xs:hexBinary` | none | 0..unbounded |
| `LogMDT.Data` | direct | 51053 | `xs:hexBinary` | none | 0..1 |
| `SpectralBandType.SpectralImage` | direct | 96540 | `xs:hexBinary` | none | 0..1 |
| `SubsystemStreamMDT.SubsystemStreamBinary` | direct | 100475 | `xs:hexBinary` | none | 0..1 |
| `AA_CodeType` | named | 109762 | `-> xs:hexBinary` | `length=6` | `SpecificBDS_RegistersType.AA_Code` 0..unbounded |
| `BDS_AddressType` | named | 112166 | `-> xs:hexBinary` | `length=2` | `SpecificBDS_RegistersType.BDS_Address` 1..6 |
| `SHA_2_256_HashType` | named | 141248 | `-> xs:hexBinary` | `length=32` | `FileMetadataMDT.SHA_2_Hash`, `ProductMetadataMDT.SHA_2_Hash` 0..1 |

Confirmed: 5 direct local-field `xs:hexBinary` references, 3 named Binary
declarations (all constrained), first direct Binary boundary
`AtomicValueType.HexBinaryValue`, 0 `xs:base64Binary`.

### UCI 2.6

| Declaration | Line | Base chain | Constraints | Referenced by |
| --- | --- | --- | --- | --- |
| `AA_CodeType` | 110190 | `-> xs:hexBinary` | `length=6` | `SpecificBDS_RegistersType.AA_Code` 0..unbounded |
| `BDS_AddressType` | 112511 | `-> xs:hexBinary` | `length=2` | `SpecificBDS_RegistersType.BDS_Address` 1..6 |
| `HexBinaryType` | 123232 | `-> xs:hexBinary` | none | `AtomicValueType.HexBinaryValue` 1..1, `LogMDT.Data` 0..1, `SpectralBandType.SpectralImage` 0..1, `SubsystemStreamMDT.SubsystemStreamBinary` 0..1 |
| `IFF_RegisterType` | 124839 | `-> HexBinaryType -> xs:hexBinary` | `length=7` | `IFF_ModeSelectionType.EHS_BDS_Registers` 0..unbounded |
| `SHA_2_256_HashType` | 141524 | `-> xs:hexBinary` | `length=32` | `FileMetadataMDT.SHA_2_Hash`, `ProductMetadataMDT.SHA_2_Hash` 0..1 |

Confirmed: 0 direct local-field `xs:hexBinary`, 5 named Binary declarations
including `HexBinaryType` (the only unconstrained one) and `IFF_RegisterType`
(a named restriction of a named restriction), 0 `xs:base64Binary`. UCI 2.6
moved every former direct field onto `HexBinaryType`.


### Normalized IR provenance inventory (branch; asserted in CI)

`crates/xsd-frontend/tests/uci_binary_provenance.rs` loads each pinned root
(SHA-256 verified first) and asserts, through the shared IR query only:

| Release | Direct `Primitive(Binary)` members | Named Binary declarations | Members referencing a named Binary | Constrained named | Unknown | Base64 |
| --- | --- | --- | --- | --- | --- | --- |
| UCI 2.5 | 5, all `HexBinary` | 3, all `HexBinary` | 4 | 3 (`AA_CodeType`, `BDS_AddressType`, `SHA_2_256_HashType`) | 0 | 0 |
| UCI 2.6 | 0 | 5, all `HexBinary` | 9 | 4 (`AA_CodeType`, `BDS_AddressType`, `IFF_RegisterType`, `SHA_2_256_HashType`) | 0 | 0 |

UCI 2.6 `IFF_RegisterType` resolves `IFF_RegisterType -> HexBinaryType ->
xs:hexBinary` through the named-on-named ancestry. The normalized counts
equal the raw XSD counts, so no normalization step drops or invents Binary.

## 3. IR representation

```rust
pub struct TypeRef {
    pub target: TypeRefTarget,
    pub binary_encoding: Option<BinaryLexicalEncoding>,
}

pub enum BinaryLexicalEncoding { HexBinary, Base64Binary }

pub fn resolve_binary_encoding(schema, type_ref) -> Option<BinaryLexicalEncoding>;
pub fn declaration_binary_encoding(schema, declaration) -> Option<BinaryLexicalEncoding>;
```

* `PrimitiveKind::Binary` is unchanged and remains the only Binary VALUE
  kind. No second Binary kind exists; provenance is a separate field.
* Provenance lives on the **primitive reference** (`TypeRef`), i.e. on the
  primitive ancestry, not on the value model:
  * a direct `field F type="xs:hexBinary"` has
    `TypeRef::binary(HexBinary)` as its `type_ref`;
  * `A restricts xs:hexBinary` has `TypeRef::binary(HexBinary)` as its
    `base_type`;
  * `B restricts A` has `TypeRef::named(A)` as its `base_type`, and a field of
    type `B` has `TypeRef::named(B)`: **nothing is copied** onto either. The
    shared query answers `B -> A -> xs:hexBinary -> HexBinary`.
* `TypeRef::primitive(kind)` and `TypeRef::named(name)` never fabricate
  provenance. Hand-built `TypeRef::primitive(Binary)` is legal IR with
  UNKNOWN provenance.
* Restriction normalization intersects constraints but copies `base_type`
  verbatim, so constraint intersection cannot erase provenance
  (`task052_constraint_intersection_keeps_provenance`, and the frontend
  `Hex0 -> Hex1(maxLength) -> Hex2(length)` fixture).
* No raw string is semantic state; `BinaryLexicalEncoding::xsd_name` exists
  only for diagnostics.

### IR validation

`SchemaIr::validate` rejects `InvalidBinaryEncodingProvenance` when a
`TypeRef` carries provenance but is not `Primitive(Binary)`: e.g.
`Primitive(SignedInteger) + HexBinary`, or a named field/base/alias/list/
payload reference with provenance. Unknown provenance on Binary is legal:
synthetic IR can model octets independently of XSD, and model generation does
not need the lexical family. Only codec readiness rejects it.

### ServicePlan binding

`TypeSemantics.base_type` and `MemberSemantics.type_ref` are `TypeRef`s, so
provenance participates in the existing semantic snapshot with no new
mechanism. A plan resolved against a hexBinary schema does not bind to an
otherwise identical schema whose Binary is Base64 or unknown, for a direct
field or a named restriction base
(`service_plan_binding::task052_changed_binary_lexical_provenance_is_rejected`).

## 4. Frontend

`builtin_primitive_semantics` maps `xs:hexBinary -> (Binary, HexBinary)`.
The single `resolve_type_semantics` site serves both a local element `@type`
and a restriction `@base`, so direct fields and named restrictions receive
provenance from one place; named-on-named restrictions get it through their
named `base_type`. There is no field-name or type-name heuristic.
`xs:base64Binary` remains an explicit `UnsupportedConstruct` for both a field
and a restriction base (`binary_provenance::task052_base64_binary_remains_an_explicit_unsupported_construct`).
The production frontend never emits `Base64Binary`.

## 5. Codec readiness (`codegen-core::service_codec`)

The Task 050 `BinaryProvenance` blocker is replaced. A Binary value's
decision comes ONLY from resolved provenance (`binary_support`):

| Resolved provenance | Model representation | Codec |
| --- | --- | --- |
| `HexBinary` | supported (unconstrained) | **READY** |
| `HexBinary` | constrained | model NOT READY first; codec not measured |
| unknown | any | NOT READY: `<Owner.Member> is Binary but its XSD lexical encoding provenance is unknown` |
| `Base64Binary` (manual IR only) | any | NOT READY: `<loc> is Binary with XSD lexical encoding base64Binary; the base64Binary codec mapping is not implemented` |

Direct members use `resolve_binary_encoding(field.type_ref)`; named Binary
declarations use `declaration_binary_encoding(declaration)`. There is no
`PrimitiveKind::Binary => hex` path anywhere; the direct renderer re-checks
provenance too and refuses anything but resolved `HexBinary`.


## 6. Generated Rust codec

### One shared private helper pair

`service_codec.rs` gains exactly one private
`encode_hex_binary(&[u8]) -> Value` and one private
`decode_hex_binary(&Value, &str) -> Result<Vec<u8>, CodecError>`, appended
**only** when the rendered codec actually encodes a HexBinary value. Every
Binary-free codec is byte-identical to Task 051 output. No per-type parser, no
public helper, no runtime-rust API change, and **no new dependency** (no
`hex`, `data-encoding`, or base64 crate; `Cargo.lock` is unchanged). The
mapping is payload lexical mapping and belongs in generated source.

* **Encoder:** a 16-entry uppercase digit table, two characters per octet.
* **Decoder:** (1) require a JSON string; (2) whiteSpace=collapse: split on
  SPACE/TAB/LF/CR, drop empty pieces, rejoin with one SPACE (equivalent to
  replace + collapse + trim); (3) every remaining byte must be an ASCII
  `[0-9A-Fa-f]` digit, else `only hexadecimal digits [0-9A-Fa-f] are
  allowed`; (4) the digit count must be even, else `odd number of
  hexadecimal digits`; (5) pair nibbles into octets. The alphabet is checked
  before parity, so `"AA BB"` reports the space rather than a misleading
  parity error.

### Lowering

| Shape | Encode | Decode |
| --- | --- | --- |
| direct `Vec<u8>` (required) | `encode_hex_binary(x)` | `decode_hex_binary(v, path)?` |
| optional `Option<Vec<u8>>` | member only when `Some` | absent → `None` |
| bounded/unbounded repeated | array of hex strings (one per Binary VALUE) | array → `BoundedVec::new` / `UnboundedVec::new` |
| named unconstrained wrapper | `encode_hex_binary(value.as_slice())` | `decode_hex_binary(value, path).map(Type::new)` |
| Choice alternative | `{"HexBinaryValue": "..."}` | selects that variant |

Outer cardinality counts Binary values; the inner string length is 2 ×
the octets of one value. The two are independent (a 4-item `Chunks` array
exceeds `0..3` whatever each item's length). A single Binary value is
never a JSON array of numbers, and an array is rejected on decode
(`expected hexBinary string, found array`).

The named wrapper is reached only through its public `new`/`as_slice`; no
private representation is touched. Constrained named Binary never reaches
the codec: the model backends still reject it first.

### Corpora (`runtime-rust-facade-tests/tests/generated_codec_hexbinary.rs`)

Canonical encoder: `[] -> ""`, `[00] -> "00"`, `[0a] -> "0A"`,
`[ee] -> "EE"`, `[00,0f,ab] -> "000FAB"`, `[ff] -> "FF"`,
`[00,0a,ee,ff] -> "000AEEFF"`, and all 256 octet values (512 uppercase
characters) with exact round trip.

Valid decode: `""`, `"00"`, `"ee"`, `"EE"`, `"0aFf"`, `"000aeeff"`,
`"  0A  "`, `"\t0A\r\n"`, and `" \t\n\r "` (collapses to empty). Each
re-encodes canonically.

Invalid decode (exact path-prefixed messages): odd `"0"`, `"ABC"`,
`"  ABC "`; alphabet `"GG"`, `"0xAA"`, `"AA-BB"`, `"AA BB"`, `"AA\tBB"`,
`"AA\u{a0}"` (NBSP is not XML whitespace), fullwidth `"\u{ff10}\u{ff10}"`,
Arabic-Indic `"\u{0660}\u{0661}"`, `"+0A"`. JSON type: `null`, boolean,
number, array, object are each rejected.


## 7. Synthetic codec services

* `codec-binary.xsd` / `.yaml` (Task 050 control, **flipped**): one direct
  `xs:hexBinary` `Data`. Before: model READY, codec NOT READY `0/1` with the
  "cannot distinguish" message, nothing written. After: model READY, codec
  READY `1/1`, three files, `Data = [00,0A,EE,FF]` → `"Data": "000AEEFF"`.
  The historical explanation stays in the test doc comment and in Task 050's
  doc; the negative control is now manual-IR unknown provenance.
* `codec-hexbinary.xsd` / `.yaml` (new, OAM): required, optional, bounded
  (0..3) and unbounded direct Binary; `BlobBytes -> xs:hexBinary` and
  `BlobAlias -> BlobBytes -> xs:hexBinary` named wrappers; and the
  `AtomicLike` Choice (`IntValue | HexBinaryValue`) shaped like UCI 2.5
  `AtomicValueType`. Model READY, codec READY `4/4`. Generated model:
  `data: Vec<u8>`, `maybedata: Option<Vec<u8>>`,
  `chunks: BoundedVec<Vec<u8>, 0, 3>`, `stream: UnboundedVec<Vec<u8>, 0>`,
  `blob: BlobBytes`, `alias: Option<BlobAlias>`,
  `AtomicLike::HexBinaryValue(Vec<u8>)`: the Task 025 API, unchanged.

## 8. Real UCI 2.5 selected-message evidence

Every UCI 2.5 global message whose raw closure reaches a Binary (27
messages) was checked with `service-check --language rust --world
closed-schema --with-codec` on a throwaway loop contract, before (main
`ef1e767`) and after (this branch):

| Category | Messages | Before → after |
| --- | --- | --- |
| **A**: model READY, Binary the only codec blocker | `SubsystemStream` (40 selected types; codec 38/39 → **39/39**), `SpectralDensityReport` (45; 43/44 → **44/44**) | codec NOT READY → **READY** |
| B: model READY, other codec blockers | none | — |
| C: model NOT READY for unrelated constructs | `Log` (`LogMDT.ServiceUpTime` direct `xs:duration`), `FileMetadata` (`FileNameType`, constrained `SHA_2_256_HashType`), `ProductMetadata`, `IFF_Activity`, `IFF_Command` (constrained `AA_CodeType`/`BDS_AddressType`, `DurationType`, `EmptyType`, ...) | unchanged |
| C: projection fails before readiness | 20 `Product*`/`ProductProcessingFunction`/`Response` messages (16 "cyclic generated value dependencies", 4 "unsupported abstract structural value"); these are the messages that reach `AtomicValueType.HexBinaryValue` | unchanged |

Only the two category-A reports changed (25 of 27 reports are byte-identical).
`SubsystemStream` is the smaller of the two, so it gets the test-only
contract `tests/fixtures/service-generate/subsystem-stream-loop.yaml` (one
input and one output exchange on `subsystem.stream`), generated by
`runtime-rust-facade-tests/build.rs` with `--with-codec` from the
SHA-256-verified root:

* `real_uci_subsystem_stream::real::task052_real_subsystem_stream_hex_binary_round_trips`:
  the real generated `SubsystemStreamMT` decodes a document whose
  `MessageData.SubsystemStreamBinary` is `"000aeeff"` to
  `Some([00,0A,EE,FF])` and re-encodes it as `"000AEEFF"`; absent → `None`
  → no member; `"ABC"` → `SubsystemStreamMT.MessageData.SubsystemStreamBinary:
  "ABC" is not hexBinary: odd number of hexadecimal digits`.
* `...::task052_real_subsystem_stream_round_trips_through_real_sleet`: the
  same typed value through the generated façade, `SleetRuntime`, and
  unmodified Sleet loaded with the same pinned UCI 2.5 root, back to the typed
  handler with identical octets.

No UCI vector exists for this message in pinned Sleet, so the document is
test-authored: header/security members are the pinned PositionReport
vector's, plus the minimal required `SubsystemStreamMDT` members.


### AtomicValueType

`AtomicValueType.HexBinaryValue` (UCI 2.5 line 12893) is asserted on the real
IR (`uci_binary_provenance`): a **required** (1..1) Choice alternative, a
direct `TypeRef::binary(HexBinary)`, wire QName `HexBinaryValue` in the OAM
namespace. Every real message that reaches it fails projection for unrelated
reasons (cycles / open abstract values), so its generated representation is
proven with the synthetic `AtomicLike` Choice instead
(`AtomicLike::HexBinaryValue(Vec<u8>)`, JSON `{"HexBinaryValue": "00A1FF"}`;
decoding `"00a1ff"` selects that variant). The UCI declaration is not
rewritten or forked.

### UCI 2.6

All 5 named Binary declarations resolve to HexBinary; there are 0 direct
Binary members and 0 Base64. The four constrained declarations
(`AA_CodeType` length 6, `BDS_AddressType` 2, `IFF_RegisterType` 7,
`SHA_2_256_HashType` 32) remain MODEL unsupported exactly as before; Task 052
adds no constrained Binary carriers. `task052_constrained_hex_binary_remains_a_model_blocker`
proves the same boundary on a synthetic `length=6` hexBinary: model NOT
READY, no `service codec boundary` line.

## 9. Runtime evidence

### Mock OWP

`generated_codec_mock_owp::task052_hex_binary_generated_codec_round_trips_through_mock_owp`:
typed `codec-hexbinary` value → generated codec → generated publish façade →
`SleetRuntime<ServiceCodec>` → pinned sleet-client → mock peer. The PUB body
is exactly `{"BlobNotice":{"Id":9,"Data":"000AEEFF","Chunks":["AB"],"Blob":"0F","Atomic":{"HexBinaryValue":"00A1FF"}}}`
(canonical uppercase). A MSG with lowercase `"000aeeff"`/`"0f"`/`"00a1ff"`
reaches the typed handler as `[00,0A,EE,FF]`, `[0F]`, and
`HexBinaryValue([00,A1,FF])`. A MSG with `"000"` is a
`SubscriptionDecodeError` (`BlobPayload.Data: "000" is not hexBinary: odd
number of hexadecimal digits`), never a handler call.

### Real pinned Sleet

Unmodified `open-arsenal/ams-gra-hello-world-sk-infra-sleet` @
`e38f61d8ce0d75c8508434a52f2ed77c69cf6a3b`, through
`scripts/run-real-sleet-test.sh` (new `run_one` lines; the Task 049/050/051
lines are unchanged):

* `generated_codec_sleet_hexbinary`: generated typed bytes → generated hex
  codec → `SleetRuntime` → sleet-client → Sleet (schema `codec-hexbinary.xsd`)
  → generated decoder → identical typed value (`received == value`).
* `real_uci_subsystem_stream` (with the pinned UCI 2.5 root): see §8.

**Sleet lexical validation, observed and recorded.** Pinned Sleet's
`validator.rs` maps every XSD primitive other than boolean, decimal, the
integer family, float, and double to `SimpleValueKind::String`, which only
checks that the JSON value is a string. The same test publishes, through a
test-only probe codec that delegates to the generated codec, a document
whose `Data` is `"0G"`. Observed: Sleet **accepts and routes** it; the
GENERATED decoder rejects it (`BlobPayload.Data: "0G" is not hexBinary: only
hexadecimal digits [0-9A-Fa-f] are allowed`) and no handler runs. This is an
external validator limitation. The generated decoder is the authority for
even length, alphabet, whitespace collapse, and byte reconstruction, and it
was not loosened. Sleet rejected no spec-valid hexBinary string in any test.

## 10. Compatibility and byte identity

Method (same discipline as Tasks 050/051): a release build of main
`ef1e767` and a release build of this branch, run over every
`tests/fixtures/service-generate/*.yaml` contract with its schema, the two
upstream contracts with their fixtures, and the pinned real UCI 2.5 root
with `upstream-minimal.yaml`, `position-report-loop.yaml`, and
`subsystem-stream-loop.yaml`; for `rust`, `cpp`, and `ada` without the flag,
and for `rust` also with `--with-codec`. Each pair compares
`service-check` stdout and exit code, `service-generate` exit code and
stderr, the generated file set, and every generated file byte for byte.

Result: **188 runs; 385 generated model/wrapper files byte-identical, 0
different; 37 `service_codec.rs` files that main could already generate
byte-identical, 0 different.** The only differences are the four expected
Rust `--with-codec` exit-code flips below (1 → 0, reports now end in
`codec status: READY`).

* **Default output:** every `service-check` report and exit code, every
  Rust/C++/Ada model file, and every `service_api.{rs,hpp,ads}` wrapper is
  byte-identical. Adding provenance changed no model and no default API.
* **Existing codecs without Binary:** every `service_codec.rs` main could
  already generate (codec-oam, runtime-test, codec-choice, codec-inherit,
  codec-shape, codec-form-*, the real PositionReport, ...) is byte-identical:
  the hex helpers are emitted only for HexBinary surfaces.
* **Expected differences, and only these:** Rust `--with-codec` for the
  Binary-containing services (`codec-binary`, `codec-hexbinary`,
  `optional-primitive` with its `Maybe_Binary`, and the real UCI
  `subsystem-stream-loop`) flip from exit 1 (codec NOT READY, nothing
  written) to exit 0 (codec READY, codec file written).

### PositionReport regression

Real UCI 2.5 PositionReport has no Binary in its closure. Unchanged: model
60/60 READY, codec 59/59 READY, model files, `service_api.rs`, and
`service_codec.rs` byte-identical to main;
`real::task050_real_position_report_decodes_and_re_encodes` and
`real::task050_real_position_report_round_trips_through_real_sleet` pass
unchanged.

### Backend model coverage

The whole-schema coverage matrix was not rerun: Task 052 changes no model
renderability, and the byte comparison above shows every generated model
file unchanged. `cargo test --workspace` (which includes the coverage and
backend suites) passes unchanged.

### Rust 1.95

`cargo +1.95.0 check --locked` of `ams-gra-oms-runtime-api`,
`ams-gra-oms-runtime-rust --all-targets`, and
`ams-gra-oms-runtime-rust-facade-tests --all-targets` (which compiles the
generated `codec_binary` and `codec_hexbinary` codecs) passes, and again
with `AMS_GRA_UCI_2_5_ROOT` (generated real PositionReport and real
SubsystemStream codecs). No MSRV change.

## 11. Tests and CI gates

| Test | Proves |
| --- | --- |
| `ir` `tests::task052_*` (4) | named-chain resolution `Hex2 -> Hex1 -> Hex0 -> xs:hexBinary`; no copy onto fields; constraint intersection keeps provenance; unknown/non-Binary/missing resolve to `None`; `TypeRef::named` fabricates nothing; invalid provenance on `SignedInteger` and on a named base is rejected |
| `xsd-frontend` `binary_provenance` (4) | direct field, named chain (with facets), Choice alternative; `xs:base64Binary` stays `UnsupportedConstruct` |
| `xsd-frontend` `uci_binary_provenance` (2) | real UCI 2.5/2.6 inventories with exact counts (needs the pinned roots) |
| `codegen-core` `service_codec::tests` (3 new, 1 updated) | unknown provenance fails closed with the exact message; Base64 not ready; named Binary decided by ancestry |
| `codegen-core` `service_plan_binding::task052_changed_binary_lexical_provenance_is_rejected` | binding notices provenance on a direct field and a named base |
| `backend-rust` `service_codec` (2) | direct renderer emits the helpers for hexBinary; refuses stripped (unknown) provenance even when called directly |
| `cli` `service_codec` (3) | `codec-binary` flip with exact generated lines; helpers only on HexBinary surfaces; constrained hexBinary is a model blocker |
| `runtime-rust-facade-tests` `generated_codec_hexbinary` (7) | encoder corpus, valid/invalid decode corpora, JSON-type strictness, optional/repeated/named/Choice, diagnostics, `codec-binary` control |
| `generated_codec_mock_owp::task052_*` | mock OWP loop (§9) |
| `generated_codec_sleet_hexbinary` | real pinned Sleet loop + Sleet lexical-validation probe (§9) |
| `real_uci_subsystem_stream` (2) | real UCI 2.5 SubsystemStream codec and real Sleet loop (§8) |

CI adds two must-execute steps ("hexBinary provenance and Binary codecs",
"Real UCI Binary provenance inventory"; the latter uses the new
`scripts/fetch-pinned-uci-2.6.sh`, same pin and safe extractor as the Pages
build), extends the MSRV and real-Sleet steps, and adds two `run_one` lines
to `scripts/run-real-sleet-test.sh`. Every gate must match and pass exactly
one test. All Task 049–051 and GNAT gates are kept; the one renamed Task 050
gate is noted in the workflow.

### Local validation (baseline main `ef1e767` → this branch)

| Check | Result |
| --- | --- |
| `cargo fmt --all -- --check` | pass |
| `cargo check --workspace --all-targets` | pass |
| `cargo clippy --workspace --all-targets -- -D warnings` | pass (`ServiceApiExchangeKind` gets a documented `large_enum_variant` allow: the larger `TypeRef` pushed the inline binding past clippy's threshold) |
| `AMS_GRA_REQUIRE_GNAT=1 cargo test --workspace` | **1049 → 1077 passed**, 0 failed, 0 ignored |
| `git diff --check` | pass |
| Rust 1.95.0 checks (§10) | pass |
| real UCI 2.5/2.6 provenance inventories | pass |
| `scripts/run-real-sleet-test.sh` (pinned Sleet, with UCI 2.5 root) | pass: all six `run_one` tests (Task 049 handwritten, Task 050 generated, Task 051 non-OAM probe, **Task 052 hexBinary**, real PositionReport, **real SubsystemStream**) |

Tools: rustc/cargo 1.98.1 (stable), 1.95.0 (MSRV), GNAT 14.2.0,
Python 3.13.5 (`lxml` 5.4.0 for the evidence-only raw inventory).

## 12. Still open

* `xs:base64Binary`: not used by pinned UCI; frontend-unsupported; codec
  fail-closed if hand-built.
* Constrained Binary model carriers (octet `length`/`minLength`/`maxLength`).
* Ada and C++ generated codecs.
* Real UCI messages reaching `AtomicValueType` (blocked by cyclic value
  dependencies and open abstract values, not by Binary).

No generic Binary codec completeness is claimed.

