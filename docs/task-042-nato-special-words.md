# Task 042 — Validated NATO special-words String profile

Adds a generated, validated carrier for the constrained-String profile carried by
`NATO_SpecialWordsType`, in Ada, Rust, and C++, for **both** pinned releases.
This is lexical validation only: a value being accepted says nothing about a
security marking's operational meaning, authorization, or release policy.

## 1. Authoritative evidence

Freshly acquired for this task: the authoritative public UCI Standard repository
(`https://gitlab.com/open-arsenal/uci/standard.git`) was cloned and both pinned
commits were checked out as detached worktrees. UCI 2.6 was extracted from its
authoritative ZIP. Neither tree was modified.

| Release | Revision | Schema source |
| --- | --- | --- |
| UCI 2.5 | `093610b7753944059360d3236770ab446d039556` | `03_OAC-STD-002_RevE_UCI_Schema_v2_5/` |
| UCI 2.6 | `78eb61b6112c8bffa40820c33124b57787fc5bd9` | `03_UCI-STD-002_Rev6_UCI_Schema_v2_6-CDRL.zip` |

SHA-256 of the security-markings documents: 2.5
`4a8056f1503234d423a0c2495844ab65387febced62bea58b87d4f343e9e7a0b`, 2.6
`ee6e58d5db9fd80d526bc964b8244ec296d106301640d34c3cb2c5c00b710b88` (identical to
the Task 041 extraction).

### 1.1 The declaration, raw XSD

| | UCI 2.5 | UCI 2.6 |
| --- | --- | --- |
| QName | `{https://www.vdl.afrl.af.mil/programs/oam}NATO_SpecialWordsType` | same |
| File:line | `UCI_SecurityMarkings_v2_5_0.xsd:4903` | `UCI_SecurityMarkings_v2_6_0.xsd:4916` |
| Reached via | the root's `xs:include` | same |
| Immediate base / ultimate primitive | `xs:string` / `xs:string` | same |
| Restriction chain | depth 1 (direct restriction) | same |
| `length` | absent | absent |
| `minLength` / `maxLength` | **6** / **261** | same |
| explicit `whiteSpace` | absent | absent |
| effective `whiteSpace` | `preserve` (intrinsic to `xs:string`) | same |
| pattern groups / alternatives | 1 / 1 | same |
| dialect | XML Schema | same |
| expression | `NATO:[a-zA-Z\-_]{1,256}` | same |
| other facets | none (no numeric, no enumeration) | none |

The two fragments are **byte-identical after end-of-line normalization** (2.5 is
CRLF, 2.6 LF). The releases do not differ, so one shape is implemented.

```xml
<xs:simpleType name="NATO_SpecialWordsType" uci:version="000.001.000.000">
  <xs:annotation>
    <xs:documentation>North Atlantic Treaty Organization Special Words</xs:documentation>
    <xs:documentation>Total number of distinct values is 54.</xs:documentation>
  </xs:annotation>
  <xs:restriction base="xs:string">
    <xs:minLength value="6"/>
    <xs:maxLength value="261"/>
    <xs:pattern value="NATO:[a-zA-Z\-_]{1,256}">
      <xs:annotation>
        <xs:documentation>North Atlantic Treaty Organization Special Words</xs:documentation>
      </xs:annotation>
    </xs:pattern>
  </xs:restriction>
</xs:simpleType>
```

The length facets were read **separately** from the quantifier. They agree only
once the fixed five-character prefix is counted (6 = 5 + 1, 261 = 5 + 256); that
agreement is an observation, not used to infer either quantity. The type-level
"Total number of distinct values is 54" text is documentation, not a facet (no
enumeration exists), and is not enforced.

### 1.2 The pattern's leading annotation

In both releases the pattern facet has exactly one element child: a leading,
attribute-free `xs:annotation` holding one plain-text `xs:documentation`. The
frontend's existing handling (Tasks 008/014) validates that shape and discards
the text. Nothing was added to the semantic IR, and backend support does not
depend on it. The new fixtures carry the same annotation, so generation through
it is exercised.

### 1.3 Normalized frontend output

`load_schema_set` over both pinned roots produced, for both releases:

