# CPP-OMS-005 — C++17 bounded-ASCII OMS JSON codecs and authentic AMTI_SettingsCommand interoperability

Builds on CPP-OMS-001 through CPP-OMS-004. Started from `main`
`afe0c510866c08fc188fef335e74879840794a70` (PR #177). The C++ model
generator, the generated service API and the LA-CAL runtime are unchanged;
no Sleet source, wire rule or schema-validation behavior changed, and no
AMTI-specific codec logic exists anywhere (the typed codec is 100 %
generated).

## Result

| Release | Pin | Message (payload) | Selected types | Codec declarations BEFORE | AFTER |
| --- | --- | --- | --- | --- | --- |
| UCI 2.5 | `093610b7…`, root `ac943049…bf27` | `AMTI_SettingsCommand` (`AMTI_SettingsCommandMT`) | 61 / 61 READY | 58/59 NOT READY at `EmptyType` (bounded ASCII) | **59/59 READY** |
| UCI 2.6 | `78eb61b6…`, root `af54ce72…955b` | `AMTI_SettingsCommand` (`AMTI_SettingsCommandMT`) | 61 / 61 READY | 58/59 NOT READY at `EmptyType` (bounded ASCII) | **59/59 READY** |

BEFORE was measured with a CLI built from the starting `main`, for both
releases separately (the historical 61/59 figures were re-measured, not
assumed; the 2.6 closure was not inferred from 2.5). The only blocker in either
release was `EmptyType` (`[a-zA-Z]{0}`, `length 0`), reached through the
optional `AMTI_SettingsCommandMDT.UnassignAll`.

Typed publish/subscribe of `AMTI_SettingsCommand` through the generated facade
and codec, the unchanged `cpp/oms-lacal-runtime` and the unmodified pinned
Sleet `e38f61d8ce0d75c8508434a52f2ed77c69cf6a3b` is qualified for **both**
releases.

## Why codec readiness rejected bounded ASCII (Stage A)

Task 058 already lowered `StringProfile::BoundedAscii` into a checked C++ model
carrier (`static std::optional<T> create(std::string_view)`, `const
std::string& value()`, no default constructor, copy only), and C++ **model**
readiness was READY for `AMTI_SettingsCommand`. The C++ **codec** analyzer
(`codegen-core::cpp_emission_codec_support`) admitted only the five String
profiles the CPP-OMS-002 `Log` vertical needed (UUID, schema-version, visible
ASCII, whitespace-visible, NATO special words) and reported every other
profile as `named String profile <label> has no C++ codec mapping`. The C++
renderer already rendered any admitted named String through the model's
`create`/`value()`; nothing else was missing.

## Supported and unsupported String profiles

| Profile (shared classifier) | C++ model carrier | C++ codec | Since |
| --- | --- | --- | --- |
| UUID, UCI schema-version, visible ASCII, whitespace-visible, NATO special words | checked `create` | READY | CPP-OMS-002 |
| **Task 058 bounded ASCII** — exactly the 60 pinned `(alphabet, spelling, length)` rows (19 alphabets, 61 UCI declarations per release), incl. zero-length `EmptyType` | checked `create` | **READY** | **CPP-OMS-005** |
| Structured ASCII, alternating ASCII, IPv6 address, Unicode 3.1 | checked `create` | NOT READY | — |
| Unpinned neighbours (e.g. `[A-Z0-9 ]{9}`, `maxLength`-only, unobserved spellings or bounds, an explicit `whiteSpace`) | none (the model refuses them) | NOT READY | — |
| Unconstrained named String | — | NOT READY | — |

The decision lives once, in the new
`codegen-core::cpp_codec_admits_string_profile`, which matches every
`StringProfile` variant explicitly (a future profile cannot become codec-ready
by default). The analyzer uses it, and the C++ renderer re-asks it before it
emits a String carrier codec, so `service-check` and `service-generate` cannot
disagree and a direct renderer caller cannot obtain a codec for an
unsupported profile. Nothing is written for a NOT READY selection.

### Mapping

* **Decode**: the member must be a JSON string (any other kind is `expected
  string, found <kind>`); its unescaped UTF-8 text is passed unchanged to the
  model's `T::create`; a refusal is `value rejected by the generated model's
  <Type> check` at the member's schema path.
* **Encode**: the carrier's `value()` is written unchanged with only the
  JSON-mandatory escapes (`"`, `\`, controls).
* The codec restates no character class, length, pattern or normalization rule
  (asserted by `bounded_ascii_codec_is_ready_delegating_and_separate`: the
  generated codec contains no `kLength`, `is_member`, ordinal or regex text). No
  regex engine or dependency was added; output for existing profiles is
  byte-identical.
* Exact case-sensitive ASCII membership, fixed and min/max length, and
  `whiteSpace = preserve` all come from the model: leading/trailing/all-SPACE
  values are legal exactly where SPACE is a member and are stored verbatim.
  The model checks the length on bytes and then requires every byte to be a
  class member; every member is ASCII, so for an accepted value bytes equal
  characters, and a multi-byte UTF-8 value whose byte count happens to equal
  the length (`"abcdé"` for a 6-character type) still fails the class check.
* **`EmptyType`**: an optional `EmptyType` member is
  `std::optional<EmptyType>`. Present `""` decodes to an engaged optional and
  re-encodes as `""`; absent stays absent (no `""` is invented). `" "`, `"x"`,
  `"\u0000"`, any other non-empty string, numbers, `null`, arrays, objects and
  booleans are rejected.
* An encoder cannot meet an invalid bounded-ASCII value: the carrier is only
  constructible through `create` (statically asserted by the consumers), so
  there is no forged-carrier path to test, unlike Float64.

## Synthetic qualification (Stage C)

`tests/fixtures/cpp-codec/bounded-ascii/` is a synthetic OAM fixture (not UCI)
with twelve pinned rows under non-UCI names: the `EmptyType` shape
(`[a-zA-Z]{0}`), `[A-Z0-9 ]{8}`, `[a-zA-Z0-9]{4}`, `[a-zA-Z0-9]{1,20}`,
`[a-zA-Z0-9 \-_]{1,20}`, USMTF punctuation `[\-\.,\(\)\?A-Z0-9 ]{1,8}`,
`[a-zA-Z0-9 '\(\).,@;+\-]{1,256}`, the USMTF serial class (members include
`"`, `\`, `&`, `<`, `>`), `[0-9]{4,5}`, `[0-7]{4}`, `[ -~]{6}` and
`[A-Z]{1,3}`; in required, optional (incl. the optional zero-length
`UnassignAll`), bounded 0..3 and 2..4, unbounded, Choice, repeated-Choice
and nested-Record positions.

`tests/cpp-codec/bounded_ascii_consumer.cpp` uses only the generated public
API and hand-written JSON (behavioral reference: the Rust `codec-bounded-ascii`
tests):

* zero-length `EmptyType` required/optional/nested/repeated/Choice; present
  `""` vs absent in both directions;
* fixed lengths, bounded lengths at and one beyond both ends, character-class
  violations, wrong case, non-ASCII UTF-8 (incl. fullwidth digits and NBSP),
  TAB/LF/NUL/DEL via escapes, leading/trailing SPACE preserved;
* JSON escapes of class members (`\"`, `\\`, `\u0041`) decode to exactly
  those characters and re-encode canonically; an escaped solidus is still `/`
  (not a member);
* wrong JSON kinds, duplicate members (also after key unescaping), unknown and
  wrong-case members, the global-element envelope, raw control characters and
  ill-formed UTF-8 (JSON errors at `$`), cardinality violations;
* every rejection asserts its exact schema path (e.g.
  `BoundedPayloadType.Picks[1].Launch`) and message; encoding of model-built
  values is compared byte for byte.

Result in every configuration: `CPP_OMS_005_BOUNDED_ASCII checks=289
rejections=148`.

**Negative controls.** `tests/cpp-codec/bounded_ascii_mutations.py` builds 13
deliberately weakened copies of the generated headers and requires the
consumer to fail on each: model ignores the `EmptyType` length; fixed-length
type ignores its class; max length off by one; min length off by one; upper
letters accept lower case; visible ASCII accepts non-ASCII bytes; a SPACE class
accepts TAB; codec trims before `create`; present `""` decodes as absent;
present `""` encodes as absent; a JSON number accepted as text; a wrong error
path; an unescaped `"` on encode. Result: `CPP_OMS_005_MUTATIONS detected=13/13`
(also a registered test).

The shared Task 058 fixture `codec-bounded-ascii` (used by the Rust codec) is
now C++ codec READY as well and round-trips the same documents the Rust
mock-OWP test uses (`task058_shared_bounded_ascii_fixture_round_trips_in_cpp`).

## Authentic AMTI_SettingsCommand (Stage D)

`tests/cpp-codec/amti_settings_documents.hpp` holds hand-written canonical
documents per release; `amti_settings_consumer.cpp` (`-DUCI_AMTI_VERSION=25|26`)
checks them against headers generated by production `service-generate`.

**2.5 and 2.6 differ** and are authored separately, not adapted:

| | UCI 2.5 | UCI 2.6 |
| --- | --- | --- |
| `MessageOutput` | `AMTI_MessageOutputsEnum`, 0..4, in `AMTI_SettingsCommandMDT` after `EmissionBalanceState` | `CapabilityMessageOutputsEnum` (49 values), 0..49, moved to `CapabilitySettingsCommandBaseType` after `RepresentationKey` (so before `ElectronicProtectionOptions` on the wire) |
| `DeclassDate` carrier | direct `xs:dateTime` | named `DateTimeType` |
| global element version | `003.000.004.000` | `004.000.005.000` |
| `UnassignAll : EmptyType` | optional, identical | optional, identical |

The consumer:

1. decodes a document with `"UnassignAll":""` and inspects the typed checked
   `EmptyType` (engaged, `value()` empty, equal to `EmptyType::create("")`);
2. re-encodes to the independently written canonical document;
3. verifies an absent `UnassignAll` stays absent both ways (also for values
   built through the model API);
4. rejects `" "`, `"x"`, `"A"`, `"  "`, TAB, LF, NUL, `"é"`, `"\u0020"` (model
   check) and a number, `null`, `[]`, `[""]`, `{}`, `true` (JSON kind) — all at
   exactly `AMTI_SettingsCommandMT.MessageData.UnassignAll` — duplicate
   `UnassignAll` at `$`, and wrong-case/Clark-form keys as unknown members;
5. checks the full document (every MessageData member of the closure: MDF
   activation, rank 65535, capability state, representation key, EP options,
   emission balance, message outputs, `DefaultEmcon` Choice), whitespace and
   member reordering, release-specific `MessageOutput` domain (`ACTION_ACTIVITY`
   rejected in 2.5, accepted in 2.6) and maxOccurs (5 vs 50 items);
6. checks missing required members, malformed UUID / timestamp /
   schema-version, rank 65536 / −1 / `"65535"` / `65535.0`, enumeration case,
   empty Choice, exact underscored wire names (`MDF_ID`, `ActivateMDF`), the
   envelope, `$type`, and an out-of-range enumerator refused by the encoder.

Result: `CPP_OMS_005_AMTI_SETTINGS uci=25 checks=68 rejections=47` and
`uci=26 checks=70 rejections=46` with strict g++, strict clang++, g++ and
clang++ ASan+UBSan. Model and service API headers are byte-identical with and
without `--with-codec` for both releases, and both the production contract
(`real-bounded-ascii-amti-settings.yaml`) and the test-only loop contract are
READY 59/59.

## Pinned Sleet interoperability (Stage E)

`tests/cpp-lacal/real_amti_settings_sleet.cpp` connects the unchanged runtime
(`service_id = "amti-settings-loop"`, verbose INIT with the release's schema
version) to a fresh Sleet loaded with the full pinned schema of that release,
subscribes through `exchange_input_settings::subscribe` (payload bound to the
generated `encode_/decode_AMTISettingsCommandMT` via
`AMS_GRA_LACAL_PAYLOAD_CODEC`), and runs an independent raw OWP observer and
publisher (`raw_owp_client.hpp`, no runtime code). Contract:
`tests/fixtures/service-generate/amti-settings-loop.yaml` (test-only, one input
and one output exchange on `amti.settings`); the production fixture is
unchanged.

1. Present-`""`, absent and full documents published through
   `exchange_output_settings::publish`: Sleet `+OK`; the observer's MSG equals
   `{"AMTI_SettingsCommand":<hand-written document>}` byte for byte (absent
   really has no `UnassignAll` key); the typed callback is checked and
   re-encodes exactly.
2. The raw publisher sends a reordered, whitespace-laden `UnassignAll:""`
   document and an absent document: delivered verbatim, decoded to the same
   typed values.
3. Twelve malformed documents, each followed by recovery on the **same**
   subscription. The two rejection authorities are kept apart:
   * **Sleet schema rejection** (`-ERR Invalid-Message` to the publisher, no
     MSG, no client event, no callback). The fragment is Sleet's own
     diagnostic, observed, e.g. `FacetViolation { path:
     "AMTI_SettingsCommand.MessageData.UnassignAll", facet: "pattern" …` for
     `" "` and `"x"` (Sleet checks the pattern before the length),
     `TypeMismatch { … expected: "string", found: "number" }` / `"null"`,
     `OccurrenceMismatch { … expected_array: false }` for `[]`, `UnknownMember`
     for `Unassignall`, a duplicate whose last value is `" "`, enumeration case,
     missing `CommandState`, rank 65536 (`builtinRange`), `MessageOutput` above
     maxOccurs, and the wrong global element `AMTI_SettingsCommandMT`.
   * **Generated C++ decoder rejection** (Sleet `+OK`, the MSG crosses Sleet,
     the subscription reports `decode_error` with `ErrorKind::codec`, no
     callback): `"UnassignAll":" ","UnassignAll":""` — Sleet's JSON reader keeps
     the last duplicate (a valid `""`) and forwards the bytes verbatim; the
     generated decoder rejects duplicate members at `$` (`duplicate object
     member`). For bounded ASCII the pinned Sleet enforces type, pattern and
     length itself, so every single-valued invalid `UnassignAll` is stopped at
     the broker.
4. Runtime level: PUB to an unauthorized topic → `broker_rejected`
   `Illegal-State`; an invalid topic → `invalid_argument`; an out-of-range
   `CommandState` enumerator → local `codec` error at
   `AMTI_SettingsCommandMT.MessageData.CommandState`, nothing sent.
5. `unsubscribe()` acknowledged (`+OK`), a later publish reaches the observer
   but no callback, `close()`, no `ams-lacal-*` thread and no runtime TCP
   connection left, exact `stats()` (`delivered` 19, `decode_errors` 1).

Result per release and configuration: every marker and
`CPP_OMS_005_REAL_AMTI_SETTINGS_COMPLETE uci=25|26 tests=2 checks=206`.

## Compatibility impact (Stage F)

C++ `--with-codec` readiness, measured with the baseline (`afe0c51`) and branch
CLIs on both pins (selected closure; codec renderable/emitted):

| Message | Release | BEFORE | AFTER | Remaining blockers after bounded ASCII (complete list) |
| --- | --- | --- | --- | --- |
| `AMTI_SettingsCommand` | 2.5 / 2.6 | 61; 58/59 at `EmptyType` | 61; **59/59 READY** | none |
| `AMTI_Command` | 2.5 | 221; 211/217 at `EmptyType` | 221; 212/217 NOT READY at `SpoilFactorType` | named Float32 `SpoilFactorType`, `UnitIntervalFloatType`; direct Float32 `MetricValueType.Value`; non-required-single Choice alternatives `OrbitalKinematicsStandardEphemerisType.J2K_StateVector`, `PolygonPointChoiceType.Point2D` |
| `AMTI_Command` | 2.6 | 222; 212/218 at `EmptyType` | 222; 213/218 NOT READY at `SpoilFactorType` | the same five |
| `AO_CapabilityStatus` | 2.5 / 2.6 | 64 (+68 generated support); 85/130 at `AO_PIM_CodeType` | 103/130 NOT READY at `AO_PRF_CodeType` | 11 structured ASCII (e.g. `AO_PRF_CodeType`, `MeterUnitLetterType`, `NumericStringLength1Type`, `NITF_FileSecurityClassificationType`), 11 alternating ASCII (e.g. `FIPS_CountryCodeType`, `NITF_UTC_TimeType`), 3 Unicode 3.1 (`NITF_DateType`, `NITF_DateAndTimeType`, `NITF_MSTGTA_TargetLocationType`), direct Float32 `JPEG_SettingsType.CompressionRatio`, abstract `$type` `NITF_PackingPlanPET` |

Bounded ASCII made 1 more `AMTI_Command` declaration and 18 more
`AO_CapabilityStatus` declarations renderable; neither message is claimed.
The complete blocker lists came from a scratch copy of the CLI instrumented to
print every codec blocker (not committed; the production report names only the
first). The whole-release sweep of every global message is below.

**Whole-release sweep.** Every global message of each pin (715 in 2.5, 718 in
2.6; one output exchange per contract) was checked with production
`service-check-batch --language cpp --world closed-schema --with-codec`
(one schema load per run) using the baseline and branch CLIs:

| Release | Model READY | Codec READY BEFORE | AFTER | Newly READY | Lost | Messages whose codec count moved |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 2.5 | 669 (unchanged) | 346 | **390** | 44 | 0 | 267 |
| 2.6 | 672 (unchanged) | 346 | **390** | 44 | 0 | 267 |

Bounded ASCII was the first codec blocker of 254 (2.5) / 255 (2.6)
model-READY messages before, and of none after. The first blockers that remain
(2.5 / 2.6): structured ASCII 85 / 85, Float32 63 / 68, optional or repeated
Choice alternatives 56 / 51, alternating ASCII 35 / 35, named `Time` 27 / 30,
abstract `$type` 11 / 11, length-constrained named hexBinary 2 / 2.

The same 44 messages became codec READY in both releases:
`AMTI_Capability`, `AMTI_SettingsCommand`, `ActionCapability`,
`AirSampleCapability`, `CargoDeliveryCapability`, `CommTerminalCapability`,
`CommandedTimelineSettingsCommand`, `ControlStatus`, `CounterSpaceCapability`,
`CounterSpaceDeconfliction`, `CryptoCommand`, `DE_CapabilityStatus`,
`DE_SettingsCommand`, `DoorStatus`, `EA_SettingsCommand`, `EffectCapability`,
`FlightCapability`, `GatewayCapability`, `IFF_Capability`,
`IFF_SettingsCommand`, `Mission`, `MissionPlanMetricsReportRequest`,
`Operator`, `OrbitChangeCapability`, `OrbitConjunctionAssessment`,
`OrbitalSurveillanceCapability`, `OrbitalSurveillanceSensorCapability`,
`Package`, `PlanModificationRequest`, `PlanScores`, `PlanScoresRequest`,
`RadarAltimeterCapability`, `RadarAltimeterSettingsCommand`,
`RefuelCapability`, `RefuelCapabilityStatus`, `RefuelCommand`,
`RefuelSettingsCommand`, `ResponseCapability`, `ServiceRegistrationRequest`,
`SharedApertureSupportCapabilityStatus`, `StoreManagementCommand`,
`TacticalOrderCapability`, `WeatherRadarCapability`,
`WeatherRadarSettingsCommand`.

For each of those 88 selections the branch CLI generated model, service API
and codec, and a probe including all three compiled under the strict g++ flags
and confirmed the generated decoder rejects `{}` at the payload root (88/88).
That is a **compile smoke only**: apart from `AMTI_SettingsCommand`, none of
these messages is behaviorally qualified or claimed as supported.

## Evidence (this branch, local container)

Environment: Ubuntu 24.04, g++ 13.3.0, clang++ 18.1.3 (libstdc++ 13; clang
sanitizer runtime `libclang-rt-18-dev` 18.1.3), Boost 1.83.0
(`libboost1.83-dev 1.83.0-2.1ubuntu3.2`), Python 3, Rust 1.97.0 for the CLI;
Sleet `e38f61d8…` (tree `7e705f4e…`, clean) built `--locked` with its own
toolchain (rustc 1.95.0).

`scripts/check-cpp-oms-005-amti-settings.sh` (pinned roots verified: 2.5
`ac943049…bf27` + SecurityMarkings `4a8056f1…7a0b`; 2.6 `af54ce72…955b` +
`ee6e58d5…0b88`; Sleet `e38f61d8…`, tree `7e705f4e…`, clean;
`CPP_OMS_005_BASELINE_CLI` = CLI built at `afe0c51`) exited 0:

| Stage | Configuration | Result |
| --- | --- | --- |
| Synthetic bounded ASCII | g++ strict, clang++ strict, g++ ASan+UBSan, g++ TSan, clang++ ASan+UBSan, clang++ TSan | `CPP_OMS_005_BOUNDED_ASCII checks=289 rejections=148` each |
| Negative controls | g++ | `CPP_OMS_005_MUTATIONS detected=13/13` |
| Readiness 2.5 / 2.6 | release CLI | BEFORE 58/59 NOT READY at `EmptyType`; AFTER closure 61, model READY, codec 59/59 READY (production and loop contracts); model + API byte-identical with/without `--with-codec` |
| Codec consumer 2.5 | g++ strict, clang++ strict, g++ ASan+UBSan, clang++ ASan+UBSan | `CPP_OMS_005_AMTI_SETTINGS uci=25 checks=68 rejections=47` each |
| Codec consumer 2.6 | same | `CPP_OMS_005_AMTI_SETTINGS uci=26 checks=70 rejections=46` each |
| Pinned Sleet 2.5 | g++ strict, clang++ strict, g++ ASan+UBSan, g++ TSan, clang++ ASan+UBSan, clang++ TSan | `tests=2 checks=206`, all markers, each |
| Pinned Sleet 2.6 | same six | `tests=2 checks=206`, all markers, each |
| Skipped | — | none (`sanitizers exercised: gcc-asan-ubsan gcc-tsan clang-asan-ubsan clang-tsan`) |

The consumers are single-threaded, so TSan is applied to the Sleet program
(runtime threads) only. clang++ sanitizers ran here because
`libclang-rt-18-dev` was installed in this container (CPP-OMS-004 could not).

Generated header identities (SHA-256): UCI 2.5 `oam.hpp` `8ab5720f…8bfd`,
`service_api.hpp` `8ea38fc9…3d7c`, `service_codec.hpp` `5c4d7d80…182f`; UCI
2.6 `oam.hpp` `96d5334f…b6a5`, `service_api.hpp` `8ea38fc9…3d7c` (identical:
the API names no release), `service_codec.hpp` `71691f48…185b`; synthetic
bounded ASCII `oam.hpp` `8182fea2…85f`, `service_api.hpp` `564f38f4…99`,
`service_codec.hpp` `c4e1123f…849b`.

## Regressions

| Check | Result |
| --- | --- |
| `scripts/check-cpp-oms-001-parity.sh` (baseline `afe0c51` CLI vs this branch: cpp/rust/ada `service-check` + `service-generate` without the flag, rust/ada `--with-codec` readiness) | 59 pairs, 472 runs, **0 mismatches** |
| Python generator, same sweep plus the three `tests/fixtures/cpp-codec` pairs (`--language python`, `service-check` and `service-generate`, with and without `--with-codec`) | 248 runs, **0 mismatches** |
| C++ `--with-codec` over every fixture pair (62): codecs READY on `main` | 32 / 32 byte-identical (incl. the CPP-OMS-001 golden and the Float64 fixture); newly READY only `codec-bounded-ascii` and the new `cpp-codec/bounded-ascii`; 28 NOT READY on both; none lost |
| Authentic Log (`real-duration-log.yaml`) and PositionReport (`position-report-loop.yaml`) model/API/codec headers, 2.5 and 2.6, `main` vs branch | byte-identical |
| `scripts/check-cpp-oms-002-real-log.sh` | PASS: `checks=307 rejections=189` (2.5) / `192` (2.6) with g++, clang++, g++ and clang++ ASan+UBSan |
| `scripts/check-cpp-oms-003-runtime.sh` | PASS: g++ and clang++ strict, ASan+UBSan and TSan (owp 6/101, mock 30/364, shutdown 12/217, oam 1/13) |
| `scripts/check-cpp-oms-003-real-log-sleet.sh` (pinned Sleet) | PASS for 2.5 and 2.6: `tests=2 checks=407` with g++ strict, clang++ strict, g++ ASan+UBSan |
| `scripts/check-cpp-oms-004-position-report.sh` | PASS: oracle self-test 18/18, edge `checks=305 rejections=103`, 27,167 decode / 20,474 encode rows 0 mismatches (g++/clang++ strict and ASan+UBSan), 6/6 mutation controls; consumer `checks=253 rejections=35` and pinned Sleet `tests=2 checks=247` for 2.5 and 2.6 in all six configurations incl. clang++ TSan (see the script fix below) |
| `cargo test -p ams-gra-codegen-oms --test cpp_codec` | 18 passed (incl. the five new `bounded_ascii_*` / `task058_shared_*` tests; `unsupported_constructs_are_rejected_before_any_output` now lists structured ASCII and Unicode 3.1 instead of bounded ASCII) |
| `cargo test -p ams-gra-oms-codegen-core -p ams-gra-oms-backend-cpp` | 597 passed, 0 failed, 0 ignored (incl. `cpp_oms_005_bounded_ascii_admission_boundary`) |
| `cargo test --no-fail-fast -p ams-gra-codegen-oms -p ams-gra-oms-backend-{python,rust,ada}` | 619 passed, 6 ignored (pre-existing `#[ignore]`), 9 failed on the first run only because `gnatmake` was absent (`NotFound`); after installing GNAT 13.3.0 the eight affected targets were re-run: 3 + 5 + 68 + 64 + 1 + 1 + 1 + 1 passed, 0 failed |
| `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all -- --check` | clean |
| `python3 scripts/test_pr_smoke.py` / `python3 scripts/pr_smoke.py run cpp` | 34 tests OK / `CPP SMOKE PASSED` (≈ 110 s locally) |

Unsupported constructs still fail before output is written:
`codec-structured-ascii`, `codec-alternating-ascii`, `codec-ipv6`,
`codec-unicode31` (each with its profile named in the boundary line), the
Float32/`Time`/constrained-hexBinary/abstract/unqualified fixtures, all asserted
NOT READY with no output directory by
`unsupported_constructs_are_rejected_before_any_output`.

**Script fix.** `scripts/check-cpp-oms-004-position-report.sh` selected TSan
flags with `[[ "$2" == g++ ]] && flags+=(-Wno-tsan)` as the last command of a
function; with a usable clang++ TSan (possible here, not in the CPP-OMS-004
container) the false test became the function's status and `set -e` stopped
the script. It now uses an `if`; the flags are unchanged. The new script
avoids the same pattern.

## CI

The existing single-job `PR smoke` C++ step (`python3 scripts/pr_smoke.py run
cpp`) gains one **offline** regression after the CPP-OMS-004 Float64 step
(`cpp_bounded_ascii_smoke`): generate the synthetic bounded-ASCII fixture
through the production CLI, strict-compile `bounded_ascii_consumer.cpp` with
g++, and require exactly `CPP_OMS_005_BOUNDED_ASCII checks=289 rejections=148`.
Any other output, a compile error or a missing generated file fails the job.
No runner, matrix, workflow, network fetch, Sleet build, clang++ or sanitizer
was added to PR smoke. `scripts/check-cpp-oms-005-amti-settings.sh` is routed
as C++-owned but is not wired into any workflow. clang++, ASan+UBSan and the
mutation controls run in the registered `cpp_codec.rs` tests. The Deep CI test
`task058_real_uci_newly_ready_service_generates_and_compiles` (pinned UCI 2.5)
now also requires C++ codec READY 59/59 and compiles the generated codec.

## Reproduce

```sh
# authentic AMTI_SettingsCommand (2.5 + 2.6), synthetic suite, Sleet; network for the pins
scripts/check-cpp-oms-005-amti-settings.sh [scratch] [existing-sleet-checkout]

# optional BEFORE lines from a CLI built at the starting main
CPP_OMS_005_BASELINE_CLI=/path/to/main/ams-gra-codegen-oms scripts/check-cpp-oms-005-amti-settings.sh

# registered tests and PR smoke
cargo test --locked -p ams-gra-codegen-oms --test cpp_codec
cargo test --locked -p ams-gra-oms-codegen-core cpp_oms_005
python3 scripts/pr_smoke.py run cpp

# no-codec / other-language parity against a baseline CLI
scripts/check-cpp-oms-001-parity.sh BASELINE_CLI target/release/ams-gra-codegen-oms
```

## Limitations (not claimed)

* Structured ASCII, alternating ASCII, IPv6, Unicode 3.1, Float32, `Time`,
  constrained Binary, optional/repeated Choice alternatives and polymorphic
  constructs remain NOT READY for the C++ codec. (Structured ASCII is added by
  [CPP-OMS-006](task-cpp-oms-006-structured-ascii-file-metadata.md).) `AMTI_Command` and
  `AO_CapabilityStatus` are not claimed.
* The pinned Sleet keeps the last of duplicate JSON members; the generated
  C++ client rejects such a document as a subscription decode error. That
  broker behavior was observed and is not changed here.
* Only loopback `ws://` with the pinned Sleet is qualified (CPP-OMS-003
  limits apply). No authentic UCI schema or generated header is committed.
