# CPP-OMS-004 — C++17 Float64 OMS JSON codecs and authentic PositionReport interoperability

Builds on CPP-OMS-001/002 (generated `service_codec.hpp`) and the merged
CPP-OMS-003 LA-CAL runtime. Started from `main`
`410d7abf6ead1eab5ca45c1d94b0510dc1f41e4c` (PR #173). The generator and the
runtime were extended, not redesigned: no runtime source, Sleet source, wire
rule or schema-validation behavior changed, and no PositionReport-specific
codec logic exists anywhere (the typed PositionReport codec is 100 %
generated).

## Result

| Release | Pin | Message (payload) | Selected types | Codec declarations BEFORE | AFTER |
| --- | --- | --- | --- | --- | --- |
| UCI 2.5 | `093610b7…`, root `ac943049…bf27` | `PositionReport` (`PositionReportMT`) | 60 / 60 READY | 50/59 NOT READY at `AccelerationType` (named Float64) | **59/59 READY** |
| UCI 2.6 | `78eb61b6…`, root `af54ce72…955b` | `PositionReport` (`PositionReportMT`) | 60 / 60 READY | 50/59 NOT READY at `AccelerationType` (named Float64) | **59/59 READY** |

BEFORE was measured with a CLI built from the starting `main` (the historical
50/59 figure was re-measured, not assumed). The nine blockers are exactly the
nine named Float64 types of the closure: `AccelerationType`, `AngleRateType`,
`DistanceOffsetType`, `SpeedType` (unconstrained) and `AltitudeType` (≥
−6378237), `AngleHalfType` (±π/2), `AngleType` (±π), `DistanceType` and
`DoubleNonNegativeType` (≥ 0). No other construct blocked PositionReport, so
no further mapping was needed. Contract: the unchanged
`tests/fixtures/service-generate/position-report-loop.yaml` (one input and one
output exchange, topic `mission.position-report`) for both releases;
generation does not depend on its `uci_schema_version` field (the generated
headers name no release).

Typed publish/subscribe of PositionReport through the generated facade and
codec, the unchanged `cpp/oms-lacal-runtime` and the unmodified pinned Sleet
`e38f61d8ce0d75c8508434a52f2ed77c69cf6a3b` is qualified for **both** releases.

## Supported and unsupported Float64 shapes

| Shape | C++ model carrier | Codec | Authority |
| --- | --- | --- | --- |
| Named Float64, no facets | `class T { explicit T(double); double value(); }` | **READY** | every binary64 (NaN, ±∞, −0) is a member |
| Named Float64, bound-only effective domain (inclusive/exclusive, one- or two-sided, restriction chains) accepted by the shared `floating_domain` classifier | `class T { static std::optional<T> create(double); … }` | **READY** | decode → `T::create`; encode re-checks with `T::create` |
| Direct unconstrained `xs:double` member (required, optional, bounded/unbounded repetition, Choice alternative, nested) | `double` | **READY** | every binary64 |
| Named or direct Float32 (`xs:float`) | `float` | NOT READY | out of scope; `service codec boundary: … Float32 …` |
| Field-local constrained direct `xs:double` | none (the model rejects it) | NOT READY | the analyzer also refuses it explicitly |
| Float64 with facets the classifier refuses (length, pattern, empty range, non-finite bound, …) | none | NOT READY | shared classifier reason |

The decision lives once, in `codegen-core::cpp_emission_codec_support`; the
renderer calls the same analyzer first, so `service-check` and
`service-generate` cannot disagree, and nothing is written for a NOT READY
selection. The codec restates no bound: it never mentions `min_value` /
`max_value` and calls only the model's `create`, so the model's NaN rejection,
one-sided infinity ordering, equal treatment of ±0 at an exclusive zero bound,
and exact subnormal bounds all apply unchanged.

## OMS JSON mapping (OMSC-SPC-013 Rev B §6.1.4)

| Value | Wire | Decode accepts | Decode rejects |
| --- | --- | --- | --- |
| finite | JSON number, shortest round-trip digits | any RFC 8259 number lexeme, correctly rounded (nearest, ties-to-even) | a number whose correctly rounded value is ±∞ (*decimal overflow*), with `number overflows binary64` |
| NaN (any payload or sign) | `"NaN"` | exactly `"NaN"` (after JSON unescaping) | `"nan"`, `"NAN"`, `"-NaN"`, `" NaN"`, … |
| +∞ | `"Infinity"` | exactly `"Infinity"` | `"inf"`, `"+Infinity"`, `"infinity"`, … |
| −∞ | `"-Infinity"` | exactly `"-Infinity"` | `"-infinity"`, … |
| — | — | — | quoted numbers (`"1.5"`, `"0"`), booleans, `null`, arrays, objects; bare `NaN`/`Infinity`, `+1`, `01`, `1.`, `.5`, `1e` are JSON syntax errors (path `$`) |

* Signed zero, every subnormal and every finite binary64 are kept bit for bit;
  `-0` encodes as `"-0"` and decodes to the negative zero.
* Legitimate underflow becomes a zero **of the lexeme's sign** (`1e-400` → +0,
  `-1e-400` → −0, `2.4703282292062327e-324` → +0, `2.4703282292062328e-324` →
  the smallest subnormal).
