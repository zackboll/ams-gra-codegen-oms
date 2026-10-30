# Task 101 — real-UCI Ada typed DLZ through pinned Sleet

## Authority and source isolation

Verified starting `origin/main`: `e186c9dd7c700eb56e142a22309256f3de9a0141`,
tree `7074b9ded51f0aa23458c4d52c26b581dfafc143`. This includes Task098,
Task099, Task100, PY-008 PR #119, and merged CI-SELECT-002 PR #123. No existing
Task101 branch/PR was present. Worktree:
`/home/zboll/git/ams-gra-codegen-oms-task101`; branch:
`feature/101-ada-real-uci-dlz-sleet`; evidence root:
`/home/zboll/task101-scratch`. Other worktrees and performance/CI PRs are not
implementation targets. Final commit/tree, exact-head hosted runs, and delivery
disposition belong in the delivery evidence/PR, not a self-referential hash here.

| Input | Exact identity |
| --- | --- |
| UCI 2.5 tag commit | `093610b7753944059360d3236770ab446d039556` |
| UCI 2.5 MessageDefinitions SHA-256 | `ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27` |
| UCI 2.5 SecurityMarkings SHA-256 | `4a8056f1503234d423a0c2495844ab65387febced62bea58b87d4f343e9e7a0b` |
| UCI 2.6 tag commit | `78eb61b6112c8bffa40820c33124b57787fc5bd9` |
| UCI 2.6 MessageDefinitions SHA-256 | `af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b` |
| UCI 2.6 SecurityMarkings SHA-256 | `ee6e58d5db9fd80d526bc964b8244ec296d106301640d34c3cb2c5c00b710b88` |
| Sleet commit / tree | `e38f61d8ce0d75c8508434a52f2ed77c69cf6a3b` / `7e705f4eb5f658957bd414b4abc8207311f3c1e4` |
| Locally rebuilt Sleet binary SHA-256 | `4a2e9ee872d1deaa1a6ce1a1494a2f601660eb57c0df28a447b28a08eba21c94` |
| Rust / Ada | Rust 1.95.0; GNAT 14.2.0 and 16.1.0 |
| Codec runtime | `linux-x86_64-glibc-2.41-lossless-v1` |

Protocol authority remains OMSC-SPC-013 Rev B and the existing
[Task099](task-099-ada-lacal-runtime-core.md)/
[Task100](task-100-ada-websocket-transport.md) contracts. The official specification
identity and source anchors are preserved in the existing Task076 authority
record. This task neither changes the protocol nor interprets Send as acceptance.

## Phase A: authentic schema admission, separately from Ada compilation

Both roots were freshly fetched with the repository's pinned fetch scripts.
The complete include closure is MessageDefinitions plus SecurityMarkings for
each release; hashes match the authentic checkout/extracted release bytes.
Neither declarations nor validation facets were stripped or relaxed.

| Release | Production Ada generation | Pinned Sleet full-schema admission |
| --- | --- | --- |
| 2.5 | 54 selected types; 53/53 emitted codec declarations READY | Loaded version `002.5.0`, 722 globals, 4,612 complex types, 8,169 facets |
| 2.6 | 54 selected types; 53/53 emitted codec declarations READY | Loaded version `002.6.0`, 725 globals, 4,636 complex types, 8,171 facets |

Each release's unchanged Task098 full DLZ document received `+OK` and matching
`MSG` delivery in a preliminary broker admission probe. This is separate from
the Ada qualification: successful schema loading or successful codec compilation
alone is not transport interoperability evidence.

The Task098 26-document corpus was executed against actual generated model/API/
codec under both compiler families and all three assertion policies: each cell
accepted 10 and rejected 16. Preliminary evidence is in `compile-14/`,
`compile-16/`, `message-admission-25/`, and `message-admission-26/`. These are not
substitutes for the clean-source final campaign.

## Generated API/runtime architecture and wire identity

`tests/fixtures/task101/uci25-service.yaml` and `uci26-service.yaml` preserve the
Task098 input contract. The campaign adds one **output** exchange for the same
message/topic, then invokes the existing production generator with `--with-codec`.
The generated input endpoint exposes only Subscriber; the generated output
endpoint exposes Publisher. No input endpoint publishes and no generated source
is hand-edited. Generated `DLZ_MT`, `Service_Codec`, and `Service_API` are used
directly by `tests/ada-real-uci-dlz/dlz_consumer.adb`.

The consumer instantiates `OMS_LACAL.Typed` with the generated handler interface
and codec operations, and connects `OMS_LACAL.Connections` to
`OMS_LACAL.Transports.WebSocket.Client`. There is no background receive task.
The service and endpoint descriptors supply service name, topic, and global
element identity. A single contract with separate input/output exchange
occurrences suffices; the selected schema closure remains unchanged.

The original QName is
`{https://www.vdl.afrl.af.mil/programs/oam}DLZ`. As specified for the OAM namespace,
the wire name is bare `DLZ`, not `DLZ_MT`:

```text
INIT {"versions":["1.0"],"schema":"002.5.0","service_id":"task098-dlz","verbose":true}
+OK
INFO <validated broker identity>
SUB 1 DLZ task098-dlz
+OK
PUB task098-dlz {"DLZ":<generated DLZ_MT encoding>}
+OK
MSG 1 {"DLZ":<original routed encoding>}
UNSUB 1
+OK
```

The 2.6 cell uses its own root, contract, document SchemaVersion `002.6.0`,
generated outputs, and logs. No 2.6 result is inferred from 2.5.

The receiver compares the decoded/re-encoded value with the corresponding typed
published value; an independent Python oracle additionally checks every published
document's JSON kinds, exact Time spellings, and binary64 bits. The full sample
includes required/optional Time, nested Velocity2D, Float32 vector components,
negative zero, and both required repeated OwnerProducer Choice alternatives.
Other publications cover omitted optional Times, long fractions, leap-second
spellings, the least positive subnormal, maximum finite double, and NATO Choice.

## Execution matrix and non-timeout lifecycle witnesses

The mandatory campaign is two compilers × two releases × three policies × two
optimization settings: **24 real Sleet executions**. Each compiler must complete:

| Release | Normal O0/O2 | Check O0/O2 | Ignore O0/O2 |
| --- | --- | --- | --- |
| UCI 2.5 | 2 executions | 2 executions | 2 executions |
| UCI 2.6 | 2 executions | 2 executions | 2 executions |

Each execution requires 10 actual generated Ada publications, 11 primary handler
calls (one independent recovery document plus those 10 publications), exactly one
codec decode rejection, and one secondary typed-handler call after primary
unsubscribe. The secondary handler positively witnesses broker routing of the
post-unsubscribe publication while the removed handler count stays unchanged;
absence is not satisfied merely by waiting for a timeout. Binding mappings are
explicitly closed; descriptor and `/proc/self/task` counts must equal the warmed
baseline. Both consumer and server must exit zero, and Sleet must log routing and
`shutdown complete`.

Preliminary `repository-r2-14/` and `repository-r2-16/` completed their 12-cell
matrices. They were development runs with a dirty checkout and intentionally do
**not** pass the source-bound final verifier. Final evidence must be rerun in a
fresh directory from committed source after the fixture/codec-corpus additions.
No unfinished or superseded cell is a success record.

The clean committed-source campaigns `final-b271b13-14/` and
`final-b271b13-16/` subsequently passed the independent source-bound verifier:
each completed all 12 real-Sleet cells, six real-DLZ codec cells (156 documents,
60 accepted/96 rejected), 132 primary handler calls, 12 witness calls, 120 typed
publications, 120 broker rejections, and 12 recoverable Ada decode rejections.
All consumer/server exits were zero. This qualifies source
`b271b13a4bfb6251998eafcc973f1a0e823937ef`, tree
`a90ae58da7cb5d3e660befb624b63e29d81451ed`, not a later corrective head.
The final per-tool identity recording correction requires a fresh campaign;
delivery evidence must name that new source rather than relabel these runs.

## Negative stages and recovery

| Input | Observed / required stage and error |
| --- | --- |
| Missing required TimeOfIntercept | Broker `-ERR Invalid-Message`, `MissingRequiredMember` |
| Numeric Time offset | Broker `FacetViolation`, pattern `.+Z` |
| Null or numeric Time | Broker `TypeMismatch`, expected string |
| Quoted Float64 | Broker `TypeMismatch`, expected number-or-special-float-string |
| Two OwnerProducer Choice arms | Broker `ChoiceViolation` |
| Global element `DLZ_MT` | Broker `UnknownGlobalElement`; payload type is not message identity |
| Unauthorized topic | Broker `-ERR Illegal-State`, exact topic authorization detail |
| Unknown UNSUB SID | Broker `-ERR Illegal-State`, unknown subscription id |
| Unknown SID passed to typed binding | Local ownership rejection before wire traffic |
| Unknown service | `-ERR Unsupported-Service`, followed by observed WebSocket closure |
| `not-a-timeZ` | Broker accepts `. +Z` pattern (without the space); generated Ada codec rejects `invalid named Time` |
| Duplicate Time with invalid effective last member | Broker `TypeMismatch` |
| Duplicate Time with valid effective last member | Existing documented last-value-wins policy; Ada receives the good recovery value |

The last two cases characterize existing policy; they do not claim universal
duplicate rejection. No new parser or duplicate policy is introduced. The
generated codec's existing corpus separately checks Time kind/lexical failures,
Float64 constraints, Choice violations, missing/unknown members, and malformed
JSON. Broker errors are never counted as codec errors. The broker-accepted invalid
Time reaches the original typed subscription, raises one Decode_Error with the
expected text, and is followed by a valid decoded handler call on that same SID.
OWP framing/state errors remain covered by unchanged Task099/100 regressions,
not relabeled as broker schema validation results.

## Byte preservation and source evidence

No production generator/model/API/codec/runtime/transport implementation changes
are required. The campaign compares generated model/API bytes with/without codec,
and compares all model/codec sources against the original Task098 input-only
selection. Adding the output endpoint intentionally changes only Service_API
and the service manifest. All generated source, contract, schema/include, compiler,
linker, generator and Sleet identities are retained. Baseline regenerated selections
must be compared separately against the verified main binary before final delivery.

The separately built verified-main generator and candidate generator have the
same SHA-256 `f1a0fd113c155f232b625500d6141496b68d8bc06d0bb234757c55a5586c14f9`.
`generated-parity/result.json` records 238 byte-identical selections: 182
successful output trees and 56 matching atomic rejections, including both real
DLZ releases and all available service fixtures across admitted Ada profiles.
`production-source-parity.json` proves 462 production source files unchanged.

Published payloads, original delivered envelopes, broker diagnostics, command
argv/cwd, exits, per-cell handler counts, and cleanup markers are retained, not
replaced by a successful Send flag. Core markers are:

```text
TASK101_UPGRADE_OWP_COMPLETE
TASK101_INFO_VALIDATED
TASK101_SUB_ACCEPTED DLZ task098-dlz
TASK101_DECODE_RECOVERY_COMPLETE
TASK101_UNSUB_WITNESS_COMPLETE primary_unchanged witness=1
TASK101_CLEANUP_COMPLETE
TASK101_ADA_REAL_DLZ_COMPLETE handlers= 11 rejected= 1
TASK101_QUALIFICATION_COMPLETE <compiler> 12
```

## Hosted qualification and CI selection

The additive reusable Deep job `ada-real-uci-dlz-sleet` has compiler 14 on shared
lane 3 and compiler 16 on lane 5, with `queue: max`, the existing pinned Debian
image, and a 120-minute ceiling. The observed inherited Task100 pinned-Sleet jobs
in main run `40971692898` took about 8.6/16.3 minutes; Task101 has half as many
transport build cells but adds two authentic schema releases and a codec corpus.
No lane or runner bypass is introduced. There remain exactly five shared main
lanes. Total runner-cell inventory increases by two, from 40 to 42.

Mandatory artifacts are `task101-ada-real-uci-dlz-14` and
`task101-ada-real-uci-dlz-16`. `pr-ci-required` reconciles both through the existing
authoritative producer-attempt resolver and the independent Task101 bundle
verifier. It rejects missing/duplicate/expired/stale/cross-source artifacts,
skipped mandatory steps, source SHA/tree/compiler/release drift, incomplete or
duplicate cells, changed source hashes, wrong authentic roots/includes, absent
codec/handler/cleanup completion markers, and inconsistent process exits.
Synthetic adversarial verifier tests are explicitly not interoperability evidence.

The CI-SELECT-002 architecture is preserved. Dedicated Task101 consumer/harness/
fixture/verifier paths select both ordinary Ada transport compiler cells and the
new Deep job; existing Ada/runtime/codec/shared infrastructure ownership also
selects Task101. Unknown and unsafe changes remain fail-broad. Existing Task100
payload/source-bound reconciliation, both Python interpreter cells, PERF-007's
three disjoint shards, CI002 security source pins and mandatory aggregate remain.

## Inherited main blocker and qualification status

Run `40971692898`'s Task094 artifact was downloaded separately. Its nested
`qualified-gnat16.1.0-toolchain.log` ends with `curl` exit 28 fetching
`https://github.com/AdaCore/gnatcoll-core/archive/refs/tags/v26.0.0.zip` after a
GitHub port 443 connection timeout. This is a dependency-fetch failure, not a
Task101 codec or compiler assertion failure. No Task094 gate is altered or
suppressed. Current-main run `40991588790` was still pending at investigation;
successful final proposed-integration qualification remains mandatory.

Rust 1.95.0 fmt, locked workspace check, Clippy with `-D warnings`, and the
GNAT-required locked all-target workspace suite passed during development:
**1,493 passed, zero failed, eight pre-existing ignored**. Ignored cases are not
executed interoperability evidence. Task098/099/100 regressions and final
source-bound execution must finish before a PR is opened; exact-head hosted
success is separately required for review readiness.

Completed local regressions preserve all existing execution distinctions:
Task098 36 cells/5,616 documents, Task099 48 cells, and Task100 48 cells including
12 existing synthetic pinned-Sleet runs. The selector suite passes 128 tests;
Task101's verifier/selector/provenance controls pass 15 tests, with per-file
missing/duplicate and per-field adversarial subcases. All script unit tests pass
215 tests when the existing `strace` prerequisite is on PATH. CI002 passes 30
tests. Split guard and 556-check adversarial split suite pass; layout ownership
passes its three tests, including two independent 364-mutation campaigns.
Pinned actionlint 1.7.12 (binary SHA-256
`c872d6db8c6bf83a8eaa704fc93999f027d55dffbc63b8a6abdccb47df5f4cd4`)
and whitespace checks pass. Initial harness/setup failures remain preserved,
including missing strace, a missing layout-checker CLI argument, and the
development port-zero probe; none is recorded as successful qualification.

## Explicit unsupported features

No remote plaintext, TLS/wss, reconnect/restoration, background receive task,
alternate protocol/parser/codec, general polymorphism/$type, OpenExtensions
registry, arbitrary unsupported UCI types, or resource-exhaustion recovery
guarantee is added. Sleet's limited temporal validation is recorded rather than
changed. No production capability is broadened to manufacture a passing result.

## R1: hosted checkout ownership and current-main reconciliation

The original source was `844a4da20c6170607c5374963a20d66df236266d`, tree
`2cff07ab000a5b84e7510d6403f9a52446129795`. Hosted run `41002089379` failed
before DLZ execution in jobs `123888643214` and `123888643217`. Retained
artifacts `12954101998` and `12954360436` independently record `source.exit=128`
and Git's dubious-ownership error. The shared Campaign deliberately changes HOME
to its isolated Alire home, hiding the workflow's earlier global Git trust entry.

R1 scopes `git -c safe.directory=<resolved checkout>` to Task101's source and
source-status calls only. It does not change persistent configuration, wildcard
trust, shared Campaign environments, authentic schemas, codec expectations or
Sleet. Nonzero exits retain their original evidence and abort; a nonempty clean
status contract now also aborts immediately. The source-bound verifier still
requires exact expected SHA/tree and successful source/status exit records.

Portable regression tests exercise Git's real ownership validator using its
different-owner test hook. The optional
`TASK101_OWNERSHIP_FIXTURE=podman` test additionally assigns a different UID to a
disposable checkout using rootless user namespaces, restores ownership in a
finally block, and needs no privileged changes on ordinary test runners. Both
tests check untrusted and wrong-path failures, exact-path SHA/tree and clean
status success, dirty rejection, isolated HOME, no wildcard or persistent trust,
and durable nonzero failure records. Verifier mutations reject source/status
exit 128 even when other synthetic completion records claim success.

Normal merge `713ae273b96da0e21440236ab315a26d9f38fe78` incorporates CI-BENCH-001
main `b65937fefa79818ac460b28ba3d3ae5996cf0506`, tree
`101b2fdb0931a2c9c9a543c1d91667dfab513245`, with parents original Task101 and
that main commit. Benchmark controller/workflow hashes are not updated to bless
Task101 changes: Task101 uses the existing mandatory upload-step name, restoring
the inherited provenance resolver's exact bytes. Policy projects only this
single step-name change to the original Task101 payload hash; inherited hashes
remain unchanged. Focused integration tests preserve both Task101 compiler jobs,
manual-only benchmark isolation, selective Deep, producer-attempt validation,
PERF-007, CI002/trusted-dispatch boundaries, five lanes and the required gate.

Main advanced again during R1: a second normal merge incorporated PY-009 main
`a44ce80f278c0a64c604a1b800679e02f8687cdf`, tree
`845a29e78466fa5ad2397cff8c475cf298027deb`. Merge
`5fdc44519361d44582759944f3d6705fb7ecfee8` has parents
`38a49e1209439f57e670dc02636851f9ca140de0` (the R1 correction) and that main
commit. Python backend/fixture additions belong to merged main, not Task101.
PY-009's additive workflow projection remains intact; its runner-count test is
updated from 40 to the combined 42 cells, without relaxing payload contracts.

R1 evidence is separate from all historical campaigns under
`/home/zboll/task101-r1-scratch/`, including untouched original hosted ZIPs/logs,
ownership reproductions, production-byte comparisons, current-source campaign
records and hosted reconciliation. Final identities and actual completed counts
are recorded there and on PR #131; historical totals above are not R1 execution
claims. Readiness requires all new exact-head hosted mandatory gates. Stop before
approval, merge, auto-merge, release or publication.