# Task 053 — Constrained named Binary model carriers

```text
Task 025:  Binary VALUE = owned sequence of octets.
Task 052:  Binary LEXICAL provenance (xs:hexBinary) + Rust OMS JSON codec.
Task 053:  Binary VALUE-SPACE constraint (length / minLength / maxLength,
           counted in OCTETS) renderable in Ada, Rust and C++ for NAMED
           Binary declarations.
```

Three independent dimensions, deliberately kept apart:

| Dimension | Question | Owner |
| --- | --- | --- |
| semantic value | "what is it?" — an octet sequence | `PrimitiveKind::Binary` |
| value-space constraint | "which octet sequences are legal?" — allowed octet count | `codegen_core::binary_length_domain` (this task) |
| lexical provenance | "how was it spelled in XSD / OMS JSON?" — `xs:hexBinary` | `ams_gra_oms_ir::declaration_binary_encoding` (Task 052) |

## 1. Fresh real-UCI inventory (recorded BEFORE production changes)

Pinned inputs, fetched by `scripts/fetch-pinned-uci-2.5.sh` /
`scripts/fetch-pinned-uci-2.6.sh` (root SHA-256 verified by the scripts):

* UCI 2.5 `093610b7753944059360d3236770ab446d039556`, root
  `03_OAC-STD-002_RevE_UCI_Schema_v2_5/UCI_MessageDefinitions_v2_5_0.xsd`
  (SHA-256 `ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27`);
* UCI 2.6 `78eb61b6112c8bffa40820c33124b57787fc5bd9`, root
  `UCI_MessageDefinitions_v2_6_0.xsd`
  (SHA-256 `af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b`).

Method: first a raw scan of every `xs:simpleType` whose restriction base is
`xs:hexBinary` or a named type that is itself a Binary restriction, with every
facet child. Then the same set is re-derived through the normalized IR,
reading the EFFECTIVE `ConstraintSet` and the shared ancestry query; this is
the executable gate `task053_real_uci_2_{5,6}_constrained_binary_inventory`
in `crates/codegen-core/tests/uci_constrained_binary.rs`. The two agree.
Neither release contains `xs:base64Binary`.

Namespace of every row: `http://www.vdl.afrl.af.mil/programs/oam`
(`uci:` prefix). "Lexical" is the Task 052 resolved
`BinaryLexicalEncoding`.

### 1.1 UCI 2.5

| Declaration | Source line | Immediate base | Full ancestry | Lexical | length | minLength | maxLength | Other facets | Class |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `AA_CodeType` | 109761 | `xs:hexBinary` | `AA_CodeType <- xs:hexBinary` | HexBinary | 6 | — | — | none | exact length |
| `BDS_AddressType` | 112165 | `xs:hexBinary` | `BDS_AddressType <- xs:hexBinary` | HexBinary | 2 | — | — | none | exact length |
| `SHA_2_256_HashType` | 141247 | `xs:hexBinary` | `SHA_2_256_HashType <- xs:hexBinary` | HexBinary | 32 | — | — | none | exact length |

UCI 2.5 has no unconstrained named Binary declaration (its five direct
`xs:hexBinary` members are field-level, Task 052).

### 1.2 UCI 2.6

| Declaration | Source line | Immediate base | Full ancestry | Lexical | length | minLength | maxLength | Other facets | Class |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `AA_CodeType` | 110189 | `xs:hexBinary` | `AA_CodeType <- xs:hexBinary` | HexBinary | 6 | — | — | none | exact length |
| `BDS_AddressType` | 112510 | `xs:hexBinary` | `BDS_AddressType <- xs:hexBinary` | HexBinary | 2 | — | — | none | exact length |
| `HexBinaryType` | 123231 | `xs:hexBinary` | `HexBinaryType <- xs:hexBinary` | HexBinary | — | — | — | none | unconstrained |
| `IFF_RegisterType` | 124838 | `uci:HexBinaryType` | `IFF_RegisterType <- HexBinaryType <- xs:hexBinary` | HexBinary | 7 | — | — | none | exact length |
| `SHA_2_256_HashType` | 141523 | `xs:hexBinary` | `SHA_2_256_HashType <- xs:hexBinary` | HexBinary | 32 | — | — | none | exact length |

### 1.3 Classification summary