* A domain violation after decoding (e.g. `"NaN"` or `1.5707963267948968` for
  `AngleHalfType`) is `value rejected by the generated model's domain check`
  at the member's schema path.
* Integer members keep their CPP-OMS-001 semantics (`1.0`, `1e0` and
  `"NaN"` are still rejected for an `xs:int`).

### Conversion strategy

**Decode.** The strict reader already restricts the lexeme to the JSON number
grammar. It is converted once by `std::from_chars(first, last, d,
std::chars_format::general)`, which C++17 specifies as correctly rounded and
locale-independent; the whole lexeme must be consumed. `from_chars` reports
both overflow and underflow as `result_out_of_range` and leaves the output
unchanged, so the codec classifies the lexeme itself: if the decimal exponent
of its first significant digit is negative the correctly rounded value is
zero (signed zero is returned), otherwise it is ±∞ (rejected). That
classification is linear in the lexeme and uses saturating arithmetic, so
arbitrarily long digit or exponent strings are bounded (1 MB lexemes are
tested).

**Encode.** NaN/±∞ map to the three strings. A finite value is written with
`std::to_chars(first, last, d, std::chars_format::scientific)` *without* a
precision, which C++17 specifies as the shortest digit string that converts
back to the same value (the closest one if several exist). Those digits are
then laid out by the ECMAScript `Number::toString` rule (ECMA-262 §6.1.6.1.20:
fixed notation for decimal exponents −7 < e < 21, otherwise `d.ddde±x`), except
that negative zero keeps its sign. Moving digits around a decimal point cannot
change the value. Examples: `0`, `-0`, `1`, `0.1`, `100`, `123456789012345680000`,
`1e+21`, `0.000001`, `1e-7`, `5e-324`, `1.7976931348623157e+308`. The plain
`to_chars(d)` overload was rejected because it prints the exact integer digits
of large integral values (`123456789012345683968`), which round-trip but are
not the shortest digit string; fixed-precision or `printf`/`iostream`
formatting was rejected as locale-dependent or lossy.

**Toolchain.** The generated Float64 section `#error`s unless
`__cpp_lib_to_chars >= 201611L`, and unless the standard library is libstdc++
≥ 12 (where `from_chars` returns subnormal results and reports only infinite
or zero correctly rounded results as out of range — the behavior the
underflow rule relies on). Another library can be used only by defining
`SERVICE_CODEC_ASSUME_CONFORMING_CHARCONV` after qualifying it with the same
corpus. Qualified here: g++ 13.3.0 and clang++ 18.1.3, both on libstdc++ 13
(`__GLIBCXX__` 20240904). libc++ before 20 has no floating-point `from_chars`
and is therefore refused at compile time rather than silently approximated.
Dependency policy is unchanged: standard library only.

**Byte-identical output without Float64.** The Float64 helpers (and the
`<charconv>`, `<cmath>`, `<system_error>` includes) are emitted only when an
emitted declaration stores a Float64. All 31 fixtures whose C++ codec was
READY on `main` produce byte-identical `service_codec.hpp` (and model/API)
files on this branch, including the CPP-OMS-001 golden.

## Numerical qualification

**Independent oracle.** `tests/cpp-codec/float64_oracle.py` imports the
**unchanged** Task 096 oracle (`tests/ada-float64-probe/oracle.py`: exact
integer/rational decimal→binary64, nearest/ties-even, no host floating point)
and regenerates its frozen corpus; the manifest hashes must equal
`tests/ada-float64-probe/corpus-manifest.json`:

| Corpus | Rows | SHA-256 |
| --- | ---: | --- |
| `conversions.tsv` | 27,167 | `14c86c5fa674d279d198947dad03f71f517a40b583729df1184bbf6596a6451b` |
| `encoding.tsv` | 20,474 | `b1ad323c7d5d88c465bf966dc2edc1249c68982ad817044fa5579ce65e9abc82` |

It covers every one of the 2,047 finite exponent bands, signed zeros,
subnormal/normal boundaries, the largest finite value, adjacent values,
decimal powers 1e−324…1e308 and their neighbours, midpoint-even/odd ties,
4,000-digit tokens, 64-bit integer boundaries, underflow and the overflow
midpoint.

For the encoder the driver computes, from exact rationals only, the shortest
digit string that rounds back to each bit pattern (closest on ties) and the
ECMAScript layout, and requires each encoder token to be a JSON number whose
oracle value is the original bits **and** whose text equals that expected
spelling. Its `selftest` shows it rejects a non-shortest
(`0.10000000000000001`), non-closest (`1.7976931348623158e+308`), wrong-value,
lost-sign and wrong-layout (`1.0`, `1e21`, `0.0000001`) token.

**Generated-codec harness.** `tests/fixtures/cpp-codec/float64/` is a synthetic
fixture (not UCI) with every shape above: direct required/optional/repeated/
Choice/nested `xs:double`, an unconstrained named Float64, and named domains
`[-180,180]`, `≥ -6378237`, `> 0`, `< 1`, `[-3.5,3.5)`, `(0,1)`,
`[5e-324, 2.2250738585072014e-308)` and a two-level restriction chain
`[0,10]`. `tests/cpp-codec/float64_consumer.cpp` uses only the generated
public API:

| Mode | What | Result (every configuration) |
| --- | --- | --- |
| edge | hand-written bit patterns (61 constants re-verified against the oracle), ties, subnormal/normal/overflow boundaries, exact 2⁻¹⁰⁷⁵ midpoint (750 digits), specials and 20 wrong spellings, 6 wrong kinds, 19 non-JSON lexemes, every domain shape incl. exclusive bounds and ±0, error paths into repetition/Choice/nested Records, integer-token regression, 33 exact encodings, a fully populated document, 9 forged-carrier encode rejections, 1 MB digit/zero/exponent lexemes, every truncation and 740 single-byte corruptions | `CPP_OMS_004_FLOAT64 checks=305 rejections=103` |
| decode | all 27,167 oracle rows through both the direct and the named position | `rows=27167 finite=27162 overflow=5 mismatches=0` |
| encode | all 20,474 bit patterns → tokens → oracle check | `ORACLE_ENCODING rows=20474 mismatches=0` |
| locale | decode/encode under `de_DE.UTF-8` (decimal comma, built into scratch with `localedef`; `localeconv()` verified to be `,`) | `checks=5 failures=0` |

**The tests bite.** Six deliberately broken copies of the generated codec are
each detected by the script: unsigned-zero underflow, accepting decimal
overflow as ±∞, a binary32 intermediate, a fixed 16-digit precision, accepting
`"nan"`, and skipping the encode-side `create` re-check (also checked during
development: a 17-digit truncating decode).

## Authentic PositionReport (Stage D)

`tests/cpp-codec/position_report_documents.hpp` holds hand-written canonical
documents and oracle-computed bit patterns; `position_report_consumer.cpp`
(`-DUCI_PR_VERSION=25|26`) checks them against the generated headers:

* **Inputs.** 2.5: the existing pinned Sleet golden
  `tests/fixtures/oms-json/sleet-e38f61d8/PositionReport.json` (envelope
  stripped). 2.6: the PY-016 golden
  `tests/fixtures/python-whitespace-visible/goldens/2.6-PositionReport.json`,
  **not** the 2.5 golden: it differs in schema version, adds a NATO
  `OwnerProducer` and a `ClassifiedBy` that 2.6 preserves verbatim
  (`" a\n\r  b "`). Both are additionally validated by publishing their
  original text through the authentic Sleet of the same release (Stage E).
  Typed values (classification, owner producers, schema version, mode, source,
  domain, Latitude/Longitude/Altitude bits) are checked, and each re-encodes to
  an independently written canonical document.