```text
kind = Primitive(String), base = Primitive(String)
min_length = 6, max_length = 261, length = None, numeric facets = None
white_space = None
pattern_groups = [[XmlSchema "NATO:[a-zA-Z\\-_]{1,256}"]]   (23 characters;
                  BACKSLASH HYPHEN held as two characters)
classifier at a2c9eea = Err(UnsupportedConstraints)
```

### 1.4 Inventory of the complete reachable schema sets

Both releases (root + included security markings; 5557 / 5570 types) were
inventoried over raw XSD restrictions **and** over normalized IR constraint sets.

| Question | UCI 2.5 | UCI 2.6 |
| --- | --- | --- |
| declarations whose effective constraints equal the profile | **1** (`NATO_SpecialWordsType`) | **1** (same) |
| named restrictions deriving from `NATO_SpecialWordsType` | 0 | 0 |
| other patterns with a literal `NAME:` prefix | 0 | 0 |
| patterns mentioning `NATO` | 1 (this one) | 1 |
| patterns containing `:` at all | 2 (this, `IPv6_AddressType`) | same |
| use sites | 4 required choice alternatives | same |

The use sites are the `NATO_SpecialWord` alternative of
`FGI_SourceOpenChoiceType`, `FGI_SourceProtectedChoiceType`,
`OwnerProducerChoiceType`, and `ReleasableToChoiceType`. Each choice's other
alternative is an enumeration.

Nearby profiles that differ, and stay unsupported: the unprefixed
`[a-zA-Z0-9_\-]{1,N}` family (`OB_FacilityNameType`,
`UCI_SchemaComponentNameType`, `EOB_CED_NameType`, …: digits, no prefix);
`[a-zA-Z0-9 \-_]{…}` (space and digits); `IMO[0-9]{7}` (another fixed prefix,
digits); `IPv6_AddressType`.

## 2. Shared classification

`crates/codegen-core/src/string_profile.rs` gains one **fixed** variant,
`StringProfile::NatoSpecialWords`, and exported constants:

| Constant | Value | Meaning |
| --- | --- | --- |
| `NATO_SPECIAL_WORDS_PATTERN` | `NATO:[a-zA-Z\-_]{1,256}` | exact expression |
| `NATO_SPECIAL_WORDS_PREFIX` | `NATO:` | exact prefix |
| `NATO_SPECIAL_WORDS_MIN_LENGTH` / `_MAX_LENGTH` | 6 / 261 | **total** length |
| `NATO_SPECIAL_WORDS_SUFFIX_MIN_LENGTH` / `_MAX_LENGTH` | 1 / 256 | **suffix** length |

A fixed variant, not a parameterized prefix or regex facility: the inventory
shows one exact shape and no other prefixed profile, so there is nothing to
parameterize, and a generic matcher would admit unproven shapes.

The matcher reads no name, file, release, or chain depth. It requires no
`length`, `minLength == 6`, `maxLength == 261`, and then the **unchanged** strict
`has_only_pattern` guard: no explicit `whiteSpace`, no numeric facet, exactly one
group holding exactly one XML-Schema expression equal to the constant. It runs
after the four older matchers, whose code and guards are untouched.

A zero-facet named restriction inherits the profile through the frontend's
existing effective-constraint resolution (`DerivedSpecialWord`,
`DerivedReleaseToken`); no backend walks a chain.

Consumers: the three backend validation gates and renderers name the variant
explicitly. Coverage, readiness, `schema_emits_string_profile_carrier` (Ada body
emission), and Ada callable/name analysis already read
`string_profile(..).is_some()` and needed no change. No backend classifies on its
own.

### 2.1 The existing NATO-pattern negatives

`string_profile.rs` and `tests/string_profile_coverage.rs` each carried a
negative built from `visible_ascii(1, 256)` plus the NATO pattern — **total bounds
1..256**, not the authoritative 6..261. Both are **kept rejected**; only their
comments and labels changed, to "NATO pattern with non-authoritative
total-length facets". A dedicated unit test re-asserts that exact shape, and the
classifier also rejects `visible_ascii(6, 261)`.

## 3. Generated value semantics

1. total length within **6..261** — checked first;
2. the first five characters are exactly **`NATO:`** — case-sensitive;
3. every remaining character (1..256 of them) is `A`–`Z`, `a`–`z`, `-`, or `_`.

- Explicit ASCII comparisons only. No `\w`, `[A-z]`, Unicode or locale
  predicates, regex substring search, or regex dependency; generated-text tests
  forbid them.
- No trimming, case folding, or whitespace normalization. SPACE, TAB, LF, and CR
  are outside the class and rejected. Task 041's collapse is **not** imported.
- The accepted text is stored exactly as supplied, prefix included.
- Digits are rejected. `-` and `_` are accepted anywhere, including as the whole
  suffix.
- The schema's `\-` is an XSD regex escape for HYPHEN. Constructors take decoded
  strings, so a BACKSLASH in the input is an ordinary non-member.
- Length precedes any prefix access, so short input is rejected through the
  checked API — Ada `Constraint_Error`, Rust `None`, C++ `std::nullopt` — never a
  slice panic or an incidental `out_of_range`.
- Constant auxiliary space; no input-sized allocation. Owned storage is allocated
  only for an accepted value.

Per backend:

- **Ada.** Helpers are local to `Create`. Indices are `Text'First + Offset` and
  `Text'First + Prefix'Length .. Text'Last`, so non-1-based, null, and
  `Positive'Last`-ending slices are safe; all three are tested.
- **Rust.** Bytes only: `as_bytes().starts_with(b"NATO:")`, then a byte slice of
  a value already known to be at least six bytes. Non-ASCII is rejected without
  panicking.
- **C++.** Explicit byte loops through `static_cast<unsigned char>`; no `substr`
  or positional `compare`. Control-byte tests use explicit-length strings.

API conventions are unchanged (Ada `Create`/`Value`, Rust `new`/`as_str`, C++
`create`/`value()`). No comparison or ordering API was added. Existing equality
(Rust derived, Ada predefined) compares the stored text, which is `xs:string`
value equality, so suffix case is significant.

## 4. Lifecycle and composition

Reused unchanged from Task 040:

- **Ada** — private record with the raising component default. A default
  declaration raises `Program_Error` with and without
  `Assertion_Policy (Ignore)`; an absent optional wrapper constructs; an
  unchecked `Is_Present => True` payload raises.
- **C++** — explicit copy constructor and assignment, no move operations, copies
  not `noexcept`; source and destination verified after rvalue construction and
  assignment.
- **Rust** — private storage, checked construction, no `Default`.

Required, optional, bounded zero-minimum, bounded positive-minimum (no `Clear`),
and unbounded occurrences use only the existing containers.

## 5. Tests

| Area | File |
| --- | --- |
| classifier, 8 unit tests incl. the relabelled negative | `crates/codegen-core/src/string_profile.rs` |
| Ada name collisions, overloads, unsupported reserves nothing (3) | `crates/codegen-core/src/backend_names.rs` |
| coverage (4) | `crates/codegen-core/tests/nato_special_words_coverage.rs` |
| shared corpus, `<empty>` encoding, unchanged loader | `tests/fixtures/string/nato-special-words.txt` |
| backend fixture | `crates/xsd-frontend/tests/fixtures/backend-string-nato-special-words.xsd` |
| Rust generated code, compiled and run (3) | `crates/backend-rust/tests/nato_special_words.rs` |
| C++17 generated code, compiled and run (2) | `crates/backend-cpp/tests/nato_special_words.rs` |
| Ada generated code under GNAT (4) | `crates/backend-ada/tests/nato_special_words.rs` |
| selected service (5) | `crates/cli/tests/service_generate.rs`, `tests/fixtures/service-generate/nato-special-words.{xsd,yaml}` |

**Corpus.** Synthetic positives `NATO:A`, `NATO:a`, `NATO:-`, `NATO:_`,
`NATO:aBc-X_` and others — grammar membership only, not recognized markings.
Negatives: empty input and every truncated prefix; bare `NATO:`; wrong-case and
misspelled prefixes; missing, doubled, and misplaced colons; leading text;
leading, trailing, and interior whitespace; trailing LF, CR, CRLF; digits;
backslash, slash, brackets, caret, backtick, and other punctuation; NUL and
controls; Latin-1 letters, NBSP, Unicode lookalikes (Cyrillic, Greek, fullwidth),
CJK, and a four-byte emoji. Every valid case is reconstructed from its stored
value.

**Generated boundaries.** Suffix 0/1/255/256/257 = total 5/6/260/261/262, and
10 000-character invalid inputs.

**Alphabet sweeps.** All 256 single-byte suffix candidates in Ada and C++; all
128 ASCII candidates plus nine representative non-ASCII characters in Rust.
Expected answers come from an alphabet stated **in each test**, never from the
production classifier or validator, and each test asserts that alphabet has
exactly 54 members.

**Compile-fail controls.** Rust struct literal, field read, and private
validator call; Ada aggregate and component read. Each runs beside a positive
control built by the same harness and must fail with the intended diagnostic
(Ada additionally rejects "not found"/syntax-style failures).

## 6. Selected-service integration

With the same fixture, the reviewed main `a2c9eea` reported all three backends
`NOT READY` with `blocker: {urn:test}ReleaseToken`; this branch reports all three
`READY`. The unselected neighbour `LooseReleaseToken` — the NATO expression under
1..256 — is not projected. Selecting its own message yields `NOT READY` naming
it, generation fails, and the output directory stays empty. Generated clients
construct and read the values in every occurrence shape, and the Ada selection
emits the `.adb` body. `service_plan.rs`, `service_readiness.rs`, and
`service_generation.rs` are unchanged.

## 7. Measured results

Before is a clean worktree at `a2c9eea931a45c657e4156bebf878940c34de9c3`; after
is this branch. Identical pinned inputs and identical commands.

### 7.1 Workspace tests

`AMS_GRA_REQUIRE_GNAT=1 cargo test --workspace`, summing `test result` passes:

| | Passed | Failed | Ignored |
| --- | ---: | ---: | ---: |
| Baseline (`a2c9eea`), measured | 832 | 0 | 0 |
| Task 042 | **859** | 0 | 0 |

+27, all new. No existing test was deleted, skipped, or weakened.

### 7.2 Coverage — declarations fully renderable

| Cell | Before (Ada / Rust / C++) | After | Delta |
| --- | --- | --- | --- |
| 2.5 closed | 5317 / 5394 / 5397 | 5318 / 5395 / 5398 | **+1 / +1 / +1** |
| 2.5 open | 5232 / 5306 / 5309 | 5233 / 5307 / 5310 | **+1 / +1 / +1** |
| 2.6 closed | 5338 / 5416 / 5420 | 5339 / 5417 / 5421 | **+1 / +1 / +1** |
| 2.6 open | 5253 / 5328 / 5332 | 5254 / 5329 / 5333 | **+1 / +1 / +1** |

All twelve baseline cells reproduced the recorded starting values exactly.
`kinds` rises by 1 in every cell (2.5: 5447 → 5448; 2.6: 5460 → 5461).
Field-types, field-occurrences, and message-closures are unchanged.

### 7.3 Explaining the movement

Per release the one newly supported declaration is `NATO_SpecialWordsType`, a
**direct** match. There is no equivalent or inherited declaration (§1.4), so no
alias or inheritance gain exists.

There is also no transitive gain **in the declaration count**, measured rather
than assumed. A probe computed renderability over each consumer's own
dependency closure, before and after:

| Closure (both releases) | Before Ada / Rust / C++ | After |
| --- | --- | --- |
| `NATO_SpecialWordsType` (1) | 0 / 0 / 0 | 1 / 1 / 1 |
| each of the four `*ChoiceType` consumers (3) | 1 / 2 / 2 | 2 / 3 / 3 |
| `SecurityInformationType`, 2.5 (23) | 16 / 20 / 20 | 17 / 21 / 21 |
| `SecurityInformationType`, 2.6 (24) | 18 / 22 / 22 | 19 / 23 / 23 |

The choice types were **already** counted renderable before this task: the
declaration metric is per declaration, and a named reference to another
declaration does not make its holder unrenderable. So they add nothing to the
count. What changed is transitive completeness: in Rust and C++ the four choice
types' **whole closures** are now renderable; in Ada they stay blocked by their
enumeration alternatives. `SecurityInformationType` gains one renderable
dependency and stays blocked, by `DeclassExceptionEnum` in every backend and by
further enumerations in Ada.

### 7.4 Selected `PositionReport` (UCI 2.5, closed world)

| Backend | Baseline | Task 042 | First blocker |
| --- | ---: | ---: | --- |
| Ada | 53/60 | **54/60** | `SecurityInformationType` (unchanged) |
| Rust | 57/60 | **58/60** | `SecurityInformationType` (unchanged) |
| C++ | 57/60 | **58/60** | `SecurityInformationType` (unchanged) |

Still **NOT READY** everywhere; the only change in each report is
`NATO_SpecialWordsType` leaving the unsupported list. Remaining unsupported
selected types:

- Ada: `SecurityInformationType`, `DeclassExceptionEnum`, `FGI_SourceOpenEnum`,
  `FGI_SourceProtectedEnum`, `OwnerProducerEnum`, `ReleasableToEnum` — boundary
  "Ada cannot form a legal identifier from `25X1`";
- Rust and C++: `SecurityInformationType`, `DeclassExceptionEnum` — the same
  boundary.

All remaining blockers are enumeration identifier remapping, out of scope.

### 7.5 Full-schema first failures and determinism

Full-schema `generate` first failures are **unchanged** reserved-word
boundaries: 2.5 Ada `AltitudeRangePairType / Range`, Rust
`ConfigurationParameterType / Type`, C++ `ApprovalResponseType / Operator`; 2.6
Ada and Rust the same, C++ `COMINT_ChangeDwellType / Delete`.

Two consecutive UCI 2.5 closed-world coverage runs are byte-identical and match
the measurement run. Two consecutive selected-service generations are
byte-identical in all three languages.

## 8. Compilers

| Toolchain | Local | Observed in CI (run `36084380457`, main `a2c9eea`) |
| --- | --- | --- |
| OS | Debian 13 (trixie) | `ubuntu-24.04`, image `20260920.314.1` |
| rustc | `1.98.1 (48a229cea 2026-09-01)` | `1.98.1 (48a229cea 2026-09-01)` |
| GNAT | `GNATMAKE 14.2.0` | `GNATMAKE 13.3.0` (apt `gnat-13 13.3.0-6ubuntu2~24.04.1`) |
| C++ | `g++ (Debian 14.2.0-19) 14.2.0` | the image's default `c++`; the workflow does not print its version |

The workflow's `ubuntu-latest`, `dtolnay/rust-toolchain@stable`, and
apt-installed `gnat` are **floating selections, not exact version pins**. The CI
column records what was observed, not what is guaranteed. The Task 041 document's
sentence calling CI versions "pinned by the workflow" is corrected in place; no
toolchain-policy change is made.

Because CI's GNAT differs from the local one, the Ada privacy tests match the
stable part of GNAT's diagnostic and reject unrelated failures instead of
pinning one release's exact message. The final-head CI result is reported in the
PR rather than here.

## 9. Files changed

| Area | Files |
| --- | --- |
| classifier | `crates/codegen-core/src/string_profile.rs`, `crates/codegen-core/src/lib.rs` |
| name-model tests | `crates/codegen-core/src/backend_names.rs` (tests only) |
| backends | `crates/backend-{ada,rust,cpp}/src/lib.rs` |
| corpus paths | `crates/backend-{ada,rust,cpp}/tests/common/mod.rs`, `crates/backend-rust/tests/corpus_loader.rs` |
| new tests | `crates/backend-{ada,rust,cpp}/tests/nato_special_words.rs`, `crates/codegen-core/tests/nato_special_words_coverage.rs` |
| amended tests | `crates/codegen-core/tests/string_profile_coverage.rs` (label only), `crates/cli/tests/service_generate.rs` (additions) |
| fixtures | `tests/fixtures/string/nato-special-words.txt`, `crates/xsd-frontend/tests/fixtures/backend-string-nato-special-words.xsd`, `tests/fixtures/service-generate/nato-special-words.{xsd,yaml}` |
| CI | `.github/workflows/ci.yml` — three more named, must-execute GNAT probes |
| docs | this file, `docs/roadmap.md`, `docs/backend-compatibility.md`, `docs/task-041-whitespace-visible-string.md` (one-sentence correction) |

**API and dependency implications.** Additive only: one new carrier type per
supported declaration, and six new public constants from `codegen-core`. No
existing generated spelling or behaviour changed. No new dependency in the
generator or in generated code — no regex crate, `GNAT.Regpat`, `<regex>`, or
`<cctype>`.

## 10. Out of scope

Enum identifier remapping; any special case for `SecurityInformationType` or
`PositionReport`; generic prefix-string support or a regex translator; new
whitespace normalization; codecs, serialization, transport, CAL, and runtime
integration; and any redesign of repeated-value storage.
