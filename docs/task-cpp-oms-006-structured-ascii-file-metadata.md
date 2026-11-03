# CPP-OMS-006 — C++17 structured-ASCII OMS JSON codecs and the authentic UCI FileMetadata

Stacked on CPP-OMS-005 (PR #178, head
`55469a207a9cca36e0aea9f59cdfe07a5851a4bc`). The C++ model generator, the
generated service API, the LA-CAL runtime and Sleet are unchanged. No
FileMetadata-specific codec logic exists anywhere.

## Result

| Release | Pin | `FileMetadata` C++ codec BEFORE (`55469a2`) | AFTER |
| --- | --- | --- | --- |
| UCI 2.5 | `093610b7…`, root `ac943049…bf27` | closure 52, model READY, codec 48/50 NOT READY at `FileNameType` (structured ASCII) | codec **49/50 NOT READY** at `SHA_2_256_HashType` (length-constrained hexBinary) |
| UCI 2.6 | `78eb61b6…`, root `af54ce72…955b` | closure 54, model READY, codec 50/52 NOT READY at `FileNameType` | codec **51/52 NOT READY** at `SHA_2_256_HashType` |

Structured ASCII was the first codec blocker of FileMetadata in both releases,
and it is no longer a blocker. It was not the only one, though. The optional
`FileMetadataMDT.SHA_2_Hash` is a `SHA_2_256_HashType` (`xs:hexBinary`,
`length 32`). The C++ codec has supported only unconstrained hexBinary since
CPP-OMS-002, so `FileMetadata` stays codec NOT READY in **both** releases, and
`service-generate --with-codec` writes nothing for it. The declaration counts
(49/50 and 51/52) show it is the only remaining blocker.

