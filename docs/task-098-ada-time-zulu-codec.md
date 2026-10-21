# Task 098 — Ada named TimeZulu OMS JSON codec

## Scope and history

Task 061 already supplied the checked Ada **model** carrier for one named
`xs:time` profile (`TemporalProfile::TimeZulu`):

```ada
function Create (Value : String) return Clock;   --  raises Constraint_Error
function Value (Item : Clock) return String;
```

Until now, the Ada **service codec** rejected that carrier with
`unsupported Ada codec primitive or facets: Time`. After Task 097 (PR #102),
that was the last blocker for the real UCI DLZ service. Task 098 adds the
missing Ada OMS JSON codec for exactly that profile and nothing else. It is
a stacked feature on PR #102.

| Identity | Value |
|---|---|
| Stacked parent (PR #102, `feature/097-ada-float64-codec`) | `96489aa95f21f86706e227443a8bfd29b81c1d41` |
| Branch / worktree | `feature/098-ada-time-zulu-codec` / `/home/zboll/git/ams-gra-codegen-oms-task098` |
| Evidence root | `/home/zboll/task098-scratch/` |

Task 098 does not change the Ada model or service API representation,
ownership, Float64 behavior, or general XML Schema Time handling. It adds no
Time parser, clock, calendar, time arithmetic or `Duration` conversion.
**This is not general-purpose `xs:time` support.**

## Supported profile

A named declaration is admitted only if the shared semantic classifier
`codegen_core::temporal_profile(PrimitiveKind::Time, constraints)` returns
`Ok(Some(TemporalProfile::TimeZulu))`, which is the same call the Ada model
generator uses. The effective restriction must be exactly one XML Schema
pattern group whose single alternative is `.+Z`, with no other effective
facet.

Admission never depends on the type being named `TimeType`. The synthetic
fixtures name the type `Clock`, and a unit test admits the profile under both
names and rejects unconstrained Time under both names.

These constructs are still rejected, fail-closed, with attributed
diagnostics:

| Construct | Rejected by | Diagnostic |
|---|---|---|
| Direct `xs:time` member or Choice alternative | model (Task 061) and codec | `unsupported Ada codec primitive or facets: Time` |
| Named Time with no pattern, a different pattern, two alternatives, two groups, explicit whitespace, or a bound | classifier | `<Decl>: unsupported temporal declaration: Time` |
| Length facet on `xs:time` | XSD ingress | `unsupported XSD construct: xs:maxLength` |
| Field-local facets on a named Time reference | codec | `<Owner>.<Field>: field-local named facets` |
| Any DateTime, Duration, Decimal, Binary or numeric profile that was not admitted before | unchanged | unchanged |

## OMS JSON mapping

A TimeZulu value is a JSON **string** containing the validated lexical
spelling stored by the model.

* **Encode.** The codec emits `<String output> (Model.Value (Item))`, using
  the current profile's existing string operation: `Create` for portable and
  Float32, `Output.Create` for lossless. The value is not canonicalized:
  `24:00:00.000Z`, `23:59:60Z` and 50-digit fractions are emitted exactly as
  stored.
* **Decode.** `Scalar_String (Value)` requires `JSON_String_Type`, then the
  model constructor `Model.Create (Text)` validates the text. Only the
  constructor call is inside the handler, which maps `Constraint_Error` to
  `Codec_Error "invalid named Time"`. A wrong JSON kind raises
  `wrong JSON kind` before that handler is entered. No other exception is
  caught.
* **XML whitespace** is collapsed by the model's own `Create`, not by the
  codec. For example, `" 24:00:00Z\t"` decodes and re-encodes as
  `"24:00:00Z"`. Embedded whitespace is rejected.

The model constructor is the only lexical validator. The codec has no second
Time check that could disagree with it.

## Source integration

| File | Change |
|---|---|
| `crates/codegen-core/src/service_codec.rs` | `ada_scalar_support` gains a `PrimitiveKind::Time if named` arm that calls `temporal_profile` and admits only `TimeZulu`. Direct Time still falls through to the generic rejection. Adds two focused unit tests. One Task 090 boundary assertion now expects exactly the `Time && named` case to be admitted. |
| `crates/backend-ada/src/service_codec.rs` | The named-primitive encoder joins the existing `Duration \| DateTime` checked-carrier arm. A new named Time decoder arm calls `Scalar_String` and then `Model.Create`. |

No new helper name or fixed reservation is introduced. The only new generated
subprograms are the per-declaration `To_JSON_<Name>` / `From_JSON_<Name>`
pair, which every named emission already reserves. Every way a Time can
appear uses that same pair:

* optional members and nested Records
* bounded, unbounded and positive-minimum repeated members
* repeated Records
* Choice alternatives and repeated Choices

Admission propagates through the memoized `AdaCodecAnalysis::support` /
`record_support` closure. That closure covers required and optional members,
named Choice alternatives, repeated named elements and selected-message
closures. An unsupported Time is attributed to its declaration (`Clock: …`).
When it is reached through a Choice, the diagnostic also names the
alternative (`Pick.Time: Clock: …`).

## Generated-code qualification (`scripts/task098/qualify.py`)

All three fixtures are new. In each, `Clock` is the exact `.+Z` restriction.

* `codec-ada-time-zulu`: Time only. Time appears as a required member, an
  optional member, a Choice alternative, a nested Record (required and
  optional), a bounded sequence (0..3), an unbounded sequence (0..∞), a
  positive-minimum unbounded sequence (2..∞), a repeated nested Record and a
  repeated Choice.
* `codec-ada-time-zulu-float32`: the same, plus `xs:float` (Task 095
  runtime).
* `codec-ada-time-zulu-mixed`: the same, plus named and direct DateTime,
  named and direct Duration, hexBinary, Float32, Float64 and checked Float64
  (lossless runtime).

Each cell:

1. checks that the codec is READY;
2. generates atomically;
3. proves that `--with-codec` leaves the model/API bytes identical;
4. compiles the actual generated model, API and codec together with
   `tests/ada-time-zulu-codec/time_consumer.adb`;
5. checks **every** output line against `scripts/task098/cases.py`.

The consumer decodes, encodes, re-decodes and re-encodes every document and
requires the two encodings to be identical. Only `Codec_Error` counts as a
rejection.

Expected results come only from hand-authored documents and the frozen
Task 061 corpus `tests/fixtures/temporal/time-zulu.txt` (SHA-256
`6367d1e786939fc9e2e136a92f67c18495aea03c88fd770c21e366865d83bdee`;
56 cases, 13 valid and 43 invalid, verified on load). Each cell runs 156
documents:

* **2 valid baselines:** one fully populated, one minimum.
* **112 corpus documents:** all 56 corpus cases at the required member
  **and** at the Choice alternative. The corpus covers:
  * ordinary times, fractions and 50-digit fractions;
  * `24:00:00Z` with zero fractions, and seconds = 60;
  * invalid hour, minute and second values;
  * offsets `+00:00`, `-00:00` and `+05:30`;
  * missing `Z`, lower-case `z` and `ZZ`;
  * NBSP, EM SPACE, NUL and DEL characters;
  * Arabic-Indic and full-width digit lookalikes;
  * XML whitespace collapse versus embedded whitespace.
* **40 composition documents:** for each of the 10 composition positions, a
  whitespace-collapse boundary, a non-Z offset, a JSON number and JSON
  `null`.
* **2 structural documents:** malformed (truncated) JSON and a non-object
  document.

An accepted output must match its expectation exactly. Time values must be
plain JSON strings, and numbers must be JSON numbers with identical binary64
bits.

| Compiler | Namespaces | Profiles | Policies | Cells | Per cell |
|---|---|---|---|---|---|
| GNAT 14.2.0 (external) | OAM; qualified `urn:task098` | portable; Float32; lossless (mixed) | Normal; Check; Ignore | 18 | 156 documents: 38 accepted, 118 rejected |
| GNAT 16.1.0 (Alire native) | same | same | same | 18 | same |

Provisioning is fresh and task-owned (`scripts/task096/setup.py`): Alire 2.1.1,
GPRBUILD 26.0.1 and GNATCOLL 26.0.0 (`gnatcoll-json.adb` SHA-256
`7e07277731a44ea08716fd61a6462ebe7a7ce0722adc6dc787cbbaafd3099063`). Each
worker builds against its own private copy of the provider tree. The
qualifying evidence is in `qualify14-02/` and `qualify16-02/` (`result.json`,
plus every command, stream and exit file). `qualify14-01/` also completed all
18 cells; `qualify16-01/` is the preserved environmental failure described
under *Tests, gates and hosted CI*.

The production generator used for every campaign has SHA-256
`10a82d76a0ef03b35b227924f1596c6366081f392a3f54e18131e97a71839fc3`. The
parent build has SHA-256
`eeca5d517630c98949b66c445578f38b551198fa27abc16a754eadb7499358c4`, identical
to Task 097's recorded release generator.

## Real-UCI vertical: DLZ (`scripts/task098/real.py`)

The vertical uses the pinned, unchanged UCI roots:

* UCI 2.5: `ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27`
* UCI 2.6: `af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b`

The contract selects the real `DLZ` message (`DLZ_MT`), whose selected type
closure has 54 types and 53 emitted codec declarations. The table compares
the actual Task 097 parent build against the Task 098 build. Each cell gives
the renderable count and, where not READY, the first blocker.

| Release | Generator | Portable | Float32 | Lossless |
|---|---|---|---|---|
| 2.5 | Task 097 parent | 46/53; `AnglePositiveType`: Float64 | 47/53; `AnglePositiveType`: Float64 | 52/53; **`TimeType`: Time** |
| 2.5 | Task 098 | 47/53; `AnglePositiveType`: Float64 | 48/53; `AnglePositiveType`: Float64 | **53/53 READY** |
| 2.6 | Task 097 parent | 46/53; `AnglePositiveType`: Float64 | 47/53; `AnglePositiveType`: Float64 | 52/53; **`TimeType`: Time** |
| 2.6 | Task 098 | 47/53; `AnglePositiveType`: Float64 | 48/53; `AnglePositiveType`: Float64 | **53/53 READY** |

* For every NOT READY cell, `service-generate` also failed atomically with no
  output.
* Under the lossless Float64 runtime, named Time support makes the whole
  selected closure READY in both releases. No other blocker emerged.
* Portable and Float32 remain blocked by Float64. That is unchanged and out
  of scope.

For each READY release, the Task 098 model/API files are byte-identical to
the parent's model-only output. The generated model, service API and codec
were compiled under both compilers and all three policies. Both compilers
received identical generated sources: for example, the `service_codec.adb`
SHA-256 prefix is `11395b658e9e` for 2.5 and `bb9a2967672b` for 2.6. Each
build ran 26 independently authored documents.

**10 accepted documents:**

* One full document. It uses:
  * the real required Time members `TimeOfIntercept` and `TimeToRelease`;
  * the three optional `TimeOf*` members;
  * Float64 Distance, Speed and Angle values;
  * the Float32 unit-vector components;
  * the real, required, repeated `OwnerProducerChoiceType` Choice, with both
    alternatives.
* Three documents, each omitting one optional Time member.
* Six boundary documents:
  * a whitespace-collapsed `24:00:00.000…Z` with a 50-digit fraction;
  * `23:59:60.999999Z`;
  * `12:00:60.000Z`;
  * `5e-324`;
  * the largest finite double;
  * the single NATO Choice alternative.

**16 rejected documents:**

* missing `Z`;
* a `+00:00` offset;
* hour 24 with a nonzero fraction;
* second 61;
* minute 60 in an optional member;
* an Arabic-Indic digit;
* embedded whitespace;
* a JSON number as a Time;
* JSON `null` as a Time;
* a negative Float64;
* a quoted Float64;
* an invalid NATO special word;
* two Choice alternatives;
* a missing required Time;
* an unknown member;
* malformed JSON.

Every output Time is an exact Zulu JSON string, and Float64 outputs remain
JSON numbers.

| Release | GNAT 14.2.0 (Normal / Check / Ignore) | GNAT 16.1.0 (Normal / Check / Ignore) |
|---|---|---|
| 2.5 | 10 accepted, 16 rejected in each policy | 10 accepted, 16 rejected in each policy |
| 2.6 | 10 accepted, 16 rejected in each policy | 10 accepted, 16 rejected in each policy |

Evidence is in `real14-01/` and `real16-01/`. A service that generates and
executes successfully here is **not** a claim of interoperability with any
actual provider.

## Preservation

* **Task 097 regression controls** (`scripts/task097/preservation.py
  --include-lossless`, with the parent build as baseline). Of 120
  comparisons: **86 legacy outputs are byte-identical, 19 atomic rejections
  match, and 15/15 lossless outputs are byte-identical.** These are the
  historical 86/19 controls plus the lossless profile.
* **Broad sweep** (`scripts/task098/sweep.py`). This covers every committed
  `service-generate` fixture × Ada/Rust/C++ × with/without codec × every
  accepted Ada runtime: 464 comparisons.
  * **458 are identical** in exit status and complete output tree (336
    successes, 122 rejections).
  * 116 of those rejections also have byte-identical diagnostics.
  * The 12 differences (6 outputs, 6 diagnostics) all involve the Ada codec
    on `xs:time` fixtures:
    * **6 newly READY:** the six `codec-ada-time-zulu*` profile runs. Each
      one's model/API bytes equal the parent's model-only output.
    * **6 still rejected, first blocker changed:** the first codec blocker
      moves from `Clock: … Time` to a pre-existing, unchanged construct.
      These are `codec-time-zulu` (abstract polymorphic member, 3 runtimes)
      and `codec-ada-time-zulu-float32` / `-mixed` when the required
      Float32/Float64 runtime is not selected.
* No Rust, C++, Ada model/API, Time-free portable, Float32-only or Float64
  output changed.
* The full 27,167/20,474 Float64 corpus was not rerun, because no numerical
  production source changed.

## Tests, gates and hosted CI

* **New Rust tests.**
  * Core: `task098_named_time_zulu_is_the_only_admitted_time_profile`,
    `task098_named_time_composes_through_records_choices_and_repetition`.
  * CLI: `task098_ada_named_time_zulu_codec_ready_and_model_bytes_unchanged`,
    `task098_unsupported_time_neighbours_stay_atomic_rejections`.
* **Local gates** (Rust 1.95.0, `rust-gates-02/`) all exit 0: `fmt --check`,
  `check`, `clippy -D warnings` and `test --workspace --all-targets`. The
  workspace tests report 1439 passed, 0 failed and 8 ignored. The CI-integrity
  checks also pass: `check-ci-split.sh`, `test-check-ci-split.sh` (210
  checks) and both Task 077B layout checks.
* **Environmental failure, preserved.** The first attempts `rust-gates-01/`
  and `qualify16-01/` failed only because the shared tmpfs `/tmp` ran out of
  inodes (1,048,575 of 1,048,576 used; `No space left on device` in
  `task060_probes` and in one GNAT 16 lossless build). The content filling it
  belongs to other tasks and was not deleted. Instead, the reruns
  `rust-gates-02/`, `qualify14-02/` and `qualify16-02/` use task-owned,
  disk-backed `TMPDIR`s, with the same sources and generator. No test or
  check was relaxed.
* **New Fast CI job `ada-time-zulu-codec`.** It runs a GNAT 14/16 matrix on
  the pinned Debian glibc 2.41 image, and it:
  1. verifies the external GNAT 14.2.0;
  2. provisions the compiler freshly;
  3. builds the exact-head generator;
  4. runs the 18-cell matrix;
  5. requires the exact cell set with nondegenerate accept and reject counts;
  6. always uploads the textual evidence.

  Existing jobs are unchanged. The real-UCI DLZ vertical needs the pinned UCI
  roots, so it remains a local qualification step; Fast CI must not fetch UCI
  (enforced by `check-ci-split.sh`).

## Remaining unsupported

* direct `xs:time`;
* named Time with any other constraint shape;
* non-Zulu offsets;
* value-space Time equality, ordering or canonicalization;
* DLZ under the portable and Float32 profiles (blocked by Float64);
* every construct that was unsupported before this task.