| Class | UCI 2.5 | UCI 2.6 |
| --- | --- | --- |
| unconstrained | 0 | 1 (`HexBinaryType`) |
| exact length | 3 | 4 |
| bounded min/max length | 0 | 0 |
| one-sided length bound | 0 | 0 |
| unsupported extra facet | 0 | 0 |

No real declaration uses `minLength`/`maxLength`, `pattern`, a numeric facet,
or an explicit `whiteSpace`. The bounded and one-sided shapes are therefore
proven only by the synthetic fixture (section 5).

The executable gate re-derives this table from the normalized IR; its output
(`cargo test --release -p ams-gra-codegen-oms --test uci_constrained_binary`
with both roots set) matches the rows above exactly, including source lines,
and additionally proves each declaration renders in Ada, Rust and C++.

## 2. The shared classifier (`codegen-core::binary`)

```rust
pub struct BinaryLengthDomain { pub min_octets: u64, pub max_octets: Option<u64> }

pub fn binary_length_domain(kind: PrimitiveKind, constraints: &ConstraintSet)
    -> Result<Option<BinaryLengthDomain>, BinaryConstraintError>;
pub fn is_constrained_binary_carrier(kind, constraints) -> bool; // == Ok(Some(_))
```

| Result | Meaning | Output |
| --- | --- | --- |
| `Ok(None)` | unconstrained Binary | the existing Task 025 wrapper, **byte-identical** |
| `Ok(Some(domain))` | length-only octet domain | the new checked carrier |
| `Err(_)` | at least one facet outside the domain | fail closed, diagnostic names the facet and the declaration |

Normalization (every present facet participates; the `ConstraintSet` is
destructured exhaustively, so a future facet cannot reach a backend
unclassified):

| Effective facets | Domain |
| --- | --- |
| `length = N` | `N ..= N` |
| `minLength = A`, `maxLength = B` | `A ..= B` |
| `minLength = A` | `A ..` (unbounded) |
| `maxLength = B` | `0 ..= B` |
| none | unconstrained (`Ok(None)`) |
| `length` with compatible inherited `minLength`/`maxLength` | `length ..= length` |

Supported vs unsupported facet shapes:

| Facet | Verdict | Why |
| --- | --- | --- |
| `length`, `minLength`, `maxLength` | supported | counted in octets (XML Schema 2E 4.3.1-4.3.3) |
| explicit `whiteSpace = collapse` | accepted (classifier) | Binary's whiteSpace is fixed `collapse` (4.3.6): restating it is lexical only and removes no octet sequence. The XSD frontend still rejects `xs:whiteSpace` on Binary before it reaches IR, so this is reachable only from hand-built IR. |
| `whiteSpace = preserve` / `replace` | `Err(WhiteSpace)` | not Binary's fixed policy |
| `pattern` | `Err(Pattern)` | constrains the LEXICAL form, which stored octets cannot enforce |
| `min/maxInclusive`, `min/maxExclusive` | `Err(NumericFacet)` | not a Binary value-space facet |
| contradictory length facets | `Err(ContradictoryLength)` | IR validation already rejects; re-checked, not trusted |

The classifier **never consults lexical provenance**: a hand-built Binary with
unknown provenance and `length = 4` is model READY (parity test, section 5)
and codec NOT READY (Task 052 `UnknownBinaryEncoding`).

### Consumers (one classifier, no backend-specific reading)

* `coverage::primitive_declaration_renderable` — replaces
  `constraints == ConstraintSet::default()` with `binary_length_domain(..).is_ok()`;
* `backend-{ada,rust,cpp}` validation and rendering;
* `backend_names::ada_wrapper_callable_owners` — Ada `Create`/`Value`;
* `backend-rust::service_codec` — checked vs infallible decode.

### Named restriction inheritance

The frontend intersects facets along the restriction chain; the classifier
reads only the EFFECTIVE set. The synthetic ancestry (fixture
`constrained-binary.xsd`) proves it:

| Declaration | Authored | Effective domain | Lexical (Task 052 query) |
| --- | --- | --- | --- |
| `BlobBase` | `xs:hexBinary`, `minLength 4` | `4 ..` | HexBinary |
| `BlobMiddle` | `BlobBase`, `maxLength 16` | `4 ..= 16` | HexBinary |
| `BlobExact` | `BlobMiddle`, `length 8` | `8 ..= 8` | HexBinary |
| `DerivedExact4` | `Between2And6`, `length 4` | `4 ..= 4` | HexBinary |