Consequently, and as the task required, **Stages D and E did not run for any
release**: no FileMetadata codec round trip and no FileMetadata pinned-Sleet
interoperability is claimed. What was checked for the authentic FileMetadata is
listed under [Authentic FileMetadata](#authentic-filemetadata-stage-d). Making
it codec READY needs a constrained-hexBinary C++ codec, a separate value kind
outside this task (see [Limitations](#limitations-not-claimed)).

What CPP-OMS-006 does deliver:

* the generated C++ codec now supports **every** pinned Task 059
  structured-ASCII profile (27 expression/facet rows per release);
* six authentic messages per release become C++ codec READY
  (`AO_Capability`, `CounterSpaceReport`, `DMPI_DesignationRequest`,
  `OrdersMetadata`, `StrikeCapability`, `StrikeCommand`). They get a compile
  smoke only;
* nothing that was READY is lost (whole-release sweep, both pins).

## Readiness before the change (Stage A)

Task 059 already lowered `StringProfile::StructuredAscii` into a checked C++
model carrier (`static std::optional<T> create(std::string_view)`,
`const std::string& value()`, no default constructor, copy only). Validation is
a left-to-right segment matcher with no regex engine, including the
length-guided optional prefix of `1?[1-7][1-8]{2}`. C++ **model** readiness
was READY. The CPP-OMS-005 C++ **codec** analyzer refused the profile in
`cpp_codec_admits_string_profile` (`named String profile structured ASCII has
no C++ codec mapping`). The renderer already maps every admitted named String
through the model's `create` / `value()`, so the admission predicate was the
only blocker. Measured with a CLI built at `55469a2`:

| Message | Release | Closure | Codec BEFORE | First blocker |
| --- | --- | ---: | --- | --- |
| `FileMetadata` | 2.5 / 2.6 | 52 / 54 | 48/50 / 50/52 | `FileNameType` (structured) |
| `AMTI_Command` | 2.5 / 2.6 | 221 / 222 | 212/217 / 213/218 | `SpoilFactorType` (Float32) — structured ASCII is not its first blocker |
| `AO_CapabilityStatus` | 2.5 / 2.6 | 64 (+ generated support) | 103/130 | `AO_PRF_CodeType` (structured) |

Every global message was also swept; see [Compatibility
impact](#compatibility-impact-stage-f). Structured ASCII was the first codec
blocker of 86 model-READY messages in each release.

## Implementation (Stage B)

* `codegen-core::cpp_codec_admits_string_profile` (the single shared
  predicate) now admits `StringProfile::StructuredAscii(row)` **only when
  `row` is one of the pinned rows of `structured_ascii_profiles()`**. A
  `StructuredAscii` value built by hand rather than by the shared classifier
  (wrong segments or wrong facets) is refused, so the codec cannot be widened
  past what the classifier recognizes. Alternating ASCII, IPv6 and Unicode 3.1
  stay explicitly NOT READY.
* The C++ renderer already re-asked the same predicate before emitting a
  String carrier codec (CPP-OMS-005), so `service-check` and
  `service-generate` cannot disagree. Only its comments change.
* **Decode**: the member must be a JSON string; its unescaped UTF-8 text goes
  unchanged to `T::create`; a refusal is `value rejected by the generated
  model's <Type> check` at the member's schema path. **Encode**: `value()`
  unchanged, with only the JSON-mandatory escapes. Both use the existing
  `get_checked_text` / `put_string` and path diagnostics. No new parser, no
  regex, no dependency, no restated class, literal, length or segment rule:
  `structured_ascii_codec_is_ready_delegating_and_separate` asserts the
  generated codec contains none of the model's matcher text.
* The NOT READY diagnostic's "supported" list now names structured ASCII.
  That is the only change to any report text.

The unit test `cpp_oms_006_structured_ascii_admission_boundary` checks all 27
rows. Each must classify as exactly that row and be codec READY as a required,
optional, bounded 0..3, unbounded 1..∞ member and as a Choice alternative. It
also checks nine unpinned neighbours, which neither classify nor get admitted:
FileName with `maxLength 256` or `maxLength`-only, IMO with length 11, an
`IMX` prefix, PRF with range 3..5 or `maxLength`-only, `2?[1-7][1-8]{2}`, a
duplicated alternative, and an explicit `whiteSpace`. It refuses two forged
`StructuredAscii` values, and confirms that alternating ASCII, IPv6 and
Unicode 3.1 still name their own boundary. One CPP-OMS-005 near miss,
`[A-Za-z0-9]{1,4}` with `maxLength` only, was a bounded-ASCII non-match. It is
exactly the pinned structured row `Link16_SpecificTypeModelType`, so that
test now asserts it is READY as structured ASCII.

## Synthetic qualification (Stage C)

Two synthetic (non-UCI) fixtures, both generated by the production CLI:

1. **The shared Task 059 fixture**
   `tests/fixtures/service-generate/codec-structured-ascii.{xsd,yaml}`, reused
   unchanged: required `ImoType`, required length-guided `PrfType`, optional
   `FileNameType`, bounded 0..3 `ImoType`, unbounded `OctalType`, and a Choice
   of `ModelType` / `ImoType`. Codec 7/7 READY.
   `tests/cpp-codec/structured_ascii_consumer.cpp` covers:
   * FileName: ordinary names, multiple extensions (`report.tar.bz2`,
     `archive.tar.gz.part1`), dots after the boundary (`x..`), empty stem or
     extension (`report.`, `.txt`, `.`), no dot, class violations in each
     segment, leading/trailing SPACE, TAB/LF/CR/NUL/DEL/NBSP via escapes,
     2-, 3- and 4-byte UTF-8 and escaped non-ASCII, escaped class members
     (`\u0061.\u0062`, `a\u002eb`) re-encoded canonically, the escaped solidus,
     and 255 accepted / 256 rejected;
   * IMO: exact prefix (case, `IM0`, `IMX`, reversed), seven digits, wrong
     lengths, full-width digits, NUL;
   * PRF: `111`…`788` without the prefix, `1111`…`1788` with it, `[1-7]` and
     `[1-8]` class violations, `2778` and `7111` (at length four the prefix
     must be the literal `1`, with no other partition tried), lengths 1, 2
     and 5, Arabic-Indic digits, JSON numbers `178` and `1.78e2`;
   * Octal: 1..16 characters in an unbounded array of 40, invalid items at
     `Octals[1]`;
   * the Choice: each alternative, a value valid only for the other
     alternative, empty / two selected / unknown / wrong-case alternatives;
   * wrong JSON kinds (number, null, array, object, boolean) for every member,
     duplicate keys (also after key unescaping) at `$`, unknown and wrong-case
     members, the global-element envelope, `$type`, raw control characters
     and ill-formed UTF-8 at `$`, whitespace and member reordering;
   * encoding of model-built values compared byte for byte, and the model's
     `create` checked directly.

   Every rejection asserts its exact path and exact message, for example
   `StructuredPayload.Imos[2]` or `StructuredPayload.Pick.Model`. Result:
   `CPP_OMS_006_STRUCTURED_ASCII checks=323 rejections=166`.

2. **All 27 pinned rows**, `tests/fixtures/cpp-codec/structured-ascii/`
   (new): one declaration per row under non-UCI names, 26 optional members
   plus a required, an unbounded and a 1..2 `FileNameType`/`SortieType`
   member and a Choice. Codec 29/29 READY.
   `tests/cpp-codec/structured_ascii_rows_consumer.cpp` takes the valid and
   invalid samples for each row, written by hand from the XSD pattern and
   facets. Valid samples must decode to exactly that text and re-encode to
   exactly the input. Invalid samples must be refused at
   `StructuredRowsPayload.<Member>` with the model-check message. For every
   sample, `T::create` must agree with the codec. Wrong kinds and occurrence
   bounds are covered too, and all 27 rows are encoded at once from model
   values. Result: `CPP_OMS_006_STRUCTURED_ROWS rows=27 checks=828
   rejections=259`.

Both consumers use only the generated public API and hand-written JSON, so no
expected output comes from the encoder. Their behavioral reference is the
Rust `codec-structured-ascii` mock-OWP test.
`task059_shared_structured_ascii_fixture_round_trips_in_cpp` round-trips that
test's filename shapes in C++.

**Negative controls.** `tests/cpp-codec/structured_ascii_mutations.py` builds
21 weakened copies of the generated headers. Each weakens one thing, and the
consumer must fail on every one.
* Model: the FileName stem accepts `.`; empty extension; empty stem; no dot
  literal; maxLength off by one; non-ASCII accepted; TAB accepted; IMO prefix
  ignored; six IMO digits; PRF prefix consumed greedily; PRF prefix never
  consumed; any PRF prefix digit; PRF `[1-7]` widened; octal accepts `8`;
  model code accepts `""`.
* Codec: trims before `create`; upper-cases before `create`; number accepted
  as text; an invalid optional silently dropped; wrong error path; optional
  omitted on encode.

Result: `CPP_OMS_006_MUTATIONS detected=21/21`.

Both consumers ran under strict g++, strict clang++, g++ and clang++
ASan+UBSan, and g++ and clang++ TSan, with identical summaries in every
configuration.

## Authentic FileMetadata (Stage D)

Behavioral reference: `crates/runtime-rust-facade-tests/tests/real_uci_file_metadata.rs`.
For each release separately (2.5 and 2.6 are measured and generated from their
own pinned roots):

1. `service-check --with-codec` reports model READY and codec NOT READY. The
   counts are 49/50 (2.5) and 51/52 (2.6), and the single boundary line is
   `SHA_2_256_HashType: length-constrained named hexBinary has no C++ codec
   mapping`. `FileNameType` no longer appears.
2. `service-generate --with-codec` fails and creates no output directory.
3. The plain model and service API generate, and are byte-identical to the
   baseline CLI's output for that release.
4. `tests/cpp-codec/file_metadata_model_probe.cpp` is a **model-level** probe,
   checked with strict g++/clang++ and g++/clang++ ASan+UBSan. It
   static-asserts that `FileMetadataMDT.filename` is the required checked
   `FileNameType` carrier, constructible only through `create`, and that
   `SHA_2_Hash` is `std::optional<SHA2256HashType>`. It then checks the
   authentic carrier: `report.tar.bz2` is accepted, while `report.`, an
   embedded TAB, a trailing TAB, NUL and non-ASCII are rejected, and 255
   characters are accepted while 256 are rejected. Result:
   `CPP_OMS_006_FILE_METADATA_MODEL uci=25 checks=20` and `uci=26 checks=20`.

The task-specified codec steps did not run: decoding a hand-written FileMetadata
document, typed re-encoding, codec rejection paths, and model/API identity with
and without `--with-codec`. No FileMetadata codec exists to run them against.

## Pinned Sleet interoperability (Stage E)

**Not applicable; nothing is claimed.** No release qualifies, because
FileMetadata is codec NOT READY in both. No FileMetadata contract, Sleet
program or handwritten wire encoding was added. Sleet
`e38f61d8ce0d75c8508434a52f2ed77c69cf6a3b` and the LA-CAL runtime were used
unchanged, and only by the inherited CPP-OMS-003/004/005 regressions.
`scripts/check-cpp-oms-006-file-metadata.sh` stops with exit 4 if FileMetadata
ever becomes codec READY, so that the missing qualification is added rather
than silently skipped.

## Compatibility impact (Stage F)

Production `service-check-batch --language cpp --world closed-schema
--with-codec` covers every global element of each pin, one output exchange per
contract. It ran through `scripts/check-cpp-oms-006-codec-sweep.py`, with the
baseline (`55469a2`) and branch CLIs. 722 (2.5) and 725 (2.6) global elements
yield 676 and 679 resolvable single-message contracts. The other 46 fail
projection, for example on cyclic `QueryType` value dependencies, and are
unchanged.

| Release | Model READY | Codec READY BEFORE | AFTER | Newly READY | Lost | Renderable count moved | Renderable count fell |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 2.5 | 676 (unchanged) | 394 | **400** | 6 | 0 | 121 | 0 |
| 2.6 | 679 (unchanged) | 394 | **400** | 6 | 0 | 121 | 0 |

The population is all global elements, so the totals do not match
CPP-OMS-005's 390, which came from a 715/718-message population. Before/after
pairs here come from the same population and the same script.

Newly READY in both releases (before, the first blocker of each was a Task 059
row):

| Message | 2.5 closure, codec | 2.6 closure, codec | Before blocker |
| --- | --- | --- | --- |
| `AO_Capability` | 71, 68/69 → 69/69 | 71, 68/69 → 69/69 | `AO_PRF_CodeType` |
| `CounterSpaceReport` | 55, 52/54 → 54/54 | 55, 52/54 → 54/54 | `CounterSpaceCycleNumberType` |
| `DMPI_DesignationRequest` | 56, 53/54 → 54/54 | 56, 53/54 → 54/54 | `FileNameType` |
| `OrdersMetadata` | 82, 78/80 → 80/80 | 82, 78/80 → 80/80 | `CounterSpaceCycleNumberType` |
| `StrikeCapability` | 84, 81/82 → 82/82 | 84, 81/82 → 82/82 | `AO_PRF_CodeType` |
| `StrikeCommand` | 77, 73/74 → 74/74 | 78, 74/75 → 75/75 | `AO_PRF_CodeType` |

For each of these 12 selections, the qualification script checks the exact
readiness and that model and service API are byte-identical with and without
`--with-codec`. A probe including model, API and codec compiles under the
strict g++ flags and confirms the generated decoder refuses `{}` at the
payload root. That is a **compile smoke only**: none of these messages is
behaviorally qualified, and none is claimed as Sleet-interoperable.

First codec blockers among model-READY, codec-NOT-READY messages (production
boundary line, grouped):

| Blocker | 2.5 before → after | 2.6 before → after |
| --- | --- | --- |
| structured ASCII | 86 → **0** | 86 → **0** |
| alternating ASCII | 35 → 93 | 35 → 94 |
| Float32 (named or direct) | 64 → 69 | 69 → 76 |
| optional/repeated Choice alternative | 56 → 68 | 51 → 59 |
| named `Time` | 28 → 29 | 31 → 32 |
| abstract `$type` | 5 → 8 | 5 → 9 |
| uninhabited abstract member | 6 → 6 | 6 → 6 |
| length-constrained hexBinary | 2 → 3 (`FileMetadata`) | 2 → 3 (`FileMetadata`) |

Messages measured individually (closure; codec; first blocker):

| Message | Release | BEFORE | AFTER |
| --- | --- | --- | --- |
| `FileMetadata` | 2.5 | 52; 48/50; `FileNameType` | 52; 49/50; `SHA_2_256_HashType` |
| `FileMetadata` | 2.6 | 54; 50/52; `FileNameType` | 54; 51/52; `SHA_2_256_HashType` |
| `AMTI_Command` | 2.5 / 2.6 | 221; 212/217 / 222; 213/218; `SpoilFactorType` (Float32) | unchanged |
| `AO_CapabilityStatus` | 2.5 / 2.6 | 64; 103/130; `AO_PRF_CodeType` | 64; 114/130; `FIPS_CountryCodeType` (alternating ASCII) |

Structured ASCII made 11 more `AO_CapabilityStatus` declarations renderable,
and it does not affect `AMTI_Command`. Neither message is claimed.

## Evidence (this branch, local container)

Environment: Ubuntu 24.04, g++ 13.3.0, clang++ 18.1.3 (libstdc++ 13,
`libclang-rt-18-dev` 18.1.3), Boost 1.83.0, Python 3, Rust 1.97.0 for the CLI,
GNAT 13.3.0. Sleet `e38f61d8…` (tree `7e705f4e…`, clean) was built `--locked`
with its own toolchain (rustc 1.95.0) for the inherited regressions.

`scripts/check-cpp-oms-006-file-metadata.sh` exited 0 with
`CPP_OMS_006_BASELINE_CLI` set to the CLI built at `55469a2`. It verified the
pinned roots (2.5 `ac943049…bf27` + SecurityMarkings `4a8056f1…7a0b`; 2.6
`af54ce72…955b` + `ee6e58d5…0b88`):

| Stage | Configuration | Result |
| --- | --- | --- |
| Synthetic (shared fixture) | g++ strict, clang++ strict, g++/clang++ ASan+UBSan, g++/clang++ TSan | `CPP_OMS_006_STRUCTURED_ASCII checks=323 rejections=166` each |
| Synthetic (27 rows) | same six | `CPP_OMS_006_STRUCTURED_ROWS rows=27 checks=828 rejections=259` each |
| Negative controls | g++ | `CPP_OMS_006_MUTATIONS detected=21/21` |
| FileMetadata readiness 2.5 / 2.6 | release CLI | BEFORE 48/50 / 50/52 at `FileNameType`; AFTER 49/50 / 51/52 NOT READY at `SHA_2_256_HashType`; `--with-codec` writes nothing |
| FileMetadata model probe 2.5 / 2.6 | g++/clang++ strict, g++/clang++ ASan+UBSan | `CPP_OMS_006_FILE_METADATA_MODEL uci=25 checks=20` / `uci=26 checks=20` |
| FileMetadata codec + Sleet | — | NOT APPLICABLE (codec NOT READY), not run |
| Newly READY compile smoke | g++ strict | 12/12 (6 messages × 2 releases) |
| Skipped | — | none (`sanitizers exercised: gcc-asan-ubsan gcc-tsan clang-asan-ubsan clang-tsan`) |

The consumers are single-threaded; TSan runs them for completeness.

Generated header identities (SHA-256): synthetic shared fixture `oam.hpp`
`693c5611…b033`, `service_api.hpp` `4c886afa…398e`, `service_codec.hpp`
`263f869a…eeaa`; 27-row fixture `oam.hpp` `23b84328…a0a0`, `service_api.hpp`
`daa85b93…e3fe`, `service_codec.hpp` `42c7036c…e149`; authentic FileMetadata
(no codec) 2.5 `oam.hpp` `ba63196a…954456b5`, 2.6 `oam.hpp` `596b15a9…c21bb8d7`,
`service_api.hpp` `3c35edb6…9c93b8d9` (both releases).

## Regressions

| Check | Result |
| --- | --- |
| `scripts/check-cpp-oms-001-parity.sh` (CLI at `55469a2` vs this branch: cpp/rust/ada `service-check` + `service-generate` without the flag, rust/ada `--with-codec` readiness) | 59 pairs, 472 runs, **0 mismatches** |
| C++ `--with-codec` and Python (with and without `--with-codec`) `service-check` + `service-generate` over every fixture pair (63, incl. `tests/fixtures/cpp-codec/*`), baseline vs branch | 756 runs, **0 mismatches**; C++ codecs READY before: 33/33 byte-identical (CPP-OMS-001 golden, Float64, bounded ASCII, …); newly READY only `codec-structured-ascii` and `cpp-codec/structured-ascii`; none lost |
| Authentic FileMetadata model + API (no codec), 2.5 and 2.6, baseline vs branch | byte-identical |
| `scripts/check-cpp-oms-002-real-log.sh` | PASS: `checks=307 rejections=189` (2.5) / `192` (2.6), g++, clang++, g++ and clang++ ASan+UBSan |
| `scripts/check-cpp-oms-003-runtime.sh` | PASS: g++ and clang++ strict, ASan+UBSan, TSan (owp 6/101, mock 30/364, shutdown 12/217, oam 1/13) |
| `scripts/check-cpp-oms-003-real-log-sleet.sh` (pinned Sleet) | PASS for 2.5 and 2.6: `tests=2 checks=407`, g++ strict, clang++ strict, g++ ASan+UBSan |
| `scripts/check-cpp-oms-004-position-report.sh` (pinned Sleet) | PASS: edge `checks=305 rejections=103`, 27,167 decode / 20,474 encode rows 0 mismatches, mutation controls detected; consumer `checks=253 rejections=35` and Sleet `tests=2 checks=247` for 2.5 and 2.6, all six configurations |
| `scripts/check-cpp-oms-005-amti-settings.sh` (pinned Sleet) | PASS: `CPP_OMS_005_BOUNDED_ASCII checks=289 rejections=148` in six configurations, `detected=13/13`, AMTI `uci=25 checks=68 rejections=47` / `uci=26 checks=70 rejections=46`, Sleet `tests=2 checks=206` for 2.5 and 2.6 in six configurations |
| `cargo test -p ams-gra-oms-codegen-core -p ams-gra-oms-backend-cpp` | 598 passed, 0 failed (incl. `cpp_oms_006_structured_ascii_admission_boundary`) |
| `cargo test -p ams-gra-codegen-oms --test cpp_codec` | 23 passed (5 new `structured_ascii_*` / `task059_shared_*`) |
| `cargo test -p ams-gra-codegen-oms --test uci_structured_ascii cpp_oms_006` with both pinned roots | 1 passed (both releases) |
| `cargo test --no-fail-fast -p ams-gra-codegen-oms -p ams-gra-oms-backend-{python,rust,ada}` (GNAT 13.3.0) | 634 passed, 0 failed, 6 ignored (pre-existing `#[ignore]`) |
| `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all -- --check` | clean |
| `python3 scripts/test_pr_smoke.py` / hosted `PR smoke` | 34 tests OK / success: `CPP_OMS_006_STRUCTURED_ASCII checks=323 rejections=166`, `CPP SMOKE PASSED` |

Default C++, Rust, Python and Ada generation (without `--with-codec`) is
unchanged, per the parity rows above. The only report-text change is the
"supported:" list in C++ codec NOT READY boundary lines.

## CI

The existing single-job `PR smoke` C++ step (`python3 scripts/pr_smoke.py run
cpp`) gains one **offline** regression after the CPP-OMS-005 bounded-ASCII step
(`cpp_structured_ascii_smoke`). It generates the shared
`codec-structured-ascii` fixture through the production CLI, strict-compiles
`structured_ascii_consumer.cpp` with g++, and requires exactly
`CPP_OMS_006_STRUCTURED_ASCII checks=323 rejections=166`. Any other output, a
compile error or a missing generated file fails the job (fail-closed cases in
`scripts/test_pr_smoke.py`). No runner, matrix, workflow, network fetch, Sleet,
clang++ or sanitizer is added to PR smoke. The 27-row consumer, clang++,
ASan+UBSan and the mutation controls run in the registered `cpp_codec.rs`
tests. Both new scripts are routed as C++-owned and wired into no workflow.
The pinned-UCI test `cpp_oms_006_real_uci_cpp_codec_readiness`
(`crates/cli/tests/uci_structured_ascii.rs`, Deep CI, skipped without the
pinned roots) asserts both releases' exact readiness for the six newly READY
messages and FileMetadata's exact boundary. Main CI and the open CI repair PRs
are not touched.

## Reproduce

```sh
# synthetic suites, FileMetadata boundary and probe, newly-READY compile smoke; network for the pins
scripts/check-cpp-oms-006-file-metadata.sh [scratch]

# with BEFORE lines and the whole-release sweep
CPP_OMS_006_BASELINE_CLI=/path/to/cpp-oms-005/ams-gra-codegen-oms CPP_OMS_006_SWEEP=1 \
  scripts/check-cpp-oms-006-file-metadata.sh [scratch]

# the sweep alone
python3 -I scripts/check-cpp-oms-006-codec-sweep.py --cli target/release/ams-gra-codegen-oms \
  --baseline BASELINE_CLI --uci-2-5 ROOT25 --uci-2-6 ROOT26 --scratch DIR --require-no-loss

# registered tests and PR smoke
cargo test --locked -p ams-gra-codegen-oms --test cpp_codec
cargo test --locked -p ams-gra-oms-codegen-core cpp_oms_006
AMS_GRA_UCI_2_5_ROOT=… AMS_GRA_UCI_2_6_ROOT=… cargo test --locked -p ams-gra-codegen-oms --test uci_structured_ascii cpp_oms_006
python3 scripts/pr_smoke.py run cpp
```

## Limitations (not claimed)

* **FileMetadata is not C++ codec READY** in UCI 2.5 or 2.6. Its codec, its
  typed publish/subscribe and its Sleet interoperability are not qualified.
  The remaining blocker is `SHA_2_256_HashType` (`xs:hexBinary`, `length 32`).
  A follow-up adding length-constrained named hexBinary to the C++ codec
  (decode through the Task 053 checked binary carrier, as here for Strings)
  would make it 50/50 and 52/52, with nothing else in the way. Stages D and E
  could then run as specified.
* The six newly READY messages have only a generated compile smoke. They are
  not behaviorally qualified or claimed as Sleet-interoperable.
* Alternating ASCII, IPv6, Unicode 3.1, Float32, `Time`, constrained Binary,
  optional/repeated Choice alternatives and polymorphic constructs remain NOT
  READY for the C++ codec.
* No authentic UCI schema or generated header is committed.