* **Every Float64 member.** A full document sets all 25 Float64 members of the
  closure: π/2 and −π at the inclusive bounds, Altitude at −6378237, −0,
  `5e-324`, `±2.2250738585072014e-308`, `-1.7976931348623157e+308`, `1e+21`,
  `1e-7`, `0.30000000000000004`, `-1.5e+300`, … — every value checked by bits
  and re-encoded byte for byte; a specials document puts NaN/±∞ in every
  unconstrained kind and +∞ in the one-sided Altitude. Equal non-canonical
  spellings (`15707963267948966e-16`, `1000000000000000000000.000`,
  `2.4703282292062328e-324`, `-0.0e0`, `±1e-400`) decode to the same bits.
* **Rejections** (path + message): one-ulp-outside π/2, −π, −π/2, Altitude,
  NaN/±∞ in bounded types, a negative subnormal distance, overflow on both
  signs, 7 wrong special spellings, 4 wrong kinds, non-JSON lexemes, missing,
  duplicate and unknown members, the global-element envelope, integer
  semantics; and two forged carriers refused by the encoder.
* Result, both releases, strict g++, strict clang++ and g++ ASan+UBSan:
  `CPP_OMS_004_POSITION_REPORT uci=25|26 checks=253 rejections=35`.

Model and service API headers are byte-identical with and without
`--with-codec` for both releases.

## Pinned Sleet interoperability (Stage E)

`tests/cpp-lacal/real_position_report_sleet.cpp` connects the unchanged runtime
(`config.service_id = "position-report-loop"`, verbose INIT with the release's
schema version) to a fresh Sleet loaded with the full pinned schema of that
release, subscribes through `exchange_input_position::subscribe` and runs an
independent raw OWP observer and publisher (`raw_owp_client.hpp`, no runtime
code). The wire element is `PositionReport`; `PositionReportMT` is rejected by
Sleet as an unknown global element.

1. Golden, full Float64 and specials documents published through
   `exchange_output_position::publish`: Sleet `+OK`; the observer's actual MSG
   equals `{"PositionReport":<hand-written document>}` byte for byte (Sleet
   forwards payload text verbatim); the typed `PositionReportMT` callback is
   checked bit for bit and re-encodes exactly.
2. The raw publisher sends the release's pinned golden text itself (Sleet
   validates it: `+OK`) and a document with non-canonical spellings and
   underflow: both delivered verbatim and decoded to the expected bits.
