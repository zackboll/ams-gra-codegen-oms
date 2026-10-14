# Task 090 — checked Ada OMS JSON DateTime codecs

## Isolation

Starting main: `dd1353a93f4e1e62fa544f7fb7349a6eba41fc03`, the merged
PR #88 commit. Branch: `feature/090-ada-datetime-codec`. Worktree:
`/home/zboll/git/ams-gra-codegen-oms-task090`. Task-owned, disk-backed evidence:
`/home/zboll/task090-scratch`. Task 088/089 branches and qualification artifacts
were not modified. Main remained at the starting SHA on the subsequent fetch.

## Architecture and domains

Ada readiness calls the existing `temporal_profile` for named DateTime and
requires exactly `TemporalProfile::DateTimeZulu`. For direct members it calls
`direct_temporal_profile` and requires `DirectTemporalProfile::DateTime`.
No type-name recognition, new parser, representation, or timezone conversion
was introduced. Named unconstrained DateTime and unsupported constraints fail
closed; named TimeZulu, Duration, Float, Decimal and Binary remain unsupported
by the Ada codec. Choice and repetition are deliberately outside this branch.

The generated model API is unchanged:

```ada
function Create (Value : String) return Instant;
function Value (Item : Instant) return String;
function Create (Value : String) return Deadline;
function Value (Item : Deadline) return String;
function Create (Value : String) return XML_Schema_Date_Time;
function Value (Item : XML_Schema_Date_Time) return String;
```

Both profiles encode as JSON strings using `Model.Value`. Named decoding uses
the expected return type to resolve the overloaded checked `Model.Create`.
Direct decoding uses a private codec helper, `Scalar_Date_Time`, reserved in
codec naming preflight and emitted only when direct DateTime is stored.
Only the constructor call is in the `Constraint_Error` translation scope;
String extraction precedes that scope. Record decoders do not catch
`Constraint_Error`. Invalid expected input raises `Service_Codec.Codec_Error`.
Optional wrappers and Clark-notation `{urn:test}` wire keys are preserved.

Public payload operations are `Encode_TemporalPayload` / `Decode_TemporalPayload`
and `Encode_DirectPayload` / `Decode_DirectPayload`; their signatures follow the
existing `Item : <model payload> return String` and
`Text : String return <model payload>` codec API.

Named values require literal `Z`. Direct values allow omitted timezone, `Z`,
and valid signed offsets. Whitespace collapse, fractional trailing zeros,
leap seconds, leap days, hour 24, long years and fractions retain the model's
existing XML Schema 1.0 lexical behavior. Lexical equality is not value-space
equality; no timestamp comparison or canonicalization is performed.

## Fixture-first and reproducibility

The original `temporal` and `direct-datetime` service fixtures are used unchanged.
Models and service APIs were generated without codecs first and compiled using
GNAT 14.2.0 before production codec edits. Baseline model hashes are in
`model-baseline.sha256`. Independently authored JSON goldens use qualified wire
keys and reversed member order. Unsupported neighboring temporal declarations
remain outside the projected selected closures.

`scripts/check-task090.py` builds and executes both actual production-generated
codecs with the unchanged JSON guard. It requires Alire 2.1.1, GNAT 14.2.0 or
16.1.0, GNATCOLL 26.0.0 (including the qualified provider archive hash) and
GPRBUILD 26.0.1. Dependency builds are sandboxed in the task-owned workspace.
It records command arrays, exits, compiler identities, dependency locks and
source hashes, and verifies exact nonzero completion counts.

Frozen independent oracles:

| Oracle | SHA-256 |
|---|---|
| `datetime-zulu.txt` | `3230a3270dc2239c53435235578c343248809c08d8d30af9b85a2392ae3c86c2` |
| `datetime-direct.txt` | `a0417fee4fbd1820618f60282200502fce543836e76dd1f952ac0612f33756a5` |

The named consumer uses the entire Zulu corpus. The direct consumer additionally
uses the direct corpus and explicitly assigns positive direct expectations to
the six Zulu corpus inputs rejected solely for omitted/numeric timezone.
Negative tests catch only `Codec_Error`; unexpected exceptions fail execution.

Initial executable campaigns completed successfully on GNAT 14 and 16 under
normal, Check and Ignore policies: **161 named and 205 direct cases per run**.
Task-local sandboxed repetitions also passed for both compilers. Early
concurrent 087A/087B shared-cache builds failed with missing archive objects;
those failures were retained, not certified. Unchanged scripts were subsequently
rerun serially using isolated task-owned Alire settings.

The unchanged Task 087A and Task 087B scripts subsequently completed exit zero
on **both GNAT 14.2.0 and 16.1.0, all three policies**. Task 087A retains its
199-case/four-boundary frozen output hashes; Task 087B retains its 41-case
consumer and seven-case subset markers. Wrapper adversarial controls also pass.

## Regression and delivery gates

Rust 1.95 formatting, locked workspace all-target check, warnings-denied Clippy,
the focused core codec suite and CLI codec suite (14 tests) pass. Recursive byte
comparisons with an archived baseline generator pass for the original Task 087B
Ada codec, Rust model/API/codec and plain output, C++ output, and both temporal
Ada model/API selections. The protected model backend, shared temporal
classifier, Rust codec and C++ backend have no production edits.

The separate Fast CI job `ada-datetime-codec` depends on `ada-json-guard`, checks
out the exact PR head, runs both synthetic fixtures under all GNAT 16 policies,
and uploads logs, hashes, exits, commands and dependency locks. No real UCI
fetch or Sleet build is part of that job. Hosted execution is not implied by
registration.

Real UCI measurements use the existing pinned roots (2.5 SHA-256
`ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27`, 2.6
`af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b`).
Before/after RDMA reports and exact exit records are retained in the evidence
directory. No full RDMA READY claim is inferred from support for DateTimeType.

| Release | Before | After | Newly codec-renderable | First remaining blocker |
|---|---|---|---|---|
| UCI 2.5 | 41/49 | 42/49 | `DateTimeType` | `NetworkEndpointType` Choice |
| UCI 2.6 | 41/49 | 42/49 | `DateTimeType` | `NetworkEndpointType` Choice |

Both selected RDMA services remain codec **NOT READY** (expected command exit
1). The 2.5 first blocker changes from DateTimeType to NetworkEndpointType;
the 2.6 first blocker is unchanged. This bounded measurement is not an inventory
of every UCI service and establishes no entirely codec-ready real service.

The complete affected Rust/Ada/C++/CLI/core campaign finished exit zero:
**1,079 passed, zero failed, six pre-existing ignored** across 112 summaries.
The independent Rust/C++ backend campaign also finished exit zero. Existing
zero-OMS and unsupported-output atomicity controls pass. No ignored synthetic
DateTime case was introduced. Final Python/JSON/workflow syntax, formatting and
`git diff --check` pass.

Publication and exact tested/local/remote/PR head parity are recorded in the
final handoff. Hosted CI status is reported separately and is not claimed here.