# CPP-OMS-001 — Generated C++17 OMS JSON payload codecs

`service-check --language cpp --with-codec` and
`service-generate --language cpp --with-codec` now produce a **separate**
codec header, `service_codec.hpp`, for a deliberately small, fully tested
subset of the C++ model. The generated model header and `service_api.hpp` are
byte-identical with and without the flag; without the flag nothing changes.

This is an initial vertical, not UCI compatibility. See
[Remaining limitations](#remaining-limitations) and
[Pinned UCI measurement](#pinned-uci-measurement).

## Dependency strategy

**Standard library only (C++17).** The header embeds a small strict RFC 8259
reader and a direct writer inside `service_codec::detail`; nothing is
installed, vendored or linked, and the header is self-contained apart from the
generated model header it includes.

Why not nlohmann/json, RapidJSON or Boost.JSON: the wire rules need behavior a
general library either does not promise or hides behind configuration —

* **duplicate members must be rejected** (a conforming reader may collapse them
  silently), including after escape decoding and inside members we then ignore;
* **integers must stay exact**: an integral member accepts only an integer token
  (`7`, `-3`), never `1.0`, `1e1`, a string or a Boolean, and must cover the full
  64-bit signed and unsigned ranges without passing through `double`;
* **strict UTF-8 and surrogate handling** (no lone surrogate escapes, no
  overlong/ill-formed input), identical on every platform;
* a **resource guard** (nesting deeper than 256 is an error, not a stack
  overflow).

A consumer that wants a different JSON engine can replace the private reader;
the typed `decode_*` / `encode_*` boundary does not expose it. The reader and
writer are most of the roughly 750-line fixed runtime at the top of the generated header and are identical for every
schema (only the per-declaration functions vary).

## Generated API

For each **unique selected payload type** `P`, in contract first-occurrence
order (two messages sharing a payload share one pair):

```cpp
namespace service_codec {
struct CodecError { std::string path; std::string message; };
template <typename T> class CodecResult;      // ok(), explicit operator bool,
                                              // value(), error()
CodecResult<std::string> encode_P(const ::ns::P& value);
CodecResult<::ns::P>     decode_P(std::string_view json_utf8);
}
```

They operate on the **payload body** (the JSON object for the message's
complex type), not on the global-element envelope, a topic or a route.
`P` is the model's own type spelling (`upper_camel` of the XSD name); `::ns` is
the model namespace. The codec throws nothing itself (only standard-library
allocation failure can propagate) and `CodecError::path` is schema-derived:
`Payload.Children[1].Count`, or `$` for JSON syntax errors.

Every decoded value is built through the generated model's checked factories
(`create`), so the model decides domain and cardinality membership; the codec
duplicates no bound.

## Supported subset

| Construct | Wire form and validation |
| --- | --- |
| `xs:boolean`, named Boolean | JSON `true`/`false` only |
| Integral (`xs:byte … xs:long`, `xs:unsigned{Byte,Short,Int}`, named, bounded, patterned-domain) | JSON integer token; full `int64`/`uint64` range read exactly; out of the model's domain → rejected by `create` |
| plain `xs:string` member | JSON string; UTF-8 in, UTF-8 out; ill-formed UTF-8 rejected on both paths |
| Enumeration | the exact XSD wire value (not the C++ enumerator, e.g. `"5G"` for `Value5G`); unknown or out-of-range values rejected both ways |
| Concrete Record | object; schema order on encode, any order on decode; inherited members flattened (no nested base object) |
| required member | must be present; `null` rejected |
| optional member (`0..1`) | absent ↔ `std::nullopt`; a present empty String / `{}` is preserved as present |
| bounded and unbounded repetition | JSON array ↔ `BoundedVector` / `UnboundedVector`; count enforced by the model; encode omits an empty repetition, decode accepts absent **or** `[]` |
| nested / repeated Records | recursive, type-specific functions with element paths |

Member names follow OMSC-SPC-013 Rev B §6.1.2 using each local element's own
QName (Task 051): the bare local name for the OAM namespace
`https://www.vdl.afrl.af.mil/programs/oam`, `{namespace}local` otherwise. A
host-language escape never reaches the wire (`field_class` is `"class"`).
`"$type"` is accepted **only** when it names the concrete type being decoded
(`Payload`, or `{urn:x}Payload` outside OAM) and is never emitted: a derived
value in a base-typed position cannot be represented by the C++ model, so any
other `"$type"` is rejected rather than silently losing content.

Decode rejects: malformed or trailing JSON, a UTF-8 byte-order mark, duplicate
members (after unescaping), unknown members, `null`, wrong JSON kinds,
fraction/exponent spellings of an integer, lone surrogate escapes, ill-formed
UTF-8, raw control characters in strings, out-of-domain values and cardinality
violations. Encode rejects values a checked model cannot hold but C++ can
(an out-of-range enumerator, ill-formed UTF-8 in a `std::string`).

## Explicitly unsupported (NOT READY before any file is written)

`service-check` reports `codec status: NOT READY` with a
`service codec boundary:` line, and `service-generate` exits 1 without creating
the output directory, for:

* `xs:float` / `xs:double` (needs correctly rounded parse, shortest print and
  `"NaN"`/`"Infinity"` handling) — Float32 / Float64, named or direct;
* `xs:dateTime`, `xs:time`, `xs:duration` — named or direct;
* `xs:hexBinary` / Binary, constrained or not;
* named Strings with any constrained profile (VisibleAscii, WhitespaceVisible,
  NATO special words, UUID, schema-version, IPv6, bounded/structured/alternating
  ASCII, Unicode 3.1); a plain `xs:string` *member* is supported;
* Choice declarations and members of Choice type;
* abstract structural values / `$type` polymorphism, including uninhabited
  (absent-only) abstract members;
* nillable members; unqualified local elements (no evidenced wire name);
* aliases and lists; anything the C++ **model** itself cannot yet render (that
  is reported as the existing model blocker and the codec is "not measured").

The analysis lives once in `codegen-core`
(`cpp_emission_codec_support`, behind `analyze_service_codec`); the renderer
calls the same analyzer first, so `service-check` and `service-generate` cannot
disagree. Unsupported combinations that sit *outside* the selected closure do
not matter: only the projected declarations are measured.

## Reproducible validation

```sh
# everything below, one command (g++, clang++ if present, ASan/UBSan if usable)
scripts/check-cpp-oms-001.sh

# the registered tests
cargo test --locked -p ams-gra-codegen-oms --test cpp_codec
cargo test --locked -p ams-gra-codegen-oms --test service_codec

# re-bless the golden after an intentional generator change
CPP_OMS_001_BLESS=1 cargo test --locked -p ams-gra-codegen-oms --test cpp_codec
```

Strict flags: `-std=c++17 -Wall -Wextra -Wpedantic -Werror -Wconversion
-Wsign-conversion -Wshadow -Wold-style-cast -Wcast-align -Wnon-virtual-dtor
-Woverloaded-virtual -Wnull-dereference -Wdouble-promotion -Wformat=2`.

What the tests cover (`crates/cli/tests/cpp_codec.rs`,
`tests/cpp-codec/consumer.cpp`, fixtures under `tests/fixtures/cpp-codec/`):

* readiness text, and `--with-codec` flag semantics;
* **no-codec generation unchanged**: model and service API headers
  byte-identical with and without the flag, no codec file without it, default
  summary lines unchanged; a one-off byte comparison against the pre-change
  `main` binary over the whole fixture corpus is recorded below;
* a golden `service_codec.hpp` (`tests/fixtures/cpp-codec/expected/`) and
  determinism (two generations are identical);
* the consumer: exact canonical wire text for a fully populated and a minimal
  value, decode → re-encode identity, non-canonical input (whitespace, member
  order, `\u` escapes, surrogate pairs, `-0`, `[]` vs absence), about 100 named
  malformed-input / domain / cardinality / path rejections, encode-side
  rejections, 64-bit integer boundaries, every proper prefix of a valid
  document rejected, about 5 700 single-byte corruptions that must not crash, a
  200 000-deep array and 50 000-deep object bomb; compiled with strict flags
  under g++ **and** clang++, and under ASan + UBSan;
* non-OAM namespaces (Clark names, `$type`, flattened inheritance, local
  `form="qualified"`);
* 13 unsupported-construct fixtures rejected by both commands with the model
  still READY, no output directory created, and ordinary generation still
  working without the flag;
* a zero-OMS service gets no codec file.

## PR smoke (CPP-OMS-001R1)

The existing single-job `PR smoke` workflow has a fourth, router-selected step,
`python3 scripts/pr_smoke.py run cpp`. `scripts/pr_smoke.py` routes
`crates/backend-cpp/`, `tests/cpp-codec/`, `tests/fixtures/cpp-codec/`,
`crates/cli/tests/cpp_codec.rs` and the two `scripts/check-cpp-oms-001*.sh`
files to C++ only; shared and unknown paths still run every suite (now
including C++). The suite: the two focused `backend-cpp` golden tests; three
exact `cpp_codec` tests (golden/determinism, separate artifact with unchanged
no-codec output, unsupported constructs rejected before output); generation of
the model and codec through the production CLI; a strict-flag `g++` build of
`tests/cpp-codec/consumer.cpp`; and execution of that consumer. A missing
compiler, a compile error, a failed or non-reporting consumer, a missing
generated file or an unexecuted/skipped test all fail the job. clang++ and the
sanitizer build stay in the full `cpp_codec` suite and
`scripts/check-cpp-oms-001.sh`.

## No-codec generation is unchanged (parity evidence)

`scripts/check-cpp-oms-001-parity.sh BASELINE_CLI CANDIDATE_CLI` runs a CLI
built from the pre-change commit and the candidate over every
`tests/fixtures/service-generate` schema/contract pair that shares a stem
(59 pairs): `service-check` and `service-generate` in `cpp`, `rust` and `ada`
without the flag, plus `service-check --with-codec` in `rust` and `ada` (their
codec readiness must not move). Exit codes, normalised stdout and every
generated byte must match.

| Baseline | Candidate | Pairs | Runs | Mismatches |
| --- | --- | --- | --- | --- |
| `a28cb1d` (`origin/main`) | this branch | 59 | 472 | **0** |

Negative control: the same script against a candidate wrapper that silently
appends `--with-codec` reports 292 mismatches, so the comparison does detect
differences. The only intended behavior change is `--language cpp
--with-codec`, which was previously always `NOT READY` with "not implemented
for C++".

## Pinned UCI measurement

Source: UCI 2.5 tag `v2.5` = `093610b7753944059360d3236770ab446d039556`
(`gitlab.com/open-arsenal/uci/standard`), root
`03_OAC-STD-002_RevE_UCI_Schema_v2_5/UCI_MessageDefinitions_v2_5_0.xsd`,
SHA-256 `ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27`,
fetched and verified by `scripts/fetch-pinned-uci-2.5.sh`. Contracts are the
repository's `position-report-loop.yaml` with the message name substituted.
Each row is one `service-check --language cpp --world closed-schema
--with-codec` run (release build):

| Message | Model | Codec-renderable emitted declarations | First codec blocker |
| --- | --- | --- | --- |
| `PositionReport` | READY (60/60) | 38 / 59 | `AccelerationType`: named Float64 |
| `AMTI_Command` | READY (221/221) | 153 / 217 | `AccelerationType`: named Float64 |
| `AO_CapabilityStatus` | READY (64/64 + 68 support) | 68 / 130 | `AO_PIM_CodeType`: constrained named String |
| `AMTI_CommandStatus` | READY (46/46) | 30 / 43 | `DateTimeType`: named DateTime |

**No real UCI message was validated, and this task does not claim any.** The
four rows are measurements, not a sweep of all 722 messages (one run takes
about 17 s). The conclusion that no `*MT` message can be READY is structural:
every message type extends `MessageType`, whose required members
`SecurityInformation` (DateTime, constrained Strings, Choices) and
`MessageHeader` (`SystemID` with the UUID String profile, `Timestamp` of
`DateTimeType`) are all outside this subset. Real-UCI codec support therefore
needs, at minimum, the named String profiles, DateTime, Float64 and Choice
mappings before any message becomes READY; that is future work, not claimed
here.

## Remaining limitations

* This is **not** UCI compatibility. Real UCI messages use Float64, DateTime,
  constrained String profiles, Choices and polymorphism almost everywhere; see
  the measurement below.
* Strings are Unicode scalar sequences only. XML 1.0 `Char` restrictions (for
  example U+0000, U+FFFE) are **not** enforced, matching the unconstrained
  `std::string` model carrier and the Rust/Python codecs.
* `xs:unsignedLong` is not exposed by the XSD frontend, so the 64-bit unsigned
  path is covered by a white-box runtime test rather than a schema fixture.
* Decode takes complete JSON text; there is no streaming, no size limit
  (callers own message size), no transport, OWP framing, envelope handling or
  LA-CAL runtime for C++.
* No claim of conformance beyond the cited OMSC-SPC-013 Rev B §6.1 rules and
  the synthetic fixtures.