`declaration_binary_encoding` walks `base_type` for provenance;
`binary_length_domain` reads the already-folded constraints. Neither
duplicates the other's traversal. Real UCI 2.6 `IFF_RegisterType <-
HexBinaryType <- xs:hexBinary` is the real-world instance.

## 3. Generated carriers

### Rust

Unconstrained (unchanged): `pub struct Blob(Vec<u8>)` with infallible
`new(Vec<u8>) -> Self`.

Constrained:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Exact4(Vec<u8>);

impl Exact4 {
    pub const MIN_OCTETS: u64 = 4;
    pub const MAX_OCTETS: u64 = 4;       // omitted when unbounded above

    pub fn new(value: Vec<u8>) -> Option<Self> {
        let len = u64::try_from(value.len()).ok()?;
        if len >= Self::MIN_OCTETS && len <= Self::MAX_OCTETS { Some(Self(value)) } else { None }
    }
    pub fn as_slice(&self) -> &[u8] { &self.0 }
    pub fn into_vec(self) -> Vec<u8> { self.0 }
}
```

* No unchecked constructor, no `Default`; the tuple field is private to the
  generated module.
* `usize` is widened to `u64` at run time; no schema bound is narrowed to the
  host's `usize` at generation time. An always-true `len >= 0` is omitted
  (e.g. `Max6`), and a domain admitting every length (`minLength 0`, no max)
  stores directly with no `len` at all, so the output is lint-clean.
* **No move workaround.** Moving a Rust value makes the source statically
  unusable; `Clone` copies an already valid value. Nothing clone-on-move was
  added.

### C++

Unconstrained (unchanged): public `explicit Blob(std::vector<std::uint8_t>)`.

Constrained:

```cpp
class Exact4 {
public:
    static constexpr std::uint64_t min_octets = 4u;
    static constexpr std::uint64_t max_octets = 4u;

    static std::optional<Exact4> create(std::vector<std::uint8_t> value) {
        const auto octets = static_cast<std::uint64_t>(value.size());
        if (octets < min_octets || octets > max_octets) return std::nullopt;
        return Exact4(std::move(value));
    }

    const std::vector<std::uint8_t>& value() const noexcept { return value_; }

    Exact4(const Exact4&) = default;
    Exact4& operator=(const Exact4&) = default;

private:
    explicit Exact4(std::vector<std::uint8_t> validated) : value_(std::move(validated)) {}
    std::vector<std::uint8_t> value_;
};
```

**Lifecycle (Task 040 policy).** Declaring the copy operations suppresses the
implicit move constructor and move assignment. `auto b = std::move(a);` and
`c = std::move(b);` therefore COPY, and the still-live source keeps its valid
octets; a synthesized move would have emptied the `std::vector` and broken
`minLength > 0`. The trade-off is an allocation per rvalue transfer, accepted
to keep the invariant. `create` and the copy operations are not `noexcept`
(copying allocates); only the accessor is. No default constructor exists
(any user-declared constructor suppresses it), including for `ZeroBlob`.

### Ada

Unconstrained (unchanged): `type Blob is record Value : Binary_Vectors.Vector; end record;`.

Constrained (visible part):

```ada
type Exact4 is private;
function Create (Value : Binary_Vectors.Vector) return Exact4;
function Value (Item : Exact4) return Binary_Vectors.Vector;
```

Private part (expression functions, so **no package body is needed**):

```ada
type Exact4 is record
   Data : Binary_Vectors.Vector :=
     raise Standard.Program_Error
       with "Exact4 requires initialization from Create";
end record;

function Create (Value : Binary_Vectors.Vector) return Exact4
is (if Interfaces.">=" (Interfaces.Unsigned_64 (Binary_Vectors.Length (Value)), 4)
      and then Interfaces."<=" (Interfaces.Unsigned_64 (Binary_Vectors.Length (Value)), 4)
    then Exact4'(Data => Value)
    else raise Standard.Constraint_Error with "Exact4 requires exactly 4 octets");

function Value (Item : Exact4) return Binary_Vectors.Vector is (Item.Data);
```

* The check is an explicit `raise`, independent of `-gnata` and
  `Assertion_Policy`. The count is widened to `Unsigned_64`; prefix-call
  operators are used because the package has no `use Interfaces`.
* Default initialization raises `Program_Error` by policy **even when the
  domain admits zero octets** (`ZeroBlob`): an empty VALUE is legal through
  `Create (Empty_Vector)`, an unchecked default is not.
* **Package body / artifacts.** No `.adb` is added for a constrained Binary
  carrier; the shared body predicate is unchanged. The synthetic fixture
  does emit `programs-oam.adb`, solely because of its repeated storage (the
  existing Task 040 predicate), and `service-check` / `generate` agree.

### Ada generated-name preflight

`Create` and `Value` are package-scope overloads. They join the existing
shared `ada_wrapper_callable_owners` / `collect_ada_wrapper_callable_conflicts`
analysis (no Binary-only checker), and are reserved **only** when the
classifier returns `Ok(Some(_))`:

* coexist with String/DateTime/floating `Create`/`Value` overloads;
* collide with a non-overloadable declaration named `Create` or `Value`
  (both sides attributed);
* an unconstrained or unsupported (pattern) Binary reserves nothing.

No new package-scope helper name was introduced (only `Create`, `Value`, and
the private record component `Data`). Rust and C++ add member functions only.

### Ada repeated storage

`ada_sequence_shape` and the Task 040 storage are element-agnostic: every
repeated field already uses the opaque `Indefinite_Vectors` path (unbounded)
or the discriminated-slot array (bounded). A constrained Binary element
therefore needed no new predicate and no new container: spare capacity is an
access value or a payload-free slot, so the failing default is never
evaluated. Proven at run time under GNAT (section 5).

## 4. Codec integration (Rust `--with-codec`)

| Named Binary | Decode | Encode |
| --- | --- | --- |
| unconstrained (unchanged) | `decode_hex_binary(value, path).map(T::new)` | `encode_hex_binary(value.as_slice())` |
| constrained | `decode_hex_binary(value, path).and_then(\|x\| T::new(x).ok_or_else(\|\| rejected(path, "T")))` | `encode_hex_binary(value.as_slice())` |

No octet bound appears in `service_codec.rs`: the generated MODEL `T::new` is
the single authority. A lexically valid value of an illegal octet count is
reported at the member path as a model rejection, e.g.

```text
ConstrainedBlobPayload.Exact: value rejected by generated Exact4::new
ConstrainedBlobPayload.Bounded[1]: value rejected by generated Max6::new
```

never as `is not hexBinary`. A genuine lexical error is still a lexical error.

## 5. Evidence

### Synthetic fixture

`tests/fixtures/service-generate/constrained-binary.{xsd,yaml}` (OAM
namespace): `Exact4`, `Min2`, `Max6`, `Between2And6`, `DerivedExact4`,
`ZeroBlob`, `BlobBase`/`BlobMiddle`/`BlobExact`, and the unconstrained
`PlainBytes` control, used as required, optional, bounded `0..3` and `1..3`,
unbounded `0..` and `1..`, and Choice alternatives. `service-check` is READY
in Ada, Rust and C++ (12/12 types) and Rust `--with-codec` is codec READY
(12/12).

| Test | Proves |
| --- | --- |
| `codegen-core` `binary::tests::task053_*` | classifier shapes, collapse, every rejection |
| `codegen-core` `backend_names::tests::task053_*` | Ada `Create`/`Value` collide/coexist/not-reserved |
| `cli` `constrained_binary_parity` | coverage == service-check == generation for unconstrained, exact, exact-0, min, max, min+max, pattern, numeric, direct `length`/`minLength`/`maxLength`; direct facets fail in required/optional/bounded/unbounded occurrences |
| `backend-rust` / `backend-cpp` / `backend-ada` unit tests | length-only renders; pattern/numeric fail with the facet named |
| `runtime-rust-facade-tests` `generated_constrained_binary` | Rust bounds, `as_slice`/`into_vec`, zero-length, composition, codec round trip, invalid-length diagnostics |
| `backend-cpp` `constrained_binary` | strict C++17 (`-Wall -Wextra -Werror -pedantic-errors`) bounds, const-ref accessor, copy/rvalue-copy lifecycle, `ZeroBlob`, optional/Bounded/Unbounded/variant composition; negative probes: vector constructor (private), default (no matching function, incl. `ZeroBlob`), `value_` (private) |
| `backend-ada` `constrained_binary` | GNAT: bounds, `Constraint_Error`, default `Program_Error` (incl. `ZeroBlob`), copy, Task 034 optional, bounded 0..3 / 1..3, unbounded 0.. / 1.., append/element/clear/reserve, invalid length rejected before insertion, Choice — normal policy **and** `Assertion_Policy (Ignore)` |
| `generated_codec_mock_owp::task053_*` | typed carrier -> generated codec -> Publish -> SleetRuntime -> mock OWP; canonical uppercase; lowercase legal MSG reaches handler; 3-octet `Exact4` is a decode event, no handler |
| `generated_codec_sleet_constrained_binary` | same through unmodified pinned Sleet |
| `cli` `uci_constrained_binary` | real UCI 2.5/2.6 inventory, per-declaration rendering, message impact |
| `cli` `service_codec::task052_constrained_hex_binary_remains_a_model_blocker` | flipped: length-only now READY; pattern / whiteSpace / numeric still rejected |

### Direct field-local Binary constraints stay fail-closed

`field Data : Primitive(Binary)` with a local `length`, `minLength` or
`maxLength` is NOT renderable in any backend, in any occurrence, and the
diagnostic is `field constraints on Data` (the field-local facet), not
Binary. No per-field carrier was created.

## 6. Fresh 12-cell coverage matrix (before -> after)

BEFORE: clean `origin/main` `edd30087` built in a separate worktree; AFTER:
this branch. Both release binaries run `coverage --schema <pinned root>
--world <world>` on the same roots. Raw outputs are preserved outside the
repository as eight separate files (no historical Task 033-052 evidence was
rewritten):

| File | SHA-256 |
| --- | --- |
| `uci25-closed-schema-before.txt` | `7a9e07e2d8e8f70fe9f3e7afd6f91828e5065b2cdeefd4252554f1f3bbe27b41` |
| `uci25-closed-schema-after.txt` | `19670d720e9a9375520b03b1899c4d15222fb3af1ad955427b61664c6a6b9459` |
| `uci25-open-extensions-before.txt` | `ae29d99206b970fc9cb7f37df315d08620ff5efd75c71e3b865c25604d0074cd` |
| `uci25-open-extensions-after.txt` | `3136e3e9fe5624ea5928508d06384ee22ac5cc305b07c2935dcb9725749fd690` |
| `uci26-closed-schema-before.txt` | `cd138fac9b335ff6417b4255d7265167c2aea6195de05aa58b8c0671a3d73723` |
| `uci26-closed-schema-after.txt` | `868959b61c62315d1c9f27f7a1a28b2d14144a184524431e88e5e3baa53161b2` |
| `uci26-open-extensions-before.txt` | `329d10236bfa3d1f5cda7b0ebef181cf1788516b9afacd7561e95c4a12dfc7dc` |
| `uci26-open-extensions-after.txt` | `af5436c07f9f76f53052d4b3ae1806aa38e2f318f5c95feea47df893eb3ead1d` |

The only differing lines in each before/after pair are the three
`<Backend>: ... declarations N/M` summary lines (and wall-clock timing).

| Release | World | Backend | Declarations | Field-type refs | Field occurrences | Message closures |
| --- | --- | --- | --- | --- | --- | --- |
| 2.5 | closed | Ada | 5365 -> **5368** /5557 | 13151/13160 = | 13153/13160 = | 324/722 = |
| 2.5 | closed | Rust | 5407 -> **5410** /5557 | 13151/13160 = | 13160/13160 = | 354/722 = |
| 2.5 | closed | C++ | 5410 -> **5413** /5557 | 13151/13160 = | 13160/13160 = | 351/722 = |
| 2.5 | open | Ada | 5280 -> **5283** /5557 | 13151/13160 = | 13153/13160 = | 317/722 = |
| 2.5 | open | Rust | 5319 -> **5322** /5557 | 13151/13160 = | 13160/13160 = | 347/722 = |
| 2.5 | open | C++ | 5322 -> **5325** /5557 | 13151/13160 = | 13160/13160 = | 344/722 = |
| 2.6 | closed | Ada | 5385 -> **5389** /5570 | 13198/13198 = | 13198/13198 = | 323/725 = |
| 2.6 | closed | Rust | 5427 -> **5431** /5570 | 13198/13198 = | 13198/13198 = | 354/725 = |
| 2.6 | closed | C++ | 5431 -> **5435** /5570 | 13198/13198 = | 13198/13198 = | 352/725 = |
| 2.6 | open | Ada | 5300 -> **5304** /5570 | 13198/13198 = | 13198/13198 = | 316/725 = |
| 2.6 | open | Rust | 5339 -> **5343** /5570 | 13198/13198 = | 13198/13198 = | 347/725 = |
| 2.6 | open | C++ | 5343 -> **5347** /5570 | 13198/13198 = | 13198/13198 = | 345/725 = |

Every delta is explained:

* **Declarations +3 (2.5) / +4 (2.6)** in every cell: exactly the constrained
  named Binary declarations of section 1 (`AA_CodeType`, `BDS_AddressType`,
  `SHA_2_256_HashType`, plus 2.6 `IFF_RegisterType`). No other declaration
  flipped. Kind counts are unchanged because Binary was already a
  renderable *kind*.
* **Field-type references / occurrences: unchanged**, as expected: this task
  touches no direct-field capability.
* **Message closures: unchanged**: every closure reaching a constrained
  Binary has another blocker (section 7).

### Full-schema generation first blocker

`generate` over the whole pinned schema was also re-run in all 12 cells (plus
the Task 029-style closed-world probe overlay adding one synthetic concrete
`SourceCommandEXT` descendant, uncommitted, in `/tmp`). The first blocker is
**identical before and after** in every cell; it is no longer
`constraints on AA_CodeType` because intervening tasks (Task 044 onwards)
already moved the full-schema first failure to the shared reserved-word
name preflight:

| Release | Backend | First blocker (before = after; closed, open, and overlay) |
| --- | --- | --- |
| 2.5 | Ada | `Ada name "Range" generates reserved word "Range" in the members of AltitudeRangePairType` |
| 2.5 | Rust | `Rust name "Type" generates reserved word "type" in the members of ConfigurationParameterType` |
| 2.5 | C++ | `C++ name "Operator" generates reserved word "operator" in the members of ApprovalResponseType` |
| 2.6 | Ada | `Ada name "Range" generates reserved word "Range" in the members of AltitudeRangePairType` |
| 2.6 | Rust | `Rust name "Type" generates reserved word "type" in the members of ConfigurationParameterType` |
| 2.6 | C++ | `C++ name "Delete" generates reserved word "delete" in the members of COMINT_ChangeDwellType` |

The historical Task 029 `constraints on AA_CodeType` row for 2.6 therefore
describes an older first failure; the `AA_CodeType` constraint is now
supported (proven per declaration by `uci_constrained_binary`), and the
full-schema first blocker is a name-remapping issue out of scope here. Not
widened.

## 7. Real message-closure impact

For each release, every message whose payload closure reaches a constrained
Binary was given its own one-message contract and passed through the real
selected-service readiness (projection + coverage + preflight), for every
backend, `closed-schema` (`task053_real_uci_constrained_binary_message_impact`):

| Message | Closure (2.5 / 2.6) | Via | Ada | Rust / C++ | Class |
| --- | --- | --- | --- | --- | --- |
| `FileMetadata` | 52 / 54 | `SHA_2_256_HashType` | `FileNameType` | `FileNameType` | B |
| `IFF_Activity` | 225 / 227 | `AA_CodeType`, `BDS_AddressType` (+2.6 `IFF_RegisterType`) | `AltitudeRangePairType` | `AircraftIdentifierType` | B |
| `IFF_Command` | 243 / 246 | same | `AltitudeRangePairType` | `AircraftIdentifierType` | B |
| `ProductMetadata` | 181 / 183 | `SHA_2_256_HashType` | `DateTimeRangeType` | `AlphanumericStringLength4Type` | B |
| `Response` | 1278 / 1291 | `AA_CodeType`, `BDS_AddressType` (+2.6 `IFF_RegisterType`) | cyclic value dependencies | cyclic value dependencies | C |

* **A** (fully READY because Task 053 removed its last blocker): **0** in
  either release.
* **B** (constrained Binary fixed, another model blocker remains): 4.
* **C** (projection/topology failure): 1 (`Response`, `QueryMatchType ->
  QueryPET -> QueryType -> QueryMatchType`).

The test asserts no constrained Binary is ever the reported blocker.

### No category-A message, so no real selected-service milestone

As required, no milestone was forced. The **synthetic** `constrained-binary`
service is the cross-language compile/run evidence instead: `service-check`
READY and `service-generate` succeeding in Ada, Rust and C++, the generated
Ada compiling and running under GNAT, the generated C++ header compiling
under strict C++17, and the generated Rust model + codec compiling (also
under Rust 1.95.0) and running, including through mock OWP and real pinned
Sleet. Real `--with-codec` evidence likewise uses the synthetic service.

## 8. Runtime boundary evidence

### Mock OWP (synthetic `constrained-binary`)

`generated_codec_mock_owp::task053_constrained_binary_round_trips_through_mock_owp`:
typed constrained carriers -> generated `ServiceCodec` -> generated `publish`
-> `SleetRuntime` -> mock OWP. The PUB body is canonical uppercase
(`"Exact":"000AEEFF"`, `"Hashed":"EEEE..."`, `"Zero":""`, Choice
`{"Fixed":"DEADBEEF"}`); an MSG with lowercase hex of legal octet counts
reaches the typed handler as exactly the same checked carriers; an MSG with
`"Exact":"000aee"` (lexically valid, 3 octets) produces
`SubscriptionDecodeError("ConstrainedBlobPayload.Exact: value rejected by
generated Exact4::new")` and no handler call.

### Real pinned Sleet (`e38f61d8`)

A new test, `generated_codec_sleet_constrained_binary`, beside the unchanged
Task 052 one, run by `scripts/run-real-sleet-test.sh`.

**Recorded external divergence.** Pinned Sleet
(`sleet/src/facets.rs`, `lexical.chars().count()`) evaluates `xs:length`/
`minLength`/`maxLength` of a hexBinary value in lexical **characters**; XML
Schema 2E 4.3.1 defines them in **octets**. Observed:

```text
PUB with a legal 4-octet Exact4 ("DEADBEEF", 8 characters)
  -> ServerError Invalid-Message: FacetViolation { path: "SleetBlobNotice.Exact",
     facet: "length", detail: "length 8 != 4" }          (not routed)
PUB with Between2And6 = "AB" (1 octet, 2 characters; ILLEGAL, 2..6 octets)
  -> routed by Sleet; GENERATED decode:
     "SleetBlobPayload.Ranged: value rejected by generated Between2And6::new"
     (no handler call)
```

The generated codec was **not** loosened to match Sleet; the generated carrier
stays the authority. Because every required member of the main synthetic
fixture is exact-length (always rejected by pinned Sleet unless zero octets),
the real-Sleet test uses a companion fixture,
`constrained-binary-sleet.{xsd,yaml}` (`Between2And6`,
`BlobMiddle <- BlobBase`, `ZeroBlob`, optional `Exact4`, unbounded
`Between2And6`), where legal octet counts that also satisfy Sleet's character
count exist. With it: legal byte lengths route through unmodified Sleet and
reach the typed handler as identical checked carriers; the encoder emits
canonical uppercase (`"ABCD"`, `"000AEEFF"`, `""`); the decoder reconstructs
the carriers. The same consequence applies to real UCI exact-length Binary
(`SHA_2_256_HashType` = 32 octets = 64 characters) routed through pinned
Sleet; no real UCI message with one is READY, so this is recorded rather than
exercised.

## 9. Regressions and compatibility

### Task 052 Binary regressions (unchanged tests, all pass)

Direct canonical encoder corpus, valid and invalid lexical decoder corpora,
whitespace collapse, optional/repeated direct Binary, Choice
`HexBinaryValue`, unknown provenance, `base64Binary` unsupported, the
frontend and IR provenance tests, the mock OWP hexBinary loop, the real
pinned-Sleet hexBinary test, and real UCI 2.5 `SubsystemStream` through real
Sleet. The one Task 052 test whose premise this task deliberately changes,
`task052_constrained_hex_binary_remains_a_model_blocker`, now asserts
length-only READY and pattern / whiteSpace / numeric still rejected.

### Unconstrained Binary and broad output identity

Clean `origin/main` release binary vs this branch, over every XSD fixture
in `crates/xsd-frontend/tests/fixtures` and
`tests/fixtures/service-generate` (model `generate`, Ada/Rust/C++) plus every
`service-generate` contract (Ada/Rust/C++ and Rust `--with-codec`):
**432 identical, 0 exit-code changes, 2 diagnostic changes.** Every
unconstrained Binary fixture (`backend-binary-only.xsd`, `codec-binary`,
`codec-hexbinary`, `optional-primitive`, ...) is byte-identical in every
backend, wrapper and codec. The two changes are the Rust and C++ first error
for `scalar-restrictions.xsd`: its `BinaryExact` (`length 6`) is now
supported, so generation proceeds to the next, pre-existing unsupported
declaration (`unsupported constrained String declaration ... on StringRule`);
both still exit 1. (Ada already failed earlier there on the reserved word
`At` and is unchanged.)

### PositionReport (real UCI 2.5) regression

Before/after `service-check` reports and `service-generate` outputs are
byte-identical for Ada, Rust, C++ and Rust `--with-codec`: model 60/60
READY, codec 59/59 READY; model files, `service_api.rs`, and
`service_codec.rs` unchanged. `SubsystemStream` likewise identical (40/40).
The real PositionReport and SubsystemStream Sleet tests pass.

## 10. Scope boundaries honored

* No model change for unconstrained Binary (classifier `Ok(None)` falls
  through to the unchanged Task 025 renderers).
* No direct field-local Binary constraint support; no per-field carrier.
* No Binary pattern implementation; no `base64Binary` support.
* No new dependency; no Task 052 provenance change.
* No committed UCI or Sleet checkout (both fetched to `/tmp`, pinned, hashed).
* No name-remapping / full-schema cleanup (the reserved-word first blockers
  are recorded, not touched).
* Task 040 lifecycle guarantees are extended, not weakened: C++ copy-only,
  Ada failing default, Ada repeated storage unchanged.

## 11. CI must-execute gates

Two new steps in `.github/workflows/ci.yml`, both zero-match-fails:

* **Constrained Binary carriers (Task 053, must execute)** — the 9
  classifier / Ada-name tests, 2 parity tests, 3 C++ strict tests, 2 GNAT
  tests (with `AMS_GRA_REQUIRE_GNAT=1`, and a check that the Ada probe
  printed PASSED under both assertion policies), 4 generated Rust
  carrier/codec tests, and the mock OWP loop;
* **Real UCI constrained Binary inventory (Task 053, must execute)** —
  `uci_constrained_binary` with both pinned roots: 2.5 and 2.6 inventory and
  message-impact PASSED lines and exactly 3 tests.

The real-Sleet test runs through `scripts/run-real-sleet-test.sh` (already a
CI step). The Rust 1.95.0 step compiles the new generated
`constrained_binary` / `constrained_binary_sleet` services through the
facade-tests crate. All existing GNAT, Task 049 runtime, Task 050-052 codec,
Rust 1.95 and real-Sleet gates are unchanged.

## 12. Local validation (baseline main `edd30087` -> this branch)

| Check | Result |
| --- | --- |
| `cargo fmt --all -- --check` | pass |
| `cargo check --workspace --all-targets` | pass |
| `cargo clippy --workspace --all-targets -- -D warnings` | pass |
| `AMS_GRA_REQUIRE_GNAT=1 cargo test --workspace` | **1077 -> 1102 passed**, 0 failed, 0 ignored |
| `git diff --check` | pass |
| Task 053 CI gate script, run locally | pass |
| Rust 1.95.0 `check --locked` (runtime-api, runtime-rust, facade-tests, and facade-tests with the UCI 2.5 root) | pass |
| strict C++17 / GNAT probes | pass (`-std=c++17 -Wall -Wextra -Werror -pedantic-errors`; GNAT default and `Assertion_Policy (Ignore)`) |
| real UCI 2.5/2.6 inventory + message impact | pass |
| `scripts/run-real-sleet-test.sh` (pinned Sleet, UCI 2.5 root) | pass: all seven `run_one` tests (Task 049, 050, 051 probe, 052 hexBinary, **053 constrained Binary**, real PositionReport, real SubsystemStream) |

Tools: rustc/cargo 1.98.1 (stable), 1.95.0 (MSRV), GNAT (gnatmake) 14.2.0,
g++ 14.2.0, Python 3.13.5.

## 13. Still open

* [ ] direct field-local Binary constraints;
* [ ] Binary lexical patterns;
* [ ] `xs:base64Binary` support;
* [ ] Ada/C++ OMS JSON codecs;
* [ ] Ada/C++ LA-CAL runtimes;
* the full-schema reserved-word name blockers and the non-Binary blockers of
  the five real messages that reach a constrained Binary (section 7).