3. Fifteen malformed documents, each followed by recovery on the **same**
   subscription:
   * **Sleet delivers, the client rejects** (subscription `decode_error` event,
     `ErrorKind::codec`, exact codec path, no callback): `"NaN"`/`"Infinity"`
     Latitude, `"-Infinity"` Altitude, `"-Infinity"` SemiMajorAxisLength,
     `"Infinity"` WanderAngle — the pinned Sleet admits the three special
     strings for any `xs:double` and skips range facets for them, while the
     generated model's domain check does not.
   * **Sleet rejects** (`-ERR Invalid-Message` to the publisher, no MSG, no
     event, no callback): one ulp above π/2, one ulp below −π, below the
     Altitude bound, a negative subnormal distance (facet violations);
     `"nan"`, `"1.5"`, `true`, `null` (type mismatch); `1e309` (`number out of
     range` in Sleet's JSON parser); missing Latitude; and `PositionReportMT`.
4. Runtime level: PUB to an unauthorized topic → `broker_rejected`
   `Illegal-State`; an invalid topic → `invalid_argument`; a forged Latitude
   carrier → local `codec` error at
   `PositionReportMT.MessageData.InertialState.Position.Latitude`, nothing sent.
5. `unsubscribe()` (`+OK`), a later publish reaches the observer but no
   callback, `close()`, no `ams-lacal-*` thread and no runtime TCP connection
   left, exact `stats()` (`delivered` 22, `decode_errors` 5).

Result per release and configuration: every marker and
`CPP_OMS_004_REAL_POSITION_REPORT_COMPLETE uci=25|26 tests=2 checks=247` — see
the evidence table below.

## Evidence (this branch, local container)

Environment: Ubuntu 24.04, g++ 13.3.0, clang++ 18.1.3 (libstdc++ 13), Boost
1.83.0 (`libboost1.83-dev 1.83.0-2.1ubuntu3.2`), Python 3 for the oracle,
Rust 1.97.0 for the CLI; Sleet `e38f61d8…` (tree `7e705f4e…`, clean) built
`--locked` with its own toolchain.

`scripts/check-cpp-oms-004-position-report.sh` (pinned roots verified: 2.5
`ac943049…bf27` + SecurityMarkings `4a8056f1…7a0b`; 2.6 `af54ce72…955b` +
`ee6e58d5…0b88`; Sleet `e38f61d8…`, tree `7e705f4e…`, clean, built with rustc
1.95.0; `CPP_OMS_004_BASELINE_CLI` = CLI built at `410d7ab`) exited 0:

| Stage | Configuration | Result |
| --- | --- | --- |
| Oracle | — | self-test 18/18; corpus hashes equal the frozen manifest |
| Numerics (synthetic) | g++ strict, clang++ strict, g++ ASan+UBSan | edge `checks=305 rejections=103`; decode 27,167 rows 0 mismatches; 20,474 encoder tokens 0 oracle mismatches |
| Locale | g++, `de_DE.UTF-8` | `checks=5 failures=0` |
| Mutation controls | g++ | 6 / 6 detected |
| Readiness 2.5 / 2.6 | release CLI | BEFORE 50/59 NOT READY at `AccelerationType`; AFTER closure 60, model READY, codec 59/59 READY; model + API byte-identical with/without `--with-codec` |
| Codec consumer 2.5 / 2.6 | g++ strict, clang++ strict, g++ ASan+UBSan | `checks=253 rejections=35` each |
| Pinned Sleet 2.5 | g++ strict, clang++ strict, g++ ASan+UBSan, g++ TSan | `tests=2 checks=247`, all markers, each |
| Pinned Sleet 2.6 | g++ strict, clang++ strict, g++ ASan+UBSan, g++ TSan | `tests=2 checks=247`, all markers, each |
| clang++ ASan+UBSan, clang++ TSan | — | **SKIPPED — not claimed** (no clang sanitizer runtime in this container) |

Generated header identities (SHA-256): UCI 2.5 `oam.hpp` `988348a4…6de4`,
`service_api.hpp` `fc8fae2c…9496`, `service_codec.hpp` `745257dc…d752`; UCI 2.6
`oam.hpp` `14826d6a…f0a4`, `service_api.hpp` `fc8fae2c…9496` (identical: the
API names no release), `service_codec.hpp` `486a744d…ff3d`; synthetic Float64
`oam.hpp` `dbe3a691…7474`, `service_codec.hpp` `7bf19781…1782`.

## Regressions

| Check | Result |
| --- | --- |
| `scripts/check-cpp-oms-001-parity.sh` (baseline `410d7ab` CLI vs this branch: cpp/rust/ada `service-check` + `service-generate` without the flag, rust/ada `--with-codec` readiness) | 59 pairs, 472 runs, **0 mismatches** |
| Python generator, same sweep (`--language python`, both worlds, with and without `--with-codec`) | 472 runs, **0 mismatches** |
| C++ `--with-codec` over every fixture pair (61): codecs READY on `main` | 31 / 31 byte-identical (incl. the CPP-OMS-001 golden); only the new Float64 fixture newly READY; 29 NOT READY on both |
| Authentic Log codec headers (2.5, 2.6), `main` vs branch | byte-identical |
| `scripts/check-cpp-oms-002-real-log.sh` | PASS: `checks=307 rejections=189` (2.5) / `192` (2.6) with g++, clang++, g++ ASan+UBSan; clang++ sanitizers SKIPPED |
| `scripts/check-cpp-oms-003-runtime.sh` | PASS: g++ strict, clang++ strict, g++ ASan+UBSan, g++ TSan (owp 6/101, mock 30/364, shutdown 12/217, oam 1/13); clang++ sanitizers SKIPPED |
| `scripts/check-cpp-oms-003-real-log-sleet.sh` (pinned Sleet) | PASS for 2.5 and 2.6: `tests=2 checks=407` with g++ strict, clang++ strict, g++ ASan+UBSan |
| `cargo test -p ams-gra-codegen-oms --test cpp_codec --test service_codec` | 13 + 28 passed (`task050_ada_and_cpp_have_no_generated_codec` updated: `codec-oam`'s first C++ blocker is now the named Float32 `Ratio`, still NOT READY) |
| `cargo test -p ams-gra-oms-codegen-core -p ams-gra-oms-backend-cpp` | 596 passed, 0 failed, 0 ignored (incl. `cpp_oms_004_float64_admission_boundary`) |
| `python3 scripts/test_pr_smoke.py` / `python3 scripts/pr_smoke.py run cpp` | 34 tests OK / `CPP SMOKE PASSED` (≈ 90 s) |
| `cargo test --no-fail-fast -p ams-gra-codegen-oms -p ams-gra-oms-backend-{python,rust,ada}` | 614 passed, 6 ignored (pre-existing `#[ignore]`), 9 **not run**: every one spawns `gnat`/`gnatmake`, which is not installed in this container (`NotFound`) — not claimed |

Unsupported constructs still fail before output is written: `codec-ada-float64`
(direct `xs:float`), `codec-ada-float32`, `constrained-float` (named Float32)
and the CPP-OMS-002 list are asserted NOT READY with no output directory by
`unsupported_constructs_are_rejected_before_any_output`.

## CI

The existing single-job `PR smoke` C++ step (`python3 scripts/pr_smoke.py run
cpp`) gains an **offline** Float64 regression after the CPP-OMS-003 runtime
suites (`cpp_float64_smoke`): generate the synthetic Float64 fixture through
the production CLI; regenerate the frozen oracle corpus and its focused subset
(every non-exhaustive conversion row and every 128th encoding: 6,693 / 161
rows, subset hash pinned); run the oracle self-test; strict-compile
`float64_consumer.cpp` with g++; and require exactly
`CPP_OMS_004_FLOAT64 checks=305 rejections=103`,
`CPP_OMS_004_DECODE rows=6693 finite=6688 overflow=5 mismatches=0`,
`CPP_OMS_004_ENCODE rows=161 roundtrip_mismatches=0` and
`CPP_OMS_004_ORACLE_ENCODING rows=161 mismatches=0`. Any other output, a
compile error or a missing generated file fails the job. No runner, matrix,
workflow, Sleet build, UCI fetch, clang++ or sanitizer was added to PR smoke;
the whole C++ step took about 90 s locally. `scripts/check-cpp-oms-004-position-report.sh`
is routed as C++-owned but is not wired into any workflow. The full corpus,
clang++ and ASan+UBSan run in the registered `cpp_codec.rs` tests
(`float64_section_is_emitted_only_for_float64_selections`,
`float64_numerics_match_the_independent_oracle`,
`float64_numerics_are_clean_under_sanitizers`).

## Reproduce

```sh
# authentic PositionReport (2.5 + 2.6), numerics, Sleet; network for the pins
scripts/check-cpp-oms-004-position-report.sh [scratch] [existing-sleet-checkout]

# optional BEFORE line from a CLI built at the starting main
CPP_OMS_004_BASELINE_CLI=/path/to/main/ams-gra-codegen-oms scripts/check-cpp-oms-004-position-report.sh

# registered tests and PR smoke
cargo test --locked -p ams-gra-codegen-oms --test cpp_codec
cargo test --locked -p ams-gra-oms-codegen-core cpp_oms_004
python3 scripts/pr_smoke.py run cpp

# no-codec / other-language parity against a baseline CLI
scripts/check-cpp-oms-001-parity.sh BASELINE_CLI target/release/ams-gra-codegen-oms
```

## Limitations (not claimed)

* Float32, field-local constrained direct Float64, and every construct listed
  as NOT READY in CPP-OMS-002 other than Float64 remain unsupported. Of the
  other messages CPP-OMS-002 measured, `AMTI_Command` now stops at
  `EmptyType` (bounded ASCII String; 211/217 in 2.5, 212/218 in 2.6) and
  `AO_CapabilityStatus` at `AO_PIM_CodeType` (bounded ASCII; 85/130) — neither
  is claimed.
* OMS does not carry NaN payloads or signs: every NaN encodes as `"NaN"` and
  decodes to the default quiet NaN.
* Only libstdc++ ≥ 12 `<charconv>` is qualified; other standard libraries are
  refused at compile time unless the integrator opts in after qualification.
* The pinned Sleet admits the special strings on range-constrained Float64
  members; the generated C++ client rejects them as subscription decode
  errors. That broker behavior was observed and is not changed here.
* Only loopback `ws://` with the pinned Sleet is qualified (CPP-OMS-003
  limits apply). No authentic UCI schema or generated byte is committed.
