# Task 097 — production lossless Ada Float64 OMS JSON codecs

This is the production record for Ada OMS JSON Float64 support. It covers the
supported profile, guarantees, composition, limits, compatibility and the final
qualification results. Historical research and checkpoint records are kept
unchanged in the other `docs/task-097-*.md` files; see
[Historical evidence](#historical-evidence-c1c7).

For the hosted Git ownership correction and ordinary PY-001 main reconciliation,
see the [Task 097-CI-R1 corrective appendix](task-097-ci-r1.md).

## 1. Scope decision

> **Deliberate scope decision (Task 097-F64).** The release qualification scope
> is **Float64 correctness and ordinary valid/invalid-input behavior** using
> the **existing Ada model representation**. It is *not* recovery after memory
> exhaustion. The earlier C1–C7 research tried to establish a recoverable
> resource contract (`docs/task-097-c1-resource-contract.md`) for every public
> operation, including model construction under injected allocation failure.
> That contract was **not met** and is **not claimed** here. Its findings stay
> valid. An owned/immutable model representation ("Owned_V1", C7) is
> **deferred** as an optional, separately qualified hardening project. It is
> not a prerequisite for, or part of, this feature.

### Guaranteed (qualified, Section 9)

* Correct IEEE-754 binary64 conversion from JSON numbers (correctly rounded,
  nearest/ties-to-even), including signed zero, subnormals, underflow to a
  correctly rounded signed zero, largest finite, and overflow rejection.
* Lossless serialization: every finite Float64 is emitted as a JSON **number**
  whose exact rational value rounds back to the original bits.
* Exact OMS special strings `"NaN"`, `"Infinity"`, `"-Infinity"`. Quoted finite
  numbers are rejected for Float64 fields.
* Schema and field constraints enforced through existing model constructors,
  under `Assertion_Policy` Normal, Check **and Ignore**.
* Correct generated model values for every supported composition shape.
* Correct valid/invalid document handling **when ordinary resources are
  available**: malformed or invalid documents raise `Codec_Error`. They do
  not raise `Program_Error`, crash, or modify the caller's prior model value.
* Explicit platform and floating-environment rejection before any numeric work.
* Deterministic output on the qualified compiler matrix (GNAT 14.2.0, 16.1.0).
* Unchanged existing Ada models/APIs, portable/Float32/Binary codecs, Rust and
  C++ output (Section 10).

### Not claimed

* Recovery from arbitrary heap exhaustion.
* Failure-atomic ordinary Ada assignment.
* Preservation of every existing controlled object after an allocation failure.
* Transactional construction of arbitrary Binary-containing models under
  injected allocation failure.
* Allocation-free operation.
* Real-time determinism.
* Safe continuation after every runtime or allocator failure.
* Any deployed-transport, provider, or CAL interoperability.

Applications that need recovery from resource exhaustion must **not** rely on
this model profile for that property. It has not been qualified for it. Use an
isolation/restart strategy (e.g. a supervised process per decoding domain) or a
separately qualified hardened representation.


## 2. Supported profile

```
ams-gra-codegen-oms service-generate ... --language ada --with-codec \
    --ada-codec-runtime linux-x86_64-glibc-2.41-lossless-v1
```

`service-check` accepts the same option. The option is valid only with
`--language ada --with-codec`. Any other combination is a usage error before
output is written.

| Profile (`--ada-codec-runtime`) | Float32 | Float64 |
|---|---|---|
| *(omitted)* portable | existing policy (rejected) | rejected |
| `linux-x86_64-glibc-2.41` (Task 095) | supported | rejected |
| `linux-x86_64-glibc-2.41-lossless-v1` | supported | **supported within the qualified scope** |

The profile names the **deployment runtime**, not the generation host.
Generating on Linux does not imply compatibility. For a selection that needs
no Float64, the lossless option uses exactly the existing Float32 or portable
rendering, so output is byte-identical (Section 10). Demand is computed from
emitted operations and effective Record fields, never from type-name text.
The CLI, `service-check`, `service-generate`, shared readiness
(`analyze_service_codec_with_runtime`) and the Ada backend all carry the
explicit `AdaCodecRuntime::LinuxX86_64Glibc241LosslessV1`. Collision checks
reserve `AMS_GRA_OMS_JSON_Float64`, `AMS_GRA_OMS_JSON_Emitter`,
`Scalar_Float64` and `Output` in addition to the Float32 reservations.

### Deployment predicate

Each generated `Encode_*`/`Decode_*` operation calls
`AMS_GRA_OMS_JSON_Float64.Require_Platform` **before** any JSON or numeric
work. Every C boundary entry repeats the check. Supported only when all hold:

* Linux, x86-64, glibc reporting exactly `2.41` (`gnu_get_libc_version`).
* binary64 `double`/`IEEE_Float_64` ABI: radix 2, 53-bit mantissa, exponent
  range −1021..1024, verified through an exact C/Ada exchange of
  `1.0000000000000002` and `-0.0`.
* x87 control word rounding = nearest **and** MXCSR rounding = nearest, checked
  independently. A mismatched pair is unsupported even when `fegetround`
  reports nearest.
* MXCSR DAZ (bit 6) and FTZ (bit 15) disabled.
* Not compiled with `__FAST_MATH__`.

Otherwise the operation raises `AMS_GRA_OMS_JSON_Float64.Unsupported_Platform`.
Output and the caller's model are unchanged. Production code never changes,
caches or normalizes the caller's rounding, MXCSR, exception masks or locale.
Sticky arithmetic status flags may change.

Measured qualified target: Debian 13 `libc6 2.41-12+deb13u4`, `libc.so.6`
SHA-256 `9792e3cbb541c8f44c7acf5f14f4022ea62998ecc787d326bed4d8b6547dfd92`.
The pinned hosted container is
`debian@sha256:bcf83fd8af414eda0c0a99d25a4e61dedd88e7aac7f7bc519d04555ff0c40ec5`.

### Compiler, provider and linker requirements

* Ada 2022; GNAT 14.2.0 (external Debian `gnat-14 14.2.0-19`) and GNAT 16.1.0
  (Alire `gnat_native`) are qualified, with GPRbuild 26.0.1 and Alire 2.1.1.
* GNATCOLL and GNATCOLL minimal **26.0.0**, unpatched.
* `with "ams_gra_oms_json_lossless";` in the consumer project. It imports the
  Float32 project, which imports the ordinary guard project. It compiles only
  `ada/oms-json-support/lossless/` (Ada plus one C11 unit) and links `-lm`.
* glibc C-locale support (`newlocale`/`uselocale`/`strtod_l`).

## 3. Numerical design

### Decoding (original lexeme → `strtod_l`)

```
original JSON bytes
  -> C4 Validate_Encoded_Document (UTF-8, escapes, surrogates, strict grammar,
     1 MiB / depth 128 / 100,000 value nodes)
  -> C5 AMS_GRA_OMS_JSON_Lossless_Ingress immutable SAX arena
     (original numeric span retained per node)
  -> Numeric_Lexeme (only for JSON number nodes)
  -> AMS_GRA_OMS_JSON_Float64.Number: exact JSON number syntax re-check
  -> C ams_gra_float64_parse: private C numeric locale, strtod_l,
     full consumption, finite result (ERANGE subnormal/zero accepted,
     infinite overflow rejected)
  -> IEEE_Float_64
```

There is no intermediate GNATCOLL numeric DOM, signed-64 storage, Float32,
Ada `'Value`, or textual normalization. Integer-form tokens beyond signed-64
(e.g. `9007199254740993`, `18446744073709551616`) are converted from their
original text. JSON strings never reach `Number`. They reach only `Special`,
which accepts exactly `"NaN"`, `"Infinity"` and `"-Infinity"`
(case-sensitive) through an explicit-status C boundary; no arithmetic produces
them. NaN payload and signaling bits are not preserved.

### Encoding (`%.17g` → typed structural emitter → C4 validation)

* `Output.Floating_64` calls C `ams_gra_float64_format`: `snprintf("%.17g")`
  under a private C numeric locale installed with `uselocale`, into a 32-byte
  local buffer. Length, capacity, termination and embedded NUL are checked
  before publication.
* The bounded typed `AMS_GRA_OMS_JSON_Emitter` places the token in its
  structural position and never rewrites it. Finite values are numbers.
  Specials are the exact strings. Signed zero is `-0`.
* Objects use effective schema order (inherited fields first). A Choice emits
  exactly its selected alternative. Arrays keep element order. Strings use the
  existing guard escaping after UTF-8 validation. Float32 leaves use the
  existing Task 095 formatter, so the two widths are never confused.
* `AMS_GRA_OMS_JSON_Guard.Validate_Encoded_Document` (C4) validates the
  completed text before it is returned. No GNATCOLL DOM, `Occurrences.Read`,
  or controlled-array traversal is used for output validation.
* Sequence and constrained-Binary model reads use the C3 `<Model>.Codec_Views`
  child package (synchronous read-only callbacks). The parent model package is
  unchanged.

### Locale

The C boundary never calls process-global `setlocale`. Formatting restores the
**exact previous** thread locale handle, including `LC_GLOBAL_LOCALE`. If that
restoration fails, the process aborts by design (`abort`), because returning
could leave a private locale installed. Parsing passes the locale explicitly
to `strtod_l` and installs nothing. Locale allocation, installation and
formatting failures return a distinct `Resource_Error` status with no output.

## 4. JSON ingress semantics and limits

Unchanged from C5 (`docs/task-097-c5-resource-safe-ingress.md`):

| Limit | Value |
|---|---|
| Maximum document | 1,048,576 bytes |
| Maximum depth | 128 simultaneous collections |
| Maximum value nodes | 100,000 (keys and end events are not nodes) |

* Strict RFC 8259 grammar and complete consumption. UTF-8 and surrogates are
  validated before any string is decoded.
* Field names compare **decoded** keys, so `"\u0052ecord"` equals `"Record"`.
* Duplicate members: the **last complete subtree wins**, including its original
  numeric spans. Every occurrence still counts against the node limit and must
  be grammatically valid, so an invalid earlier duplicate rejects the document.
* Unknown members are rejected at every Record and Choice level.
* Optional members: absent → `Is_Present = False`. Empty `minOccurs=0`
  repeated fields are omitted on output (established repeated-codec convention).
* Legacy `Guard.Read/Write` and the Float32 occurrence reader are unchanged.
  Only codecs that emit Float64 under the lossless profile use the new ingress.


## 5. Supported composition

Qualified in production-generated codecs (Section 9):

* direct `xs:double` fields; named unconstrained and named range-constrained
  Float64 (`min/maxInclusive`, `min/maxExclusive`) through the existing
  `Model.Create`/`Model.Value` checked carriers;
* required and optional Float64; bounded and unbounded repeated Float64,
  including named constrained elements;
* nested Records, inherited (extension) fields, named Choices selecting
  Records, repeated Records and repeated Choices;
* mixed Float32/Float64 in the same Record;
* Float64 with unconstrained Binary, constrained named Binary
  (`length`/`minLength`/`maxLength`) and optional/repeated named Binary siblings;
* supported `xs:dateTime` / `xs:duration` siblings, and strings, enumerations,
  integers and booleans already admitted by the existing analysis.

Invalid values keep the established checked-constraint behavior. A constructor
`Assertion_Error` becomes `Codec_Error` ("invalid checked Float64"). Facet
enforcement does **not** depend on the consumer's `Assertion_Policy`: the
generated model package carries
`pragma Assertion_Policy (Dynamic_Predicate => Check)`, qualified under Ignore.

### Unsupported (atomic rejection before output)

* `xs:decimal`;
* Float64 (or other scalar) **Choice alternatives**;
* field-local floating facets (facets must be on a named type);
* unsupported Binary representations, including unconstrained named Binary as a
  repeated element;
* unsupported temporal forms (e.g. UCI `TimeType`);
* abstract/open-world model shapes not admitted by existing analysis.

The feature was not widened to make more UCI services READY.

## 6. Existing Ada model compatibility

* Model packages (`programs*.ads/adb` or the namespace equivalent) and
  `service_api.ads` are **byte-identical** with and without the lossless codec,
  and against exact merged main (Section 10). No `Owned_V1`, owned cells,
  shared immutable storage, model transactions, versioned packages or new
  Binary storage were introduced.
* The lossless profile adds `service_codec.ads/adb`. When sequence or
  constrained-Binary reads are needed, it also adds the separate child package
  `<Model>.Codec_Views` (C3).
* No ABI compatibility is claimed with any representation that was never
  adopted. C7's scratch transformation is not installable.

## 7. Operational resource limitations

These are documented limitations of the **existing** model profile. They are
not corrected and must not be described as corrected:

1. **Controlled-copy Storage_Error can become Program_Error.** If an allocation
   fails during a compiler-generated `Adjust` of a Binary-containing
   Record/Choice/sequence (for example `Binary_Vectors.Adjust` inside a
   value-returning model function or aggregate), Ada finalization semantics turn
   it into `Program_Error` ("finalize/adjust raised exception"). This happens
   inside model construction and copying, which also occur inside
   `Encode_*`/`Decode_*`. It is **not** relabeled as `Codec_Error` or
   `Storage_Error` (C1, C3, C5, C6).
2. **Failed assignment can modify the target.** A caller assignment
   `Item := Decode_*(...)` whose adjustment fails after allocation may leave
   `Item` changed (C6 `target_changed`).
3. **Provider transitions.** Unpatched GNATCOLL `XString` small→big conversion
   under allocation failure can finalize an inconsistent string (C1, 28-byte
   witness). The lossless ingress does not use `XString` (C5). The legacy guard
   and Float32 occurrence paths still do.
4. Generated `Decode_*` holds the full document plus a bounded arena: at most
   `min(100,000, length)` nodes plus document-length byte buffers.

None of these is reachable from malformed or invalid input when ordinary
resources are available. The ordinary-input gates (Section 9) found no
`Program_Error`, crash or caller-model change for any malformed, invalid,
over-limit or unsupported-environment input.


## 8. Reproducible generation and build

```sh
cargo +1.95.0 build --locked --release -p ams-gra-codegen-oms
target/release/ams-gra-codegen-oms service-generate \
  --schema tests/fixtures/service-generate/codec-ada-float64.xsd \
  --contract tests/fixtures/service-generate/codec-ada-float64.yaml \
  --language ada --world closed-schema --with-codec \
  --ada-codec-runtime linux-x86_64-glibc-2.41-lossless-v1 --output out
# out/consumer.gpr:  with "<repo>/ada/oms-json-support/ams_gra_oms_json_lossless";
gprbuild -P out/consumer.gpr -p
```

Full local qualification for one compiler, with a fresh task-owned Alire home:

```sh
python3 scripts/task097/job.py "$EVIDENCE/qualify16-job" -- \
  python3 scripts/task097/qualify.py --scratch "$EVIDENCE/qualify16" \
  --compiler 16 --generator "$PWD/target/release/ams-gra-codegen-oms"
```

`qualify.py` runs these steps in order:

1. runtime identity, oracle unit tests, frozen corpus regeneration and hash
   check, and an independent exact-rational review of every conversion row;
2. GNAT 14 only: the production C boundary corpus and lifecycle;
3. fresh provisioning and the small checkpoint;
4. the **representative composition gate** (`c3_matrix.py`: both namespaces ×
   Normal/Check/Ignore × O0/O2);
5. per namespace: the full generated numerical corpus (`generated.py`), the
   environment matrix (`environment.py`) and conformance (`conformance.py`);
6. the O2 numerical corpus and O2 conformance.

Real-UCI execution (`real.py`, with the `uci.py` inventory) and preservation
(`preservation.py`) are separate runners. If `/tmp` is full, point `TMPDIR` at
a writable task-owned directory first; GPRbuild needs it for temporary
configuration projects.


## 9. Final qualification results

Qualified implementation source: `9828c33566c91598cadeaf03aebbbdda208d52e7`
(immutable `git archive` snapshot, own release generator). Every value below
comes from an atomic `result.json` or per-step `.exit` record; a step counts
only with exit 0 and its expected counts.

### Runtime and toolchains

| Item | Identity |
|---|---|
| libc | Debian `libc6 2.41-12+deb13u4`; `libc.so.6` `9792e3cb…6547dfd92` |
| GNAT 14 | external Debian `gnat-14 14.2.0-19` / `gcc-14 14.2.0-19` (`GNAT 14.2.0`); `/usr/bin/gnat` SHA-256 `67082a9217f83ff443253d5d02973615b3f2cc01d47e527d857d87229f7c2c4f` |
| GNAT 16 | Alire `gnat_native 16.1.0` (`GNAT 16.1.0`); `gnat` SHA-256 `fa15577ec622065f1e80d18f14d315d8b3d9a6ff17cf195ffe4fe3895c1f84d9` |
| GPRbuild / Alire | `gprbuild 26.0.1`; `alr 2.1.1`; fresh task-owned Alire home per compiler |
| Provider | GNATCOLL / GNATCOLL minimal 26.0.0, unpatched |
| Rust (gates) | `cargo +1.95.0` |

### `qualify.py` (per compiler)

GNAT 14 ran 17 steps and GNAT 16 ran 16 steps; all exited 0
(`qualify14-01`, `qualify16-01`). The production C boundary step runs only
under GNAT 14.

| Step | GNAT 14.2.0 | GNAT 16.1.0 |
|---|---|---|
| Runtime identity (glibc 2.41, x86-64) | pass | pass |
| Oracle unit tests; frozen corpus regeneration + hash check | pass | pass |
| Independent exact-rational review of every conversion row | pass | pass |
| Production C boundary: 2 locales (C, `de_DE.UTF-8`) × convert/encode × O0/O2; 128 environment processes | 8/8 cells, 27,167 / 20,474 rows each | — |
| Fresh provisioning + small checkpoint (3 record positions, 6 negative cases) | pass | pass |
| Composition gate, `oam` namespace (Normal/Check/Ignore × O0/O2; 16 checks per cell) | 98 rows, 0 unexpected | 98 rows, 0 unexpected |
| Composition gate, qualified namespace | 98 rows, 0 unexpected | 98 rows, 0 unexpected |
| Generated numerical corpus, `oam`, O0, 3 policies | 27,167 conversions + 20,474 encodings per policy | same |
| Generated numerical corpus, qualified namespace, O0, 3 policies | same | same |
| Generated numerical corpus, `oam`, **O2**, 3 policies | same | same |
| Floating environment, `oam` (4 x87 × 4 MXCSR × 4 DAZ/FTZ × 3 policies) | 192/192 | 192/192 |
| Floating environment, qualified namespace | 192/192 | 192/192 |
| Conformance `oam` O0 / qualified O0 / `oam` O2 (3 policies each) | 8 accepted, 23 rejected per cell; German thread locale | same |

All numerical rows go through **production-generated** `Encode_*`/`Decode_*`
operations on generated models. Each numeric result is compared with the
frozen Task096 oracle for exact binary64 bits, and must be a JSON number (not
a string) for finite values. Specials must be the exact strings `"NaN"`,
`"Infinity"` and `"-Infinity"`. The environment matrix requires success only
for (nearest, nearest, no DAZ/FTZ). For every other combination it requires
`Unsupported_Platform` on both encode and decode. In every cell it checks that
the x87 control word and the MXCSR control bits are unchanged by the codec;
sticky status flags (MXCSR bits 0–5) are excluded, as documented in Section 2.
The harness then restores the original environment exactly. Generated Ada sources
are byte-identical between the two compiler campaigns, per namespace.

### Rust workspace and CI-integrity gates

Final attempt `local-gates-02` on the final source snapshot. All steps exited 0: `format`, `check`, `clippy`, `test`, `ci-split`, `ci-split-mutations`, `ci-layout`, `ci-layout-tests`, `oracle-tests`.
Workspace tests: 1435 passed, 0 failed, 8 ignored across 163 test-binary summaries.

Earlier attempts are kept as evidence: `local-gates-01`: format=0, check=0, clippy=0, test=101.

**Historical intermittent observation.** In `local-gates-01`, `task070_terminal_refusal_preserves_error_and_joins_worker` (`crates/runtime-rust-facade-tests/tests/runtime_initial_retry.rs`) failed once: `runtime worker outlived joined connect/close` (named worker threads 1, expected 0). At the time, the machine was heavily loaded by the Ada qualification campaigns and unrelated builds. Task 097 changes no runtime crate. The same test binary then passed six isolated reruns, and the fresh full workspace run `local-gates-02` passed. The root cause is **not established**, and load is not claimed as the cause. The test was neither modified, skipped nor relaxed.

## 10. Preservation

`preservation.py` runs the frozen baseline generator against the
final-source generator over every `tests/fixtures/service-generate` fixture
and language, plus the Task 095 Float32 runtime. With `--include-lossless`,
it also covers the new profile on every Float64-free Ada codec fixture.

| Baseline | Legacy byte-identical | Legacy matching atomic rejections | Lossless profile byte-identical (Float64-free) |
|---|---|---|---|
| merged main `5f6800d505c6501638c322d20b23cfc36b09d7d6` | 86 | 19 | not applicable (main has no lossless option) |
| C5 `9f0c7417b002909819353b2c9562d0f3758fdd33` | 86 | 19 | 15 / 15 |

The 86 legacy comparisons are 30 Rust, 15 C++, 26 portable Ada and 15 Task 095
Float32 Ada outputs. The 19 matching atomic rejections are 15 C++ and 4 Ada.
Rust and C++ output is unchanged.

The Float64 fixtures (`codec-ada-float64`, `codec-ada-copy-views`) were also
regenerated by main, C5 and the qualified source. Without the runtime option,
the output is identical in all three. With the lossless option, C5 and the
qualified source match byte for byte, and main rejects the option atomically
before writing output, as expected.

## 11. Real-UCI coverage

The UCI 2.5 and 2.6 roots are pinned by SHA-256
(`ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27`,
`af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b`).
The `uci.py` inventory finds 40 named Float64 declarations per release, used
by 1,393 of 1,395 fields and 393 of 395 static messages. These match the
historical baseline inventory byte for byte. `real.py` selects every
inventoried message that is READY under the lossless profile but **not** under
the Float32 profile, i.e. services this feature newly makes generatable:

| Release | Message | Payload | Type closure | Lossless renderable/emitted | Baseline first blocker | Valid+boundary accepted | Invalid rejected | GNAT 14 cells | GNAT 16 cells |
|---|---|---|---|---|---|---|---|---|---|
| 2.5 | EarthOrientationParameters | `EarthOrientationParametersMT` | 44 | 43/43 | `ArcSecondsType: unsupported Ada codec primitive or facets: Float64` | 1 | 5 | 3/3 | 3/3 |
| 2.5 | EntityConfirmation | `EntityConfirmationMT` | 42 | 41/41 | `DoubleNonNegativeType: unsupported Ada codec primitive or facets: Float64` | 2 | 5 | 3/3 | 3/3 |
| 2.5 | EntityLost | `EntityLostMT` | 40 | 39/39 | `EntityLostMDT.Probability: unsupported Ada codec primitive or facets: Float64` | 3 | 7 | 3/3 | 3/3 |
| 2.5 | LAR_Report | `LAR_ReportMT` | 46 | 45/45 | `AltitudeType: unsupported Ada codec primitive or facets: Float64` | 2 | 5 | 3/3 | 3/3 |
| 2.5 | SpectralDensityReport | `SpectralDensityReportMT` | 45 | 44/44 | `DoublePositiveType: unsupported Ada codec primitive or facets: Float64` | 1 | 5 | 3/3 | 3/3 |
| 2.5 | SubsystemCalibrationStatus | `SubsystemCalibrationStatusMT` | 45 | 44/44 | `DoubleNonNegativeType: unsupported Ada codec primitive or facets: Float64` | 2 | 4 | 3/3 | 3/3 |
| 2.6 | EarthOrientationParameters | `EarthOrientationParametersMT` | 44 | 43/43 | `ArcSecondsType: unsupported Ada codec primitive or facets: Float64` | 1 | 5 | 3/3 | 3/3 |
| 2.6 | EntityConfirmation | `EntityConfirmationMT` | 42 | 41/41 | `DoubleNonNegativeType: unsupported Ada codec primitive or facets: Float64` | 2 | 5 | 3/3 | 3/3 |
| 2.6 | EntityLost | `EntityLostMT` | 40 | 39/39 | `EntityLostMDT.Probability: unsupported Ada codec primitive or facets: Float64` | 3 | 7 | 3/3 | 3/3 |
| 2.6 | LAR_Report | `LAR_ReportMT` | 46 | 45/45 | `AltitudeType: unsupported Ada codec primitive or facets: Float64` | 2 | 5 | 3/3 | 3/3 |
| 2.6 | RF_Profile | `RF_ProfileMT` | 46 | 45/45 | `DoubleNonNegativeType: unsupported Ada codec primitive or facets: Float64` | 2 | 5 | 3/3 | 3/3 |
| 2.6 | SubsystemCalibrationStatus | `SubsystemCalibrationStatusMT` | 45 | 44/44 | `DoubleNonNegativeType: unsupported Ada codec primitive or facets: Float64` | 2 | 4 | 3/3 | 3/3 |

12 services × 3 policies × 2 compilers = 72 executed cells (`real14-01`: 36, `real16-01`: 36).

For each selected service, the baseline generator rejects both the portable
and Task 095 Float32 profiles atomically in `service-check` and
`service-generate`, with no output. The first blocker is the Float64 primitive.
The final generator then produces the closure with the lossless profile, and
each consumer is built and run under GNAT 14.2.0 and 16.1.0 with
Normal/Check/Ignore. Valid and boundary documents must round-trip with JSON
number kind and exact binary64 bits. The real-schema boundary values are `-0`
(EntityConfirmation), `5e-324` (SubsystemCalibrationStatus, RF_Profile), the
largest finite value (LAR_Report), and `"Infinity"`/`"-Infinity"`
(EntityLost). `"NaN"` and the other specials are covered by the conformance
corpus (Section 9). Invalid documents, including independently written Float64
value violations, malformed truncations and wrong kinds, must be rejected
atomically. All other field types in these closures, including temporal and
Binary fields, go through the existing unchanged codecs (Section 6), and the
same valid documents exercise them.

**Not READY and not claimed:**

* **DLZ** (2.5, 2.6). Float64 is no longer the blocker. Under the lossless
  profile, the first remaining blocker is
  `TimeType: unsupported Ada codec primitive or facets: Time`, unchanged.
* **ScanDataBaseType** is a synthetic owner-only projection used for
  inventory. It is not a message or service selection and makes no readiness
  claim.
* Log, RDMA_InitializeSetup and SurvivabilityRiskLevel were already READY
  without Float64, so they are covered by preservation, not by this matrix.

A selected service being READY means only that its closure can be generated
and that it passed the documents above. No provider, transport or deployed
interoperability claim is made.

## 12. Evidence identities

| Identity | Value |
|---|---|
| Qualified implementation commit (Ada/UCI campaigns) | `9828c33566c91598cadeaf03aebbbdda208d52e7` |
| Final source commit (Rust/CI gates) | `c3c7d56ea383f33f6fdaa2b9b883f443d25a9276` |
| Qualified source manifest (`src-01/source.sha256`) | `bc3d9224856d72641eed2ceaebb7d2d7a2dda946a4f66c0a13a600d17682f8a7` |
| Final source manifest (`src-03/source.sha256`) | `2767810150f2df3fd1b3bf4479809a012715c9b36b8ff19d5212a3f6fc8f3e29` |
| Files differing between the two manifests | `.github/workflows/ci.yml`, `ada/oms-json-support/README.md`, `docs/task-097-ada-float64-codec.md`, `docs/task-097-ada-float64-production.md` (docs/CI only) |
| Release generator (qualified = final, byte-identical) | `eeca5d517630c98949b66c445578f38b551198fa27abc16a754eadb7499358c4` |
| Baseline generator (merged main) | `ecfebaa2f6fdfba6aaa746c398b8d758eaa9f93fc717d07f04088b74ffd952f1` |
| Frozen corpus `conversions.tsv` (identical under both compilers) | `14c86c5fa674d279d198947dad03f71f517a40b583729df1184bbf6596a6451b` |
| Frozen corpus `encoding.tsv` (identical under both compilers) | `b1ad323c7d5d88c465bf966dc2edc1249c68982ad817044fa5579ce65e9abc82` |
| Frozen corpus `manifest.json` (identical under both compilers) | `e33d3ef9e51635bb0ece91d27a72f4809b49b279586e4693377759473b04d4b3` |
| Generated `oam` `programs-oam-codec_views.adb` (identical under both compilers) | `5795a99819b078637e35f25c29bde983fa048d7c5c609b39e239a5f527a1b7bc` |
| Generated `oam` `programs-oam-codec_views.ads` (identical under both compilers) | `fbb3e390ea9c2ecdbb860311433badce789ea78280f2be6fd70aa7cb20db2405` |
| Generated `oam` `programs-oam.adb` (identical under both compilers) | `cb5bf33002369098e0658b4e34d52645eadab568dc15651dfd130b80a526ce48` |
| Generated `oam` `programs-oam.ads` (identical under both compilers) | `482c1f0dba400c8037a6372058b1afb19d9d8c490b074f6c47c8e4360c711876` |
| Generated `oam` `programs.ads` (identical under both compilers) | `a5d92d26a18c03999588818e68c5a8b3426ff1c780a7582bf21121727b4a0d16` |
| Generated `oam` `service_api.ads` (identical under both compilers) | `1ebbfed03e87015b33c128222bd67c0c10ad4cec906522b4e79d738853c19ad7` |
| Generated `oam` `service_codec.adb` (identical under both compilers) | `46427ab09f02499af3ee95ff1344f7054014b661e3c9c4a9a0077bcd044adfe3` |
| Generated `oam` `service_codec.ads` (identical under both compilers) | `9089d3c1edeee524e7b77f1c00e7173cac43555dd2cbf539acb4ba5e35bea71a` |
| Generated qualified-namespace `service_api.ads` (identical under both compilers) | `6df296915f9f9465f9b9c4cb6ba3ec2afdd16a8756e4f40cf31cfc77dfe7b377` |
| Generated qualified-namespace `service_codec.adb` (identical under both compilers) | `c637a75c863e1df661bd49114639c2c6827e49898fe4febbdb1dc121ca640f0e` |
| Generated qualified-namespace `service_codec.ads` (identical under both compilers) | `af5f3e46a48f6b349daf3898eda8a063d47bd7ebfb72f479ec7fa97a3bc3c1c8` |
| Generated qualified-namespace `urn-task097-codec_views.adb` (identical under both compilers) | `748b296aadfe4d6a25662acae21d820d5d5eb9891a42f0145d15d1dccc550ee3` |
| Generated qualified-namespace `urn-task097-codec_views.ads` (identical under both compilers) | `1cbf416fc493589954b560233e4e21431b5d4209d374f046f7df91e0d90d1b3c` |
| Generated qualified-namespace `urn-task097.adb` (identical under both compilers) | `f178974bd65eea450e7bb563700823b050625a6bb36f07960cd0b8a4dba3aba6` |
| Generated qualified-namespace `urn-task097.ads` (identical under both compilers) | `bd6844b0c26a59f460d68205cb97d5601b6d00fc597a7e53d1ce8f299b79a352` |
| Generated qualified-namespace `urn.ads` (identical under both compilers) | `a570571230ee7f3b7e29252e195d265275f36a23f265eb374ace3eca72a8bfb5` |

The complete machine-generated ledger (`ledger-01.json`) and every per-step command, stdout, stderr and exit record are kept in the task evidence directory (`/home/zboll/task097-scratch/float64-production/`). Failed or superseded attempts (`local-gates-01`, snapshot `src-02` after `/tmp` inode exhaustion) are preserved, not deleted.

## Historical evidence (C1–C7)

These records are preserved without rewriting, and their dispositions stand:

| Record | Disposition (unchanged) | Retained finding |
|---|---|---|
| `task-096-ada-float64-boundary.md` | research boundary qualified | frozen oracle/corpora; known-lossy candidates documented, not rerun |
| `task-097-ada-float64-codec.md` | QUALIFICATION_FAILED (checkpoint) | decode budget 33 `Program_Error` from provider XString transition |
| `task-097-c1-resource-contract.md` | required contract, **not met** | recoverable-resource contract definition |
| `task-097-c2-copy-safe-traversal.md` | ADDITIONAL_COPY_BOUNDARIES_FOUND | callback visitor avoids sequence element copy |
| `task-097-c3-copy-safe-encoder.md` | ADDITIONAL_ENCODER_COPY_BOUNDARIES | production `Codec_Views`; Binary/sequence read copies removed |
| `task-097-c4-streaming-output-validation.md` | OUTPUT_VALIDATION_BOUNDARY_CLOSED_LOCAL | DOM-free final output validation |
| `task-097-c5-resource-safe-ingress.md` | INGRESS_RESOURCE_BOUNDARY_CLOSED_LOCAL | immutable SAX arena; downstream model copies remain red |
| C6 (evidence only, `/home/zboll/task097-scratch/c6/`) | EXECUTION_INTERRUPTED | 834 downstream `Program_Error` / 728 `Storage_Error` injections; caller target can change |
| C7 (evidence only, `/home/zboll/task097-scratch/c7/`) | ADDITIONAL_OWNERSHIP_BOUNDARIES | scratch Owned_V1 transformation; not integrated |

The earlier QUALIFICATION_FAILED disposition was a failure of the *resource
recovery* contract. This production record changes the release scope
(Section 1). It does not reinterpret those results as passes. The historical
`conformance.py` allocation-budget mode is removed from the production gate, so
it no longer asserts out-of-scope recovery. Its failing run
(`conformance16-oam-03/Normal-allocation`) remains historical evidence. The
72,486-failure Owned_V1 campaign was not run. The C3 composition consumer keeps
one first-allocation `Storage_Error` probe on the copy-safe encoder path. That
probe is a focused regression of the C3 correction, not a recovery guarantee.

