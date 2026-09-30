# Task 058 — Simple bounded ASCII String profiles

Status: implemented. MODEL-CAPABILITY task. Base: `main` @
`ef29da0e53d50d955f8cd3004e5092b787e49f47`, the Task 057 merge. At that commit
Fast CI 36654133221, UCI Pages 36654133220 and Deep CI 36654133204 had all
passed.

Task 058 makes one evidence-bounded family of named constrained `xs:string`
declarations renderable in Ada, Rust and C++, together with the Rust OMS JSON
codec. The family covers exactly one shape: **one finite ASCII character class
under one bounded quantifier**, in one of two forms:

* `[CLASS]{N}` with `length = N`;
* `[CLASS]{M,N}` with `minLength = M` and `maxLength = N`.

It is modelled once, in the shared classifier `codegen-core::string_profile`,
as `StringProfile::BoundedAscii { alphabet, length }`.

This is **not** a regular-expression engine and **not** a general
character-class facility. The admitted domain is exactly the
`(alphabet, class spelling, length)` rows observed in the pinned releases.

## 1. Evidence gate (before any production change)

The pinned roots were fetched fresh with `scripts/fetch-pinned-uci-2.{5,6}.sh`:

* UCI 2.5 `093610b7753944059360d3236770ab446d039556`, root SHA-256
  `ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27`;
* UCI 2.6 `78eb61b6112c8bffa40820c33124b57787fc5bd9`, root SHA-256
  `af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b`.

On unchanged `main`, we enumerated from the **normalized IR** every named
`PrimitiveKind::String` declaration that `string_profile` rejected. For each
one we recorded:

* ancestry and restriction depth;
* every pattern group and alternative, with dialect and exact expression;
* `length`, `minLength`, `maxLength`, the explicit and effective `whiteSpace`,
  and any numeric facets;
* declared and effective references, and any field-local facet or `nillable`;
* the message closures that reach it.

Representative rows were cross-checked against the raw XSD:

* `AircraftIdentifierType` is written `[A-Z0-9&#x20;]{8}`, and the character
  reference reaches the IR as a literal SPACE;
* `USMTF_MessageSerialNumberType` spells its class with `&amp;`, `&quot;`
  and `&lt;`.

Both releases carry **107** unsupported constrained String declarations, and
the names are identical. All 107 are depth-1 restrictions of `xs:string`.
None has an explicit `whiteSpace`, so the effective policy is `preserve`.
None has a numeric facet. No reference to any of them has a field-local facet
or `nillable`.

There are **0** direct field-local constrained `xs:string` members. The only
cross-release difference among the 107 is the pattern of `MilitaryGridType`,
which is excluded in both.

**The strict gate.** A declaration qualifies only if it has:

* one pattern group holding one XML-Schema expression;
* an expression of the form `[CLASS]{N}` or `[CLASS]{M,N}`;
* a class built only from single characters, escaped single characters and
  ranges, with no negation, subtraction or multi-character escape;
* only ASCII members, and no control character or DEL;
* either `length = N` alone, or `minLength = M` plus `maxLength = N`, agreeing
  with the quantifier.

**61** declarations per release qualify, and they are the same 61 in both
releases. Both `EmptyType` (`[a-zA-Z]{0}`, `length 0`) and
`AircraftIdentifierType` (`[A-Z0-9 ]{8}`, `length 8`) are members.

### Admitted domain: 19 alphabets, 60 rows, 61 declarations

Two declarations share a row: `ATO_TargetFacilityNameType` and
`USMTF_AircraftCallSignType` are both `UsmtfText 1..38`.

| Alphabet | Members | Normalized spelling(s) | Length rows | Declarations |
| --- | --- | --- | --- | --- |
| `Letters` | A-Z a-z | `[a-zA-Z]` | {0} | `EmptyType` |
| `UpperAlphanumericSpace` | SPACE 0-9 A-Z | `[A-Z0-9 ]` | {4} {8} | `Link16_VoiceCallSignType`, `AircraftIdentifierType` |
| `Alphanumeric` | 0-9 A-Z a-z | `[a-zA-Z0-9]` | {4} {7} 1..4 1..6 1..20 1..25 1..54 | `AlphanumericStringLength{4,7}Type`, `AlphanumericString{4,6,20,54}Type`, `UserIdentifierType` |
| `AlphanumericSpaceHyphenUnderscore` | SPACE - 0-9 A-Z _ a-z | `[a-zA-Z0-9 \-_]` | {12} {15} {20} 1..3 1..20 1..40 | `AlphanumericDashSpaceUnderscoreStringLength{12,15,20}Type`, `AlphanumericDashSpaceUnderscoreString20Type`, `OB_RecordOwnerType`, `MISP_ClassificationType` |
| `AlphanumericSpace` | SPACE 0-9 A-Z a-z | `[A-Za-z0-9 ]` | {15} | `AlphanumericSpaceStringLength15Type` |
| `AlphanumericHyphen` | - 0-9 A-Z a-z | `[a-zA-Z0-9\-]` | {15} | `NameSpecialCharacterRestrictionType` |
| `AlphanumericUnderscoreHyphen` | - 0-9 A-Z _ a-z | `[a-zA-Z0-9_\-]`, `[A-Za-z0-9_\-]` | 1..5 1..12 1..20 1..54 1..128 | `OB_CodeWordType`, `EOB_CED_NameType`, `EOB_CED_WeaponSystemType` (the second spelling), `OB_FacilityNameType`, `UCI_SchemaComponentNameType` |
| `AlphanumericSpaceHyphenPeriod` | SPACE - . 0-9 A-Z a-z | `[a-zA-Z0-9 \-\.]` | 1..20 | `OperatorPhoneNumberType` |
| `UpperLetters` | A-Z | `[A-Z]` | {2} {4} 1..3 | `NITF_IPON_IID2_ProjectCodeType`, `ICAO_AirfieldIdentifierType`, `LaunchPieceType` |
| `Digits` | 0-9 | `[0-9]` | {4} {5} {6} {9} 4..5 | `AO_PIM_CodeType`, `NumericStringLength{5,6}Type`, `MMSI_NumberType`, `OneUpNumberType` |
| `OctalDigits` | 0-7 | `[0-7]` | {4} {5} | `OctalStringLength4Type`, `IJMS_TrackNumberType` |
| `UpperAlphanumeric` | 0-9 A-Z | `[A-Z0-9]` | 1..3 2..3 | `OB_ActivityCodeType`, `OB_LastCollectorType` |
| `UpperAlphanumericHyphen` | - 0-9 A-Z | `[A-Z0-9\-]` | 2..10 | `TailNumberType` |
| `UsmtfText` | SPACE ( ) , - . 0-9 ? A-Z | `[\-\.,\(\)\?A-Z0-9 ]` | 1..8 1..24 1..30 1..32 1..38 1..56 | `USMTF_MissionNumberType`, `USMTF_UnitDesignatorType`, `ATO_DMPI_IdentifierType`, `USMTF_OperationCodewordType`, `ATO_TargetFacilityNameType`, `USMTF_AircraftCallSignType`, `USMTF_ExerciseNicknameType` |
| `UsmtfTextWithoutSpace` | ( ) , - . 0-9 ? A-Z | `[\-\.,\(\)\?A-Z0-9]` | 1..5 | `ATO_PackageIdentificationType` |
| `UsmtfTextUnderscore` | SPACE ( ) , - . 0-9 ? A-Z _ | `[\-\.,\(\)_\?A-Z0-9 ]` | 1..30 | `USMTF_OriginatorType` |
| `UsmtfSerialText` | U+0020..U+002E, 0-9, U+003B..U+0060, U+007B..U+007E | see `USMTF_SERIAL_CLASS` in the classifier | 1..7 | `USMTF_MessageSerialNumberType` |
| `UnitNameText` | SPACE ' ( ) + , - . 0-9 ; @ A-Z a-z | `[a-zA-Z0-9 '\(\).,@;+\-]` | 1..256 | `UnitNameType` |
| `VisibleAscii` | U+0020..U+007E | `[ -~]` | {2} {6} {10} {12} {15} {17} {18} {20} {24} {40} {42} {43} {80} | `VisibleStringLength{10,12,15,17,20,80}Type` and 7 fixed-width `NITF_*` types |

The Deep CI inventory prints one row per declaration (`UCI 2.x CONSTRAINED
STRING: …`). Each row gives the source line, ancestry, exact pattern, facets,
reference counts and the number of reaching messages. Some examples, with
message-closure counts in 2.5:

* `EmptyType`: 274 declared / 394 effective references (2.6: 305 / 430), 283
  messages;
* `AircraftIdentifierType`: 3 / 3 references, 72 messages;
* `LaunchPieceType`: 83 messages.

### Excluded neighbours (46 per release; they still fail closed)

| Reason | Count | Examples |
| --- | ---: | --- |
| unquantified single class under `length 1` (possible follow-up) | 9 | `NumericStringLength1Type` `[0-9]`, `MeterUnitLetterType` `[m]`, `NITF_FileSecurityClassificationType` `[TSCRU]` |
| `maxLength` only; `minLength` is never inferred from the quantifier | 2 | `Link16_SpecificTypeModelType` `[A-Za-z0-9]{1,4}`, `MISP_ItemDesignatorType` `[a-zA-Z0-9 \-_]{1,16}` |
| position-specific concatenated classes | 8 | `CounterSpaceSENO_Type` `[A-Z][IRS][0-9]{3}`, `OB_O_SuffixType`, `Link16_TrackNumberType`, `UnitIdentifierType` |
| alternation, or several alternatives in one group | 19 | `NotationType`, `FIPS_CountryCodeType`, `RecordOriginatorType`, IPv4/IPv6, `MilitaryGridType`, NITF date/time families |
| literal prefix, grouping, optional pieces, or unbounded `+` | 8 | `IMO_NumberType` `IMO[0-9]{7}`, `FileNameType`, `OctalValueType` `[0-7]+`, `AO_PRF_CodeType` `1?[1-7][1-8]{2}` |

The five earlier profiles are unchanged: schema-version, UUID, visible-ASCII
(`[ -~]{M,N}` under `minLength`/`maxLength`), whitespace-visible, and NATO
Special Words. None of their declarations is reclassified. The fixed-`length`
`[ -~]{N}` family was always a *different* declaration shape from Task 039's
visible-ASCII family, and it is now the `VisibleAscii` bounded-ASCII alphabet.

## 2. Shared semantic model

The model lives in `crates/codegen-core/src/string_profile.rs`:

```rust
pub enum StringProfile {
    /* five existing profiles, unchanged */
    BoundedAscii { alphabet: BoundedAsciiAlphabet, length: BoundedAsciiLength },
}
pub enum BoundedAsciiAlphabet { Digits, OctalDigits, UpperLetters, Letters, /* ... 19 */ }
pub enum BoundedAsciiLength { Exact(u64), Range { min_length: u64, max_length: u64 } }
const UCI_BOUNDED_ASCII_PROFILES: &[(BoundedAsciiAlphabet, &str, BoundedAsciiLength)]; // 60 rows
pub fn bounded_ascii_profiles() -> &'static [...];                                     // read-only
```

**Classification** (`match_bounded_ascii_profile`) runs only after the five
earlier profiles have declined. It has two steps:

1. Read the length shape from the facets alone:
   * `length` with neither min nor max gives `Exact`;
   * both `minLength` and `maxLength` without `length` gives `Range`;
   * anything else is not a member. In particular a `maxLength`-only
     declaration is never completed from its quantifier.
2. Look for a pinned row with that length shape whose `spelling + quantifier`
   equals the declaration's **single** expression. The existing strict
   `has_only_pattern` guard applies: one group, one alternative, the
   XML-Schema dialect, no explicit `whiteSpace`, and no numeric facet.

The whole expression is compared as text. Nothing parses a regex.

No check reads a declaration name, file, release or restriction depth. The
consequences are:

* a differently named declaration with an admitted profile classifies the same
  way;
* `AircraftIdentifierType` with `length 9` fails closed;
* an observed alphabet with an unobserved bound, such as `[a-zA-Z0-9]{1,21}`,
  fails closed;
* an unobserved spelling of an observed member set, such as `[0-9A-Z ]{8}`,
  fails closed.

**Consumers.** Every consumer already routes through `string_profile`:
coverage (`kind_renderable`, the constrained-String arm of
`primitive_ref_renderable`), Ada wrapper name preflight
(`ada_wrapper_callable_owners`), the three backends' pre-emission validation,
and Rust codec readiness. None of them needed a Task 058 recognition rule.

The backends match the new variant exhaustively, so no backend has its own
copy of the classification logic.

**Generated names.** Each carrier publishes exactly the names the existing
String carriers publish:

* Ada: the private type plus the overloaded `Create` / `Value`;
* Rust: `new` / `as_str`;
* C++: `create` / `value`.

All helpers (`Is_Member`, `Matches_Pattern`, `is_valid`, `is_member`,
`LENGTH`, `kLength`, ...) are nested or private. Task 058 therefore adds no
generated top-level name and has no new preflight obligation.

## 3. Lexical semantics

* **whiteSpace.** No member carries a `whiteSpace` facet, so `xs:string`'s
  intrinsic `preserve` applies, and an explicit facet of any value fails
  closed. The constructor never trims, collapses, case-folds or canonicalizes.
  The stored value is always the argument, unchanged.
* **SPACE.** SPACE is valid only where the alphabet contains U+0020. Leading,
  trailing and all-SPACE values are then legal whenever the length fits, and
  they stay distinct from their trimmed forms (`"abc"` != `"abc "`).
* **Both restrictions are enforced independently.** The length facets are
  checked first: an equality for `length`, or an inclusive interval for
  `minLength`/`maxLength`. The character class is then checked for every
  character.
* **Character counting.** Every member of every alphabet lies in
  U+0020..U+007E and is therefore a single UTF-8 byte. For every value that
  can be accepted, UTF-8 byte length equals the XML Schema character count.
  Any multi-byte UTF-8 character contains a byte >= 0x80, which no alphabet
  admits, so it is rejected by the class test rather than miscounted. Rust
  `str::len` and C++ `string_view::size` are therefore sound. Ada
  `String'Length` is already a character count, and every Latin-1 upper-half
  `Character` (every byte of a UTF-8 sequence) is outside every alphabet.
* **Rejected everywhere:** TAB, LF, CR, DEL, NUL, every other control, and all
  non-ASCII input. Case is significant: `a` is rejected by `[A-Z]`.
* **Rendering from numbers.** Generated validators test the alphabet's numeric
  member ranges (`BoundedAsciiAlphabet::ranges`) as ordinal comparisons. They
  are never built from the XSD spelling. As a result no class character (`\`,
  `"`, `'`, `[`, `]`) is ever written into Rust, C++ or Ada source, and no
  locale-sensitive classifier (`is_alphanumeric`, `<cctype>`, Ada character
  handling) is used.

## 4. Generated API and Task 040 lifecycle

| | Rust | C++ | Ada |
| --- | --- | --- | --- |
| type | `pub struct T { value: String }` (private field), `#[derive(Clone, Debug, PartialEq, Eq)]`, **no `Default`** | `class T`, private `std::string value_` | `type T is private;` |
| construct | `pub fn new(&str) -> Option<Self>` | `static std::optional<T> create(std::string_view)` | `function Create (Value : String) return T` (raises `Constraint_Error`) |
| observe | `pub fn as_str(&self) -> &str` | `const std::string& value() const noexcept` | `function Value (Item : T) return String` |
| lifecycle | no public field and no `Default`; struct literals, field reads and `Default::default()` fail to compile (tested) | no public default constructor; private explicit constructor; copy-only (user-declared copy suppresses the destructive move) | the component default is `raise Program_Error`, so default initialization fails independently of `-gnata` / `Assertion_Policy` (tested under both) |

**`EmptyType` is the boundary case.** Its generated carrier is ordinary:
`new("")`, `create("")` and `Create ("")` all **succeed** and store `""`,
while `" "` and `"A"` are rejected. A valid empty lexical value does not make
unchecked default construction legal:

* Ada `Bad : Blank;` still raises `Program_Error` under both assertion
  policies;
* the Rust type has no `Default`;
* the C++ type satisfies `!std::is_default_constructible_v<Blank>`.

The zero-length carrier was exercised in every storage form:

* optional (`Option` / `std::optional` / the Ada `_Optional` wrapper);
* bounded repeated `0..3`, where a fourth element is rejected and Ada's unused
  slots hold no live carrier (Task 040 spare-capacity rule);
* unbounded repeated, including Ada `Reserve_Capacity`;
* inherited members, through an abstract base;
* a Choice alternative.

Occurrence semantics are unchanged. No nillability support was added.

## 5. Rust OMS JSON codec

The existing codec rules were inspected rather than assumed. Under
OMSC-SPC-013 Rev B §6.1.4 case 5, an `xs:string`-derived simple type is a
JSON string whose characters are the XML lexical value (§6.1.5.4). The
generated codec already routes every named String declaration through its
generated constructor:

```text
encode: Value::String(value.as_str().to_owned())
decode: dec_str(value, path).and_then(|x| T::new(x).ok_or_else(|| rejected(path, "T")))
```

Codec readiness (`primitive_support`) accepts `String` by kind. The model
already fails closed on any constrained String that is not a supported
profile. As a result, every newly supported profile is codec-READY with **no
codec-generator change**:

* JSON strings only: numbers, booleans, null, arrays and objects are type
  mismatches, never coerced (a JSON `1234` is never a `CodeType`);
* decoding goes only through the checked constructor, so an invalid lexical is
  rejected at its member path and names the carrier;
* nothing is trimmed: `" AB12XY  "` (nine characters) is rejected for
  `length 8`, where trimming would have made it valid.

Covered by:

* `runtime-rust-facade-tests/tests/generated_codec_bounded_ascii.rs` (3
  tests: round trip, invalid lexicals in every position, non-string JSON);
* the mock-OWP test `task058_bounded_ascii_generated_codec_round_trips_through_mock_owp`
  (publish, typed MSG with SPACE preserved, decode-error event).

The synthetic fixture is `tests/fixtures/service-generate/codec-bounded-ascii.{xsd,yaml}`.
No Ada or C++ codec was added.

## 6. Compiler-backed conformance corpus

The shared `tests/fixtures/string/bounded-ascii.txt` corpus has 298 VALID and
527 INVALID cases, grouped by `CARRIER <Name>`. The synthetic fixture
`crates/xsd-frontend/tests/fixtures/backend-string-bounded-ascii.xsd` contains
23 representatives. Together they cover all 19 alphabets, including both
spellings of `AlphanumericUnderscoreHyphen`, and each representative carries
the exact spelling and bounds of one pinned row under a non-UCI name.

Per carrier the corpus covers:

* minimum and maximum length, one below the minimum and one above the maximum;
* both endpoints and a midpoint of every range;
* every admitted punctuation / SPACE character;
* the nearest forbidden ASCII neighbour on each side of every range;
* TAB, LF, CR, DEL and NUL, at the front and at the end;
* non-ASCII input: U+00A0, U+00E9, U+20AC, U+1F600;
* leading, trailing and all-SPACE values where SPACE is admitted;
* the lower-case twin where only upper case is admitted;
* the zero-length value for `Blank` (the `EmptyType` shape).

The `CARRIER`-aware loader (`common::bounded_ascii_cases`) reuses the existing
`load_corpus` line rules, so escapes and `<empty>` cannot drift.

The cases run through the **generated code**:

* **Rust** (`backend-rust/tests/bounded_ascii.rs`): `rustc --edition 2021 -D
  warnings`, with composition and zero-length storage, plus privacy bypasses
  (struct literal, field read, `Default`) that must fail to compile.
* **C++** (`backend-cpp/tests/bounded_ascii.rs`): `-std=c++17 -Wall -Wextra
  -Werror -pedantic-errors`. Explicit byte lengths are passed so embedded NUL
  is really tested. `static_assert`s cover the lifecycle, and moved-from
  carriers stay valid.
* **Ada** (`backend-ada/tests/bounded_ascii.rs`): the generated package
  compiles under `-gnatwa`. The only tolerated diagnostic is the pre-existing
  Task 040 bounded-sequence `Append` guard (`-gnatwc`), which main's Task 057
  duration fixture emits identically; any warning from a Task 058 body fails
  the test. The probe runs under the default policy and under `-gnata` with
  `pragma Assertion_Policy (Ignore)`.

**Planted failures.** One deliberately wrong expectation per language was
planted to prove each probe executes. Each was restored and not committed:

* Rust: `Tail8` 7 characters marked VALID, reported as `Tail8 case 1`;
* C++: `Tail8` leading-SPACE value marked INVALID, reported as `Tail8 case 3`;
* Ada: `Serial7` `"` marked INVALID, reported as `FAIL: Serial7 case 23`.

**Classifier tests** are in `codegen-core/tests/bounded_ascii_coverage.rs`:

* all 60 rows classify and are baseline in every backend;
* all 19 alphabets are canonical subsets of U+0020..U+007E;
* `EmptyType` / `AircraftIdentifierType` behave the same by facets whatever
  their name;
* 33 negative neighbours fail closed and are non-baseline;
* each of the 20 distinct pinned spellings denotes exactly its alphabet's
  numeric `ranges()`. A test-only expander that understands only the pinned
  constructs cross-checks the hand-written table against the XSD spellings;
* the five earlier profiles are unshadowed.

Two pre-existing near-miss tests had to change, because the fixed-`length`
`[ -~]{10}` shape is now a pinned row:

* the unit test still asserts it is never `VisibleAscii`, and now also that it
  *is* `BoundedAscii`;
* the coverage test's non-baseline control became the unobserved `[ -~]{11}`.

## 7. Twelve-cell coverage (before → after)

The pinned roots were run through two release binaries: `main` @ `ef29da0`
(before) and this branch (after). Each cell reports `coverage` for kinds,
fully renderable declarations, field type references, field occurrences and
message closures.

| Release | World | Backend | Kinds | Declarations | Field types | Field occurrences | Message closures |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 2.5 | closed | Ada | 5449 → **5510** /5557 | 5427 → **5488** /5557 | 13160 = /13160 | 13160 = | 395 → **526** /722 |
| 2.5 | closed | Rust | 5449 → **5510** | 5428 → **5489** | 13160 = | 13160 = | 395 → **529** |
| 2.5 | closed | C++ | 5449 → **5510** | 5428 → **5489** | 13160 = | 13160 = | 395 → **529** |
| 2.5 | open | Ada | 5449 → **5510** | 5340 → **5401** | 13160 = | 13160 = | 387 → **504** |
| 2.5 | open | Rust | 5449 → **5510** | 5340 → **5401** | 13160 = | 13160 = | 387 → **504** |
| 2.5 | open | C++ | 5449 → **5510** | 5340 → **5401** | 13160 = | 13160 = | 387 → **504** |
| 2.6 | closed | Ada | 5462 → **5523** /5570 | 5441 → **5502** /5570 | 13198 = /13198 | 13198 = | 395 → **526** /725 |
| 2.6 | closed | Rust | 5462 → **5523** | 5442 → **5503** | 13198 = | 13198 = | 395 → **529** |
| 2.6 | closed | C++ | 5462 → **5523** | 5442 → **5503** | 13198 = | 13198 = | 395 → **529** |
| 2.6 | open | Ada | 5462 → **5523** | 5354 → **5415** | 13198 = | 13198 = | 387 → **504** |
| 2.6 | open | Rust | 5462 → **5523** | 5354 → **5415** | 13198 = | 13198 = | 387 → **504** |
| 2.6 | open | C++ | 5462 → **5523** | 5354 → **5415** | 13198 = | 13198 = | 387 → **504** |

No cell went down. The deltas break down as follows:

* **Kinds and declarations, +61 in every cell.** These are exactly the 61
  bounded-ASCII declarations. Before this task their String kind was only
  baseline for a supported profile.
* **Field types and field occurrences: unchanged.** These were already 100%.
  Named-reference field renderability does not depend on the target's
  profile, and Ada's optional wrapper already handled named Strings.
* **Message closures.** Closed-schema gains **+134** for Rust and C++ and
  **+131** for Ada; open-extensions gains **+117** in every backend. On
  `main`, closed-schema message closures were equal (395 in all backends),
  although Ada already had one fewer fully renderable declaration. After this
  task, Ada is three closures below Rust/C++; the difference is newly exposed
  by removing the earlier bounded-String blockers, not a pre-existing
  three-closure difference. The Ada-only name-preflight collision on
  `QueryType_Kind` remains unchanged (see below).

The library `CoverageAnalysis` inside the Deep CI test reports the same
figures for `kinds`/`declarations`.

**Primitive kinds.** Task 058 adds no primitive kind. `members.primitive_references.String`
is 0 in both releases (there are no direct constrained Strings).

**Whole-schema first blockers.** These were re-measured for all 12 cells on
both binaries, and every cell is unchanged:

* closed-schema Ada: `QueryType companion` / `QueryType` -> `QueryType_Kind`
  collision;
* closed-schema Rust and C++: `SourceCommandEXT` has no concrete structural
  descendants;
* open-extensions, all backends: `CapabilityCommandBaseType` is not closed.

## 8. Real message impact

The Deep CI test `task058_real_uci_bounded_ascii_message_impact` covers every
real message whose projected surface, selected **or** generated support,
contains a Task 058 member. For each one it builds a single-message contract
and runs the full selected-service readiness in every backend. The verdicts
are identical across Ada, Rust and C++ for every message (the test asserts
this), and **no Task 058 member remains a blocker** (also asserted).

| | UCI 2.5 | UCI 2.6 |
| --- | ---: | ---: |
| messages reaching a member | 305 | 305 |
| **A** READY in all backends | **131** | **131** |
| **B** another declaration blocks | 138 | 138 |
| **C** topology / projection | 36 | 36 |

The 131 A-class messages are the same set in both releases.

**Confirmed with `service-check`, not with coverage alone.** The CLI was run
on the before and after release binaries for every reaching message: Rust for
all 305, and Ada, Rust and C++ for each A-class message.

* Every A-class message is NOT READY before and READY after, in every
  backend. The first blocker before the change was always a Task 058 member.
* Every B/C message is NOT READY both before and after.
* In UCI 2.5 this was 567 message × backend cells with 0 disagreements. The
  first blockers reported before the change for the 131 A-class messages
  were:

  | Blocker | Messages |
  | --- | ---: |
  | `EmptyType` | 104 |
  | `AlphanumericDashSpaceUnderscoreStringLength15Type` | 16 |
  | `LaunchPieceType` | 5 |
  | `AircraftIdentifierType` | 3 |
  | `USMTF_MissionNumberType`, `OperatorPhoneNumberType`, `ATO_PackageIdentificationType` | 1 each |

  The CLI names the first unsupported declaration in its own order. That is
  why these counts differ from the per-message first-blocker attribution
  Task 057 recorded, which was the readiness `blocked_messages` order over
  **all** messages.

**Distribution of first blockers before → after.** The baseline is Task 057's
recorded distribution. It was **re-measured**, not assumed: Task 057's own
Deep test `task057_real_uci_duration_message_impact` was re-run on `main` @
`ef29da0` against the pinned roots, and both releases reproduced exactly:
`EmptyType` 91 / 91, `AircraftIdentifierType` 65 / 65 (+1 support),
`…Length15Type` 25 / 25, `…String20Type` 5 / 5, `TimeType` 2 / 5,
`LaunchPieceType` 2 / 2, `AO_PIM_CodeType` 2 / 2, and one each for
`CounterSpaceSENO_Type`, `AlphanumericStringLength4Type`,
`AlphanumericString6Type`, `AlphanumericString54Type` and
`AlphanumericString20Type`. The A-class count was 40 / 40, as recorded.

| Task 057 first blocker (B) | count 2.5 / 2.6 | After Task 058 |
| --- | --- | --- |
| `EmptyType` | 91 / 91 | gone |
| `AircraftIdentifierType` | 65 / 65 (+1 support) | gone |
| `AlphanumericDashSpaceUnderscoreStringLength15Type` | 25 / 25 | gone |
| `AlphanumericDashSpaceUnderscoreString20Type` | 5 / 5 | gone |
| `LaunchPieceType`, `AO_PIM_CodeType` | 2 / 2 each | gone |
| `AlphanumericStringLength4Type`, `AlphanumericString6Type`, `AlphanumericString54Type`, `AlphanumericString20Type` | 1 each | gone |
| `TimeType` (`xs:time`) | 2 / 5 | still open, now 28 |
| `CounterSpaceSENO_Type` | 1 | still open (1) |

**New first-blocker distribution** among the 305 reaching messages. Every row
is identical in 2.5 and 2.6.

| Remaining first blocker | Messages | Why it is out of scope |
| --- | ---: | --- |
| `IMO_NumberType` | 39 (+1 support) | literal prefix `IMO[0-9]{7}` |
| `AO_PRF_CodeType` | 31 | optional `1?` and position-specific classes |
| `TimeType` | 28 | `xs:time` |
| `NotationType` | 11 | alternation |
| `FileNameType` | 6 | `+` and a literal `.` |
| `NIIRS_Type` | 5 | literal `.` between classes |
| `CounterSpaceCycleNumberType` | 3 | literal and optional suffix |
| `Link16_SpecificTypeModelType` | 3 | `maxLength` only (near-miss) |
| `MilitaryGridType`, `OB_O_SuffixType` | 2 each | alternation / position-specific |
| `CounterSpaceSENO_Type` | 1 | position-specific |
| support: `FIPS_CountryCodeType` 2, `Link16_TrackNumberType` 2, `Link1_TrackNumberType` 1, `IPv4_AddressType` 1 | 6 | alternation / position-specific |
| C: cyclic generated value dependencies | 32 | topology |
| C: `ProductOrFileDissemination*` unsupported abstract value | 4 | topology |

The newly exposed blockers include `xs:time` (`TimeType`, now 28 messages)
and several more complex String patterns (`IMO_NumberType`, `AO_PRF_CodeType`,
`NotationType`, ...). Both are out of scope for Task 058.

The Task 053 real test `uci_constrained_binary` pins its reaching messages.
It was updated accordingly: `IFF_Activity` / `IFF_Command` move from
`B(AircraftIdentifierType)` to **A**, and `ProductMetadata` moves from
`B(AlphanumericStringLength4Type)` to `B(FileNameType)`.

## 9. OrderOfBattle

The Task 057 baseline was verified first with the `service-check` CLI on the
before binary: in both releases and every backend, `OrderOfBattle` had 442
generated-support declarations, 403 renderable, 39 unsupported, and first
unsupported `AircraftIdentifierType`. The after binary gives:

| Release | Selected | Support total | Support renderable | Unsupported support | First unsupported support |
| --- | --- | --- | --- | --- | --- |
| 2.5 | 55/55 | 442 | 403 → **430** | 39 → **12** | `AircraftIdentifierType` → **`IMO_NumberType`** |
| 2.6 | 56/56 | 442 | 403 → **430** | 39 → **12** | `AircraftIdentifierType` → **`IMO_NumberType`** |

Identical in Ada, Rust and C++. The 12 remaining unsupported declarations are
the same in both releases and every backend, and every one is outside Task
058:

* `IMO_NumberType`;
* `Link1_TrackNumberType`, `Link16_TrackNumberType`,
  `OB_AirDefenseAreaType`, `OB_EmitterSurrogateKeyType`, `OB_O_SuffixType`,
  `UnitIdentifierType` (position-specific);
* `Link16_SpecificTypeModelType` (`maxLength`-only near-miss);
* `MilitaryGridType`, `NotationType`, `RecordOriginatorType` (alternation);
* `OctalValueType` (`+`).

`OrderOfBattle` stays **NOT READY** on generated support, as expected.
`service-generate` still stops before writing anything, and every backend's
first failure on the projected schema is `IMO_NumberType`.

The Task 056 Deep test `uci_generated_support` was updated from
`39 / AircraftIdentifierType` to `12 / IMO_NumberType`. It also asserts that
`AircraftIdentifierType`, `EmptyType` and
`AlphanumericDashSpaceUnderscoreStringLength15Type` are no longer unsupported.
The 34 Task 054 category-A messages stay READY.

## 10. Real newly-READY service

The smallest real message that became READY **because of Task 058** is UCI
2.5 **`AMTI_SettingsCommand`**:

* 61 selected declarations and no generated support (the smallest of the 131
  in total type count);
* its payload `AMTI_SettingsCommandMDT` has the optional
  `UnassignAll : EmptyType`;
* before this task it was NOT READY on `EmptyType` in every backend.

Using `tests/fixtures/service-generate/real-bounded-ascii-amti-settings.yaml`
(Deep CI test `task058_real_uci_newly_ready_service_generates_and_compiles`),
in each of Ada, Rust and C++:

* `service-check` reports **READY**, and Rust `--with-codec` reports
  **codec READY** (59/59 codec declarations);
* `service-generate` succeeds;
* the generated model and service API compile:
  * Rust, with `-D warnings`; the probe checks that `EmptyType::new("")`
    stores `""` and that `" "` is rejected;
  * C++, with `-std=c++17 -Wall -Wextra -Werror -pedantic-errors`, with the
    same probe through `EmptyType::create`;
  * Ada, with `gnatmake -gnatc`.

The real Rust codec is generated from the pinned root by the facade crate's
`build.rs`, as `real_uci_amti_settings`. It is exercised by
`runtime-rust-facade-tests/tests/real_uci_amti_settings.rs`:

* a present `UnassignAll: ""` round-trips through `EmptyType::new` and
  re-encodes byte-identically;
* an absent one stays absent;
* `" "`, `"x"`, `0`, `null` and `[]` are rejected at
  `AMTI_SettingsCommandMT.MessageData.UnassignAll`.

## 11. Byte identity and regressions

**Byte identity.** Every `.xsd` fixture in the repository (182 files,
including the two new ones) was generated with the before and after binaries
in 3 languages × 2 worlds, giving 1092 cells:

* **533 cells byte-identical**;
* **0 cells changed**;
* **0 regressions**;
* 547 cells fail in both binaries. These are the intentional fail-closed
  negative fixtures, for example the `CollapsingText` control in
  `service-generate/visible-ascii.xsd` and the `xs:time` fixtures;
* 12 cells went from failure to success. They are exactly the two new Task
  058 fixtures, `backend-string-bounded-ascii.xsd` and
  `codec-bounded-ascii.xsd`.

The identical cells include every earlier profile fixture:

* Task 037 schema-version and Task 038 UUID;
* Task 039 visible-ASCII (`backend-string-visible-ascii.xsd`);
* Task 041 whitespace-visible and Task 042 NATO;
* Task 044 `enum-remapping`, Task 046 direct DateTime, Task 053
  `constrained-binary`, Task 054 `member-keywords`;
* Task 057 `backend-duration` / `codec-duration`;
* `position-report`.

No pre-existing successful cell changed. That is expected, because none of
those fixtures contains a pinned bounded-ASCII row.

**Suites.** All Task 037/038/039/041/042/044/046/053/054/056/057 test suites
pass unchanged under `AMS_GRA_REQUIRE_GNAT=1 cargo test --workspace`, except
the two edits already listed:

* the `[ -~]{10}` near-miss tests (§6);
* the pinned Task 053/056 real-UCI expectations (§8, §9).

**Whole-schema first blockers** were re-measured, not assumed; see §7. All 12
cells are unchanged.

## 12. CI placement

**Fast CI** (synthetic only) runs the following through `cargo test
--workspace`:

* the classifier and coverage tests (`codegen-core` unit tests and
  `bounded_ascii_coverage.rs`);
* the Rust, C++ and Ada corpus probes;
* the lifecycle tests;
* the codec and mock-OWP tests.

It also has explicit `require_one_test` gates for:

* the GNAT corpus probe
  `generated_bounded_ascii_validators_match_the_shared_corpus_under_both_policies`;
* the three codec tests, `task058_bounded_ascii_round_trips_its_stored_text`,
  `task058_invalid_bounded_ascii_lexicals_are_rejected_by_the_model` and
  `task058_non_string_json_is_rejected`;
* the mock-OWP test
  `task058_bounded_ascii_generated_codec_round_trips_through_mock_owp`.

**Deep CI `real-uci`** runs
`cargo test --release -p ams-gra-codegen-oms --test uci_bounded_ascii_string`
(4 tests) and checks these whole-line markers:

* `UCI 2.{5,6} BOUNDED ASCII INVENTORY: PASSED`;
* `UCI 2.{5,6} BOUNDED ASCII MESSAGE IMPACT: RECORDED`;
* the libtest-prefix-tolerant `UCI 2.5 REAL NEWLY-READY BOUNDED ASCII SERVICE:
  PASSED`;
* `test result: ok. 4 passed`.

The same job still runs the updated Task 053 (`uci_constrained_binary`) and
Task 056 (`uci_generated_support`) tests.

**Deep CI `real-sleet`** adds
`real::task058_real_amti_settings_empty_type_round_trips` with the marker
`REAL AMTI_SETTINGSCOMMAND EMPTYTYPE CODEC: PASSED`.

**Guard scripts.**

* `scripts/check-ci-split.sh` forbids `--test uci_bounded_ascii_string` in
  Fast CI, and requires the target and every marker in Deep CI.
* `scripts/test-check-ci-split.sh` adds adversarial cases: a Fast-CI leak,
  each dropped marker, a glued prefix, and a foreign prefix. It now runs
  **88 checks**, up from 67.

All marker checks use here-strings, never `printf | grep -q`.

This PR edits `.github/workflows/**`, so Deep CI also runs on the PR (Task 055
trigger paths).

## 13. Local validation record

* Repository gates: `cargo fmt --all -- --check`, `cargo check --workspace
  --all-targets`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `AMS_GRA_REQUIRE_GNAT=1 cargo test --workspace`, `git diff --check`,
  `scripts/check-ci-split.sh` and `scripts/test-check-ci-split.sh`: all passed.
  The workspace run had **1191 passed, 0 failed** across 115 test binaries;
  the unchanged `main` baseline had **1170 passed, 0 failed** across 108.
* Rust 1.95.0 locked checks passed for `ams-gra-oms-runtime-api`,
  `ams-gra-oms-runtime-rust --all-targets` and
  `ams-gra-oms-runtime-rust-facade-tests --all-targets`.
* The pinned class-spelling versus numeric-alphabet cross-check, added after
  the workspace run, passed as a separate targeted test (1 passed, 0 failed).
* The real `AMTI_SettingsCommand` codec test passed (1 passed, 0 failed).
  Local Deep CI real-UCI tests passed with their workflow marker checks:
  Task 058 (4 tests), Task 056 (2), Task 053 (3), Task 054 (4) and Task 057
  (4); the five steps all passed without rerunning them for this record.
* The before/after fixture comparison (§11) had 533 byte-identical cells,
  547 failures in both binaries, and exactly 12 new successes. No previously
  successful output changed and no cell regressed.
