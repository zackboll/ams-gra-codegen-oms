# Task 102 — real-UCI Ada Log and hexBinary through pinned Sleet

## Authority, ancestry, and delivery boundary

This work started **stacked on PR #131**, with the open, green reviewed parent
`bbc2c3097792942f57341c6ec1c870406387dc02`, tree
`78b75353c2629f7d0e41f755e0784bf8620333ed`. The branch is
`feature/102-ada-real-uci-log-sleet`; the worktree is
`/home/zboll/git/ams-gra-codegen-oms-task102`; evidence is separate under
`/home/zboll/task102-scratch`. No Task101 branch was changed.

PR #131 merged at `2026-10-31T04:09:42Z`, as
`fc36c97f0c78a9fdae4f1c03715a5fbeb9db01c2`. Its main tree equals the reviewed
parent tree. Normal merge `42354b626088346c009df595c8462a1e30ae7cd1` incorporates
that main revision, with parents the reviewed Task101 HEAD and the main merge.
It changes no source bytes. The eventual Task102 PR therefore targets `main`,
not the now-merged feature branch. No rebase, amend, force-push, approval, merge,
release or publication is part of this qualification.

Final source SHA/tree and hosted run/attempt identities must come from the
delivery records and PR, not a self-referential hash in this document. Development
evidence is explicitly not clean-source acceptance evidence. No hosted success
or readiness is claimed merely because a local cell passed.

## Authentic inputs

| Input | Identity |
| --- | --- |
| UCI 2.5 release commit | `093610b7753944059360d3236770ab446d039556` |
| UCI 2.5 MessageDefinitions SHA-256 | `ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27` |
| UCI 2.5 SecurityMarkings SHA-256 | `4a8056f1503234d423a0c2495844ab65387febced62bea58b87d4f343e9e7a0b` |
| UCI 2.6 release commit | `78eb61b6112c8bffa40820c33124b57787fc5bd9` |
| UCI 2.6 MessageDefinitions SHA-256 | `af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b` |
| UCI 2.6 SecurityMarkings SHA-256 | `ee6e58d5db9fd80d526bc964b8244ec296d106301640d34c3cb2c5c00b710b88` |
| Sleet commit | `e38f61d8ce0d75c8508434a52f2ed77c69cf6a3b` |
| Sleet tree | `7e705f4eb5f658957bd414b4abc8207311f3c1e4` |
| Local Sleet binary SHA-256 | `4a2e9ee872d1deaa1a6ce1a1494a2f601660eb57c0df28a447b28a08eba21c94` |
| Compilers | GNAT 14.2.0 and GNAT 16.1.0 |
| Generator toolchain | Rust 1.95.0 |
| Codec profile | portable OMS JSON; **no Float64/lossless runtime requirement** |

The initial roots and includes are hash-verified copies of Task101's authentic
qualified releases, not simplified schemas. Full Log transport always uses the
unaltered complete two-file closure. Campaign evidence copies these inputs and
records their hashes, generator hash, actual tool paths/hashes/versions, source
SHA/tree/status, commands, exits, and generated sources. Sleet is a clean,
separately owned checkout and its own locked build is reproduced by regression.

## Mandatory Phase A: independent broker admission

Before the typed consumer was developed, a separate OWP probe published the
**exact existing Task093 full Log document** for each release. Both passed:

| Release | Schema load | OWP INIT/INFO | `SUB 1 Log task102-log` | PUB receipt | Actual MSG | Cleanup |
| --- | --- | --- | --- | --- | --- | --- |
| 2.5 | unchanged authentic closure | accepted | accepted | `+OK` | body equals Task093 golden, global `Log` | exit 0; socket closed |
| 2.6 | unchanged authentic closure | accepted | accepted | `+OK` | body equals Task093 golden, global `Log` | exit 0; socket closed |

Original evidence: `/home/zboll/task102-scratch/phase-a-25` and
`/home/zboll/task102-scratch/phase-a-26`. Each contains input bytes, schema
selection, full wire transcript, configuration, server logs/exit, identity and
result. The repository-owned admission runner independently repeats this gate
inside each compiler campaign. Loopback binding and a one-topic allow-list are
mandatory. Sending bytes, loading a schema, or receiving `+OK` alone is not a
delivery proof. The delivered message is compared independently with the golden.

## Generated service and portable model/codec parity

The Task102 contracts select one input and one output endpoint for global `Log`.
UCI metadata remains distinct (`2.5` versus `2.6`, and `002.5.0` versus
`002.6.0` in the wire documents). The original Task093 contract/goldens and
consumer are unchanged.

| Release | Selected/renderable model types | Codec declarations | Status |
| --- | --- | --- | --- |
| 2.5 | 42/42 | 41/41 | READY |
| 2.6 | 44/44 | 43/43 | READY |

These historical results were reproduced with a production generator freshly
built from the candidate. Each campaign generates the original Log-only
selection and the Task102 bidirectional selection, compares all `programs*`
model files and both `service_codec` files byte-for-byte, and separately checks
model/API equality with and without `--with-codec`. Only the service endpoint
descriptors/facades change. Generated model and codec files are never patched.
Both wrong-direction controls must fail compilation for the absent generated
`Publisher` on input and `Subscriber` on output; positive generics compile.

The fixed Ada consumer uses generated `LogMT`, `Service_API`,
`Service_Codec.Encode_LogMT`/`Decode_LogMT`, `OMS_LACAL.Typed`,
`OMS_LACAL.Connections`, and the existing WebSocket `Client`. The wire global is
**`Log`, never `LogMT`**. The UCI 2.6 named Binary wrapper is accessed through
its existing additional `Value` component in the consumer, exactly as Task093
does; the generated carrier is unchanged. There is no replacement codec,
protocol, or runtime.

## Typed transport controls

Per transport cell, required evidence includes:

* Six typed publications: full Task093 golden; independently authored optional
  Data absent, present-empty, populated `FF008001007F`; lowercase `0080ff`; XML
  boundary-whitespace spelling. Encoder output is independently compared to the
  expected JSON, never accepted just because its own decoder round-trips.
* Canonical `00 80 FF` bytes encode as `"0080FF"`. The handler additionally
  examines zero/high-bit/max octets and the independent six-byte variation.
* The golden exercises required header identity, DateTime, Duration, and two
  repeated OwnerProducer Choice arms. The unchanged Task093 consumer checks the
  actual typed fields, canonical equality, optionality and invalid expectations.
* Pinned Sleet accepts odd hex `000`, invalid digits `0G`, and internal-space
  `AA BB` as unconstrained strings. These three actual deliveries must each
  produce a recoverable Ada `Decode_Error`, with no handler call, followed by
  successful delivery on the **same** subscription. Diagnostic order is checked.
* Null/number/object binary values reject with broker `TypeMismatch`; an array
  rejects with `OccurrenceMismatch`. Invalid Choice, missing required header,
  `LogMT` global, unauthorized topic, unknown SID and malformed JSON reject at
  their observed broker stages. Unknown service initialization closes its socket.
  These broker rejections are never counted as Ada decoder executions.
* Lowercase and XML whitespace are also independently published by the protocol
  probe, proving Sleet admission and preservation of the original spelling.
  Task093 separately proves Ada acceptance and uppercase canonical encoding.
* A second live typed subscription receives the post-unsubscribe publication
  exactly once while the primary handler count remains unchanged.
* Seven primary calls, three decoder rejections, one witness call, six typed
  publications and ten broker negatives are required in each cell. All actual
  delivered documents, global identities and ordering are independently checked.
* The Ada connection closes and `/proc/self/fd` and `/proc/self/task` counts
  return to their warmed baselines; child/reader processes terminate, Sleet exits
  zero with `shutdown complete`, and its listening socket is closed.

## Separate constrained Binary compatibility limit

Neither Log golden introduces SHA fields. A separate, clearly labeled synthetic
envelope uses **unchanged extracted authentic** `SHA_2_256_HashType` and Binary
members from the Task093 extraction. The broker-only synthetic wrapper has an
explicit schema-version attribute because Sleet refuses a versionless schema;
the authentic roots, extracted declarations, member types and facets are not
changed. This probe is not described as full authentic Log transport.

For both releases, measured pinned Sleet behavior is:

| SHA value | Lexical characters | Broker observation | Required Ada observation |
| --- | ---: | --- | --- |
| 32 octets (valid SHA-256) | 64 | rejects `FacetViolation`, `length 64 != 32` | accepts |
| 16 octets (invalid SHA-256) | 32 | `+OK` and actual MSG | rejects |
| 31 octets | 62 | rejects length | rejects |
| 33 octets | 66 | rejects length | rejects |

Sleet counts lexical characters rather than XML Schema octets for this length
facet. Ada's checked carrier retains the correct octet semantics under all
assertion policies; the separate fixed hash probe requires 1 accept / 3 rejects.
This incompatibility is recorded, not worked around by relaxing a carrier,
editing the schema, or substituting a convenient binary field into Log.

## Matrix, development failures, and regression attribution

The required full transport matrix is GNAT {14.2.0,16.1.0} × UCI {2.5,2.6} ×
assertions {Normal,Check,Ignore} × optimization {O0,O2}: **24 executions**, 12 per
compiler. Each final campaign also reruns the original Task093 full Log consumer
under all three policies, requiring exactly
`TASK093_LOG_COMPLETE checks= 29 rejections= 9`, plus separate authentic extracted
Binary and constrained-length probes. No ignored or filtered test counts as an
execution. A dirty-source development invocation explicitly cannot produce
`completed: true`; the hosted verifier requires clean exact-source status.

The first broker-negative development attempt expected `TypeMismatch` for array
Data; actual `OccurrenceMismatch` was preserved and the expectation corrected.
A UCI 2.6 development build exposed the additional named Binary wrapper in a
new octet helper; only the handwritten consumer access was corrected. Another
hash-probe build erroneously applied `-gnat2022` to all GNATCOLL sources instead
of only the consumer; the consumer GPR now follows the existing project-switch
pattern and leaves provider dialects unchanged. Initial Sleet regression launches
missed local Python package/generator environment; corrected launches passed.
All failures remain in separate evidence directories/logs, not overwritten.

The first complete development matrix produced all 24 passing cell results,
but both whole campaigns failed their final source-drift guard because verifier
inputs were edited during execution. Those campaigns are **not final passes**.
Fresh stable-source qualification is mandatory.

Completed local regression observations before final source qualification:

| Gate | Observed result |
| --- | --- |
| Rust 1.95 format/check/Clippy, locked workspace | exits 0 |
| Full workspace with `AMS_GRA_REQUIRE_GNAT=1` | 1,498 passed; 0 failed; 8 ignored; 0 filtered |
| Original Task093 Log: 2 releases × 2 compilers × 3 policies | 12 executions; each 29 checks / 9 rejections |
| Original Task093 extracted Binary, same matrix | 12 executions; each 14 checks |
| Task100 full transport/Sleet regressions | 24 cells per compiler; both campaigns exit 0 |
| Task101 unchanged DLZ qualification | 12 transport cells per compiler plus 26-document codec corpus under each release/policy; both campaigns exit 0 |
| Pinned Sleet regression harness | 10 Python executions and 7 Rust executions passed; optional real-UCI Rust cases not requested in this invocation |
| Selector/orchestration suite | 193 tests passed in the recorded complete run, including 22 Task102 tests |
| Task101 verifier/ownership controls | 20 tests passed, including actual disposable different-owner fixture |
| CI002/trusted dispatch | 30 tests passed |
| Real-UCI evidence/selection tests | 18 tests passed |
| Split mutation tests | 15 tests passed |
| Job-local layout mutation suite | 3 tests passed; two 364-case mutation invocations |

These counts have their own exact logs/exits under the evidence root. Final
clean-source runs, remaining regression executions and hosted acceptance are
tracked separately; this table is not a claim that the delivery gates are done.
Independent source comparison checked 666 inherited production, runtime,
Task093 fixture, Task101 qualification/verifier and ordinary-workflow files
against the reviewed parent: zero changed files. Pinned actionlint is 1.7.12,
binary SHA-256 `c872d6db8c6bf83a8eaa704fc93999f027d55dffbc63b8a6abdccb47df5f4cd4`.
Additional wrapper controls pass 96 Task060 checks, 4 Task061 checks and
12 Task093 compiler-placement tests.
The complete Task101 regression runs used a clean detached source worktree at
normal merge `42354b626088346c009df595c8462a1e30ae7cd1`, whose tree equals the
reviewed parent. All Task101 inputs and production sources are byte-identical;
results are attributed to that exact source, not to uncommitted Task102 work.
The stable Task102 development campaign then passed all 24 transport cells and
its final input-source drift checks, but intentionally retains `completed: false`
and dirty-source status. Its data cannot satisfy clean-source final acceptance.

## Additive CI and fail-closed evidence

New reusable Deep job: `ada-real-uci-log-sleet (14)` and `(16)`; artifacts:
`task102-ada-real-uci-log-14` and `task102-ada-real-uci-log-16`. Source, consumer,
fixture and verifier paths select the new job through CI-SELECT-002. Shared
generator/runtime/codec dependency rules add Log coverage while projecting back
to the unchanged inherited dependency-ownership hash. Task101's original payload
pins, source-bound verifier and job bodies remain authoritative and unchanged.

Lane placement used the current inventory and measured parent run **41155258305**,
not an old scheduling assumption. Measured ordinary+Deep totals (seconds) were
lane 1: 7,895; 2: 7,381; 3: 9,708; 4: 8,681; 5: 6,335. The two new compiler
cells use lanes **5 (14)** and **2 (16)** without moving any inherited cell.
Only five shared main lanes exist; `queue: max` and caller cancellation remain.
Total registered cells increase 42 → 44 and Deep cells 14 → 16; tests update
those totals without removing any predecessor acceptance requirement.

The Task102-owned verifier rejects incomplete/duplicate matrix cells, missing or
duplicate files, wrong source/tree/run/producer-attempt/compiler, modified schema
roots/includes, generated byte-parity drift, failed/skipped command exits,
missing static direction probes, missing publications or actual delivery,
incorrect broker-versus-codec diagnostics/stages, missing lexical/constrained
observations, and missing cleanup. The mandatory aggregate uses the unchanged
CI-RETRY-001 producer resolver, then verifies both source-bound bundles.
Adversarial tests use explicitly synthetic bundles, never substituted live
qualification evidence. Exact-path command-scoped Git trust under isolated HOME
and bounded hidden-source uploads preserve Task101's ownership/upload lessons.
No wildcard trust, developer HOME, Git credentials or `.git` checkout is uploaded.
PERF-007 shard protection, CI002 security and CI-BENCH-001 manual-only isolation
retain their inherited pins; `pr-ci-required` remains mandatory.

## Unsupported features and review stop

This does not qualify Float32/Float64 portable codecs, arbitrary messages or
schemas, base64Binary, repeated Binary scalars, polymorphic/open-world Log,
alternate brokers, TLS/authentication deployment, or constrained SHA transport
as conforming interoperability through this Sleet revision. The supported Log
closure does not need the lossless numerical runtime. Broker acceptance never
replaces Ada validation. Do not approve or merge from local development evidence;
require exact-head hosted Fast/Deep and source-bound aggregate before independent
review readiness.