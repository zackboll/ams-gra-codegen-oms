# Task 106 — heterogeneous real-UCI Ada messages on one Sleet connection

## Scope, stack and review boundary

Task106 starts from PR #145, open and unmerged at reviewed commit
`334677f8e2f04d7cb89a098ba4bfa425796d2efd`, tree
`6fd380454512636152511744e95038f9c000d7fa`. The freshly fetched remote matched
both identities. No Task106 branch, worktree or PR existed. The isolated branch
is `feature/106-ada-multimessage-uci-sleet`; its worktree is
`/home/zboll/git/ams-gra-codegen-oms-task106`, and evidence is separate under
`/home/zboll/task106-scratch`. The stacked PR must target the still-open
`feature/105-ada-survivability-uci26-sleet`, not main.

Parent PRs #134, #142, #144 and #145, their branches, source worktrees and
evidence are not modified. Normal commits/merges only: no rebase, reset, amend,
cherry-pick or force push. Approval, merge, auto-merge, release and publication
are outside this task. Exact final source/tree, remote/PR parity, proposed merge,
workflow attempts, artifact IDs and authoritative gate disposition belong in
external delivery records, not a self-referential commit hash in this file.

Only **UCI 2.5** is selected for the combined qualification. Task105's UCI 2.6
Survivability campaign remains a separately owned inherited regression. A
single-message 2.6 result is never counted as heterogeneous qualification.

## Immutable authorities

| Input | Identity |
| --- | --- |
| UCI 2.5 release | `093610b7753944059360d3236770ab446d039556` |
| MessageDefinitions SHA-256 | `ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27` |
| SecurityMarkings SHA-256 | `4a8056f1503234d423a0c2495844ab65387febced62bea58b87d4f343e9e7a0b` |
| Sleet commit | `e38f61d8ce0d75c8508434a52f2ed77c69cf6a3b` |
| Sleet tree | `7e705f4eb5f658957bd414b4abc8207311f3c1e4` |
| AHT | `664d5ddeb568626b2f9ff66054c7091894b3b8d4` |
| websocket-ada | `765d9cd612838eb00645037977c8cc9af7171c3e` |
| Rust | 1.95.0, locked production CLI |
| Ada | GNAT 14.2.0 and 16.1.0; Alire 2.1.1; GPRbuild crate 26.0.1 |
| GNATCOLL / independent probe | 26.0.0 / websockets 15.0.1 |

The complete authentic schema closure is the root plus its SecurityMarkings
include. Both original hashes are checked before use. Sleet is a Task106-owned
clean clone built with its own lockfile. Neither broker, authorization rules,
schema, generator, model/codec runtime nor WebSocket production sources change.

## Phase A: combined production generation before consumer development

The portable contract at
`/home/zboll/git/ams-gra-codegen-oms-task106/tests/fixtures/task106/uci25-service.yaml`
selects one service identity, `task106-multimessage`, and three functions:

| Function | Authentic global element | Generated payload | Explicit topic |
| --- | --- | --- | --- |
| `log` | `Log` | `Programs.Oam.LogMT` | `task106-log` |
| `rdma` | `RDMA_InitializeSetup` | `Programs.Oam.RDMA_InitializeSetupMT` | `task106-rdma` |
| `survivability` | `SurvivabilityRiskLevel` | `Programs.Oam.SurvivabilityRiskLevelMT` | `task106-survivability` |

Each function has exactly one mandatory input `receive` exchange and one
mandatory output `publish` exchange. Topics deliberately differ; topic sharing
and arbitrary authorization semantics are not qualified.

Fresh production CLI `service-check --with-codec --language ada --world
closed-schema` reports three selected/renderable messages, **64/64 selected
types**, and **63/63 codec declarations READY**. `service-generate --with-codec`
emits one combined model, one Service_Codec with three encode/decode pairs, and
one Service_API with six endpoint operations. Complete model/API bytes match
generation without codec opt-in. Debug and optimized CLI outputs also match.

The combined union is intentionally NOT compared byte-for-byte with each
independent single-message model. The qualification separately regenerates the
independent selections, checks their original model/codec hashes, compares each
payload representation, then executes the unchanged Task093 Log and Task092
RDMA/Survivability consumers against the union. Shared dependencies compile once
without conflicting Ada declarations. No generated source is manually rewritten.

Both compilers check all six negative endpoint directions. A receive endpoint
must diagnose `"Publisher" not declared`; a publish endpoint must diagnose
`"Subscriber" not declared`. An unrelated compiler failure cannot satisfy a
negative direction probe.

## Phase B: independent real broker admission

Before implementing the full consumer, the Task106-owned protocol probe loaded
the complete schema into unchanged Sleet configured with exactly three allowed
topics. On **one** WebSocket subscriber connection it negotiated `owp`, sent one
INIT, validated INFO identity/version, accepted SIDs 1/2/3, and published the
unchanged original Task093 Log and Task092 RDMA/Survivability bodies verbatim.
Actual MSG global names, SIDs and JSON values matched independent goldens.

The probe interleaved all three messages, removed SID 1, then required actual
positive deliveries to SIDs 2 and 3 before removing them. Clean WebSocket and
broker exit, socket release and `shutdown complete` were observed. Evidence is
under `/home/zboll/task106-scratch/phase-b01` and the stricter INFO rerun
`/home/zboll/task106-scratch/phase-b02`. These are independent broker evidence,
explicitly **not** Ada decoder execution. Every complete campaign repeats admission.

## Single-session generated Ada ownership and dispatch

The Task106 consumer declares exactly one `WebSocket.Client` and one
`Connections.Connection`, at the same owning procedure scope as the receivers.
Three `OMS_LACAL.Typed` instances use the actual three payload types and their
distinct generated input Handler interfaces. Three bindings reference the same
Core. Each generated Subscriber supplies its message/topic identity; each
generated Publisher forwards the corresponding payload to the shared Core.

One receive pump calls `Connections.Poll` once, then passes that same Delivery
to all three bindings. Binding-local SID ownership decides dispatch. There is
no second routing table, per-type Connection, background receive task, reconnect,
TLS or thread pool. Receiver access checks and synchronous borrowed dispatch
remain the production runtime's responsibility.

The existing entropy provider owns a process-lifetime descriptor. Unlike the
single-message consumers, Task106 warms it directly with `AHT.Random.Fill`
before measuring resources; it does NOT open a warm-up WebSocket connection.
This preserves the exactly-one-Ada-transport constraint.

## Independent values, interleaving and failure isolation

The frozen corpus composes independently authored Task102–104 positive variants
and negative documents, not codec-generated expectations. All six variants of
each family are alternated, each delivered once from an external publisher and
once from its generated Ada Publisher. Typed handlers check independent JSON
values and generated model properties:

* Log: absent/empty/populated hexBinary, exact zero/high-bit octets and ordering,
  DateTime, lexical Duration, repeated OwnerProducer Choice arms and values.
* RDMA: IPv4/IPv6 discriminants, addresses and sequence order, required valid
  setup/header UUIDs and TCP provider, repeated OwnerProducer Choices.
* Survivability: ordered settings, priorities and nested allowed-risk values,
  optional exposure durations, required minima and repeated owner Choices.

Three Log hexBinary negatives and the independent malformed header DateTime
from each of the other two families must be **broker accepted and actually
delivered** before Ada reports Decode_Error. Each error captures its original
SID and global name. After EACH error, valid messages must reach ALL THREE
handlers on the same connection. Broker rejection is never called Ada decode.

The separate inherited broker corpus exercises 39 XSD, malformed/wrong-global,
authorization/topic and state/SID negatives. Failure stages are recorded
separately from codec errors. Protocol, unexpected/unknown event, handler or
failed-send errors fail the cell rather than being hidden by later counts.
Six foreign-binding and three unknown-SID unsubscribe attempts must fail before
wire traffic. Broker logs require exactly one Ada INIT, three SUB registrations
on one connection, and only the three legitimate UNSUB operations.

After removing Log, an independent subscriber witnesses its later generated
publication while the retired Ada callback count stays unchanged. Positive RDMA
and Survivability deliveries then prove survivor state, not just silence.
The authored 62-step plan derives expected callbacks **17 Log / 19 RDMA /
19 Survivability**, five codec errors and 61 witness deliveries. Counts are
asserted per step; no performance target is invented.

Original Log negative controls and separate RDMA/Survivability codec/model
probes execute under Normal/Check/Ignore. Broker-rejected documents therefore
also exercise the generated decoders independently, with contextual diagnostics
and model Constraint_Error controls preserved even when assertions are ignored.

## Matrix, evidence and acceptance

Mandatory combined matrix: UCI **2.5 only** × GNAT 14.2.0/16.1.0 ×
Normal/Check/Ignore × O0/O2 = **12 real-Sleet executions**. Each compiles and
runs the actual combined generated Ada consumer. Commands, working directories,
exit codes, stdout/stderr, compiler/builder/linker paths and hashes, Alire lock,
schema closure, generated hashes, broker identity, OWP transcripts, per-delivery
SID/global documents, per-step counters and cleanup are retained.

An initial interrupted debug CLI attempt exceeded the tool's 30-second command
limit and was not classified as a generation failure. Subsequent background
debug and release commands completed successfully. The first consumer run
completed routing/recovery but failed its cold entropy descriptor baseline;
that failed run is retained. Direct entropy warm-up corrected only test setup.
The first 12-cell development campaign passed each live cell but correctly
failed end-of-campaign input integrity because verifier development changed
source files concurrently. None of these runs substitutes for final clean-source
qualification. Final clean source and hosted results are reported externally.

Precommit local gates passed Rust 1.95 formatting, locked workspace build/check,
warnings-denied Clippy and the GNAT-required workspace suite (1,498 passed,
eight explicitly ignored, zero failed/filtered). The separate real-Sleet wrapper
then executed nine Rust broker tests and ten Python normal/optimized executions.
The source selector/provenance suite passed 267 tests (four inherited skips),
CI002 passed 30, real-UCI evidence controls passed 18, and pinned actionlint
1.7.12, layout controls and 556 split mutations passed. These are attributed to
the working-source bytes; clean committed reruns are separately recorded.

Fresh inherited Task099--105 campaigns ran in Task106-owned clean detached
checkout `/home/zboll/task106-scratch/regression-source` at the **exact parent**
commit/tree above, using a newly built byte-equivalent production CLI. Both
compiler families passed every campaign; twelve Task100--105 source-bound
bundles passed their unchanged independent verifiers. Dependency source parity
was checked against Task106, including 516 inherited files with zero changes.
These are fresh exact-parent executions, NOT historical runs relabeled as
Task106-source execution. Task105 2.6 stays a separate inherited regression.

## Additive CI and inherited policy preservation

Reusable Deep job: `ada-real-uci-multimessage-sleet`, with compiler artifacts
`task106-ada-multimessage-14` and `task106-ada-multimessage-16`. Exact-parent
successful run **41244320271** supplied measured assigned setup-to-completion
occupancy (seconds) for lanes 1..5: **10036 / 9129 / 11275 / 9904 / 10653**.
The new cells therefore use lanes **2 (GNAT14)** and **4 (GNAT16)**. Inherited
cells do not move; all five shared lanes and `queue: max` remain. Registered
runner topology expands 50→52 and Deep cells 22→24, not by removing a parent job.

Task106 owns a projection over unchanged Task105 and older policy pins. Owned
consumer/contract/corpus/verifier paths and shared generator/runtime/codec
dependencies select the new job. The mandatory aggregate resolves producer
attempts through existing CI-RETRY-001, then runs the source-bound verifier.
Missing, duplicate, expired, stale-source/wrong-attempt artifacts, incomplete
matrix/commands, changed generated/schema/broker pins, bad SID/global routing,
missing recovery/witnesses, lost events or cleanup fail closed. Synthetic
adversarial fixtures are explicitly separate from live interoperability evidence.

Inherited Task100–105 job bodies, source-bound verifiers and artifact producer
contracts remain. CI002/trusted dispatch, command-scoped exact-path Git trust
under isolated HOME, bounded hidden-file uploads, manual-only benchmark isolation,
and mandatory `pr-ci-required` remain. Main already contains CI-SELECT-003 at
`e4c73249d836114bc7585181b9557be6ae113011`; eventual ordinary integration must
preserve its actual merged policy instead of copying this stacked inventory over it.
During local qualification main also advanced to
`528397be1a0a5a7a14873e63bfd47399424a2df6` through merged PR #132 (PY-010).
Task105 remained open at the reviewed identity, so Task106 stays stacked rather
than merging an unrelated moving main. Both CI-SELECT-003 and PY-010 policies
remain reconciliation requirements, not changes to parent branches here.

This work does not qualify arbitrary UCI types/releases, shared topics, alternate
brokers, network/hardware RDMA, TLS, authentication deployment, reconnect,
background receive or lossless floating point. Readiness requires exact-source
local gates and authoritative hosted aggregate; development success is not approval.