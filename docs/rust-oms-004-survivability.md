# RUST-OMS-004 — three authentic Rust message families

## Source and scope

Started from fetched main `f14aa593561eda9ae5db4cc1e9522a7337403b8f`, tree
`5712209a094e47341c8e5b94966266332e815314`, on
`feature/rust-oms-004-survivability`, in the dedicated worktree
`/home/zboll/git/ams-gra-codegen-oms-rust-oms-004`.

This extends the existing application-interoperability qualification, not the
production runtime. There are no production backend, shared IR, runtime,
WebSocket, dispatcher, Ada, Python or C++ source changes, nor Sleet changes.
The original two-message RUST-OMS-003 contracts and tests remain a regression.
Only their Linux connection/worker inspection helpers move to a shared test
include; socket inode inspection additionally tracks the original connection.

## Authentic inputs

| Input | UCI 2.5 | UCI 2.6 |
| --- | --- | --- |
| Release commit | `093610b7753944059360d3236770ab446d039556` | `78eb61b6112c8bffa40820c33124b57787fc5bd9` |
| MessageDefinitions SHA-256 | `ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27` | `af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b` |
| SecurityMarkings SHA-256 | `4a8056f1503234d423a0c2495844ab65387febced62bea58b87d4f343e9e7a0b` | `ee6e58d5db9fd80d526bc964b8244ec296d106301640d34c3cb2c5c00b710b88` |
| Session/header version | `002.5.0` | `002.6.0` |

Sleet stays at `e38f61d8ce0d75c8508434a52f2ed77c69cf6a3b`, tree
`7e705f4eb5f658957bd414b4abc8207311f3c1e4`, with its own locked build.
Qualification uses Rust **1.95.0**. The existing schema fetchers and facade
build-time root/include hash checks are reused. Explicit
`AMS_GRA_REQUIRE_REAL_UCI_SURVIVABILITY=1` requires both releases. Missing
environment variables, missing files, wrong roots and wrong includes fail closed.
Ordinary builds without schemas retain diagnostic stubs, not qualification.

## Production generation and readiness

The contracts under
`/home/zboll/git/ams-gra-codegen-oms-rust-oms-004/tests/fixtures/rust-oms-004/`
extend the original contracts without changing their Log/RDMA topics. Each
function has generated input `receive` and output `publish` exchanges:

| Function | Global element | Topic |
| --- | --- | --- |
| `log` | `Log` | `rust-oms-003-log` |
| `rdma` | `RDMA_InitializeSetup` | `rust-oms-003-rdma` |
| `survivability` | `SurvivabilityRiskLevel` | `rust-oms-004-survivability` |

Actual production `service-check` and `service-generate` with `--language rust
--world closed-schema --with-codec` report:

| Release | Selected/renderable messages | Selected/renderable types | Codec declarations | Status |
| --- | --- | --- | --- | --- |
| 2.5 | 3/3 | 64/64 | 63/63 | READY |
| 2.6 | 3/3 | 65/65 | 64/64 | READY |

These are measured Rust counts, not Ada inventory copied forward. Each release
has one combined `oam.rs`, service API and `ServiceCodec`, implementing all of:

```rust
OmsJsonCodec<model::LogMT>
OmsJsonCodec<model::RDMAInitializeSetupMT>
OmsJsonCodec<model::SurvivabilityRiskLevelMT>
```

The test consumer compiles all three bounds. The build script requires exactly
three renderable messages and READY model/codec status, generates with and
without codecs, and compares every model/API Rust source byte-for-byte after
removing only the expected codec-module declaration. No backend fix was needed.
UCI 2.6 is generated independently, not aliased to 2.5.

## Authentic model and independent corpus

The JSON corpora are value-exact snapshots of unchanged
`/home/zboll/git/ams-gra-codegen-oms-rust-oms-004/scripts/task104/cases.py` and
`/home/zboll/git/ams-gra-codegen-oms-rust-oms-004/scripts/task105/cases.py`.
The runner checks equality against those independently authored sources before
acceptance; expectations never come from generated encoding. Both original
Task092/105 goldens remain unchanged.

Direct generated-model inspection covers ordered RiskSetting records (1..4),
ordered RiskLevel records (1..5), all priorities and AllowedRisk values, optional
exposure records with required composite/cumulative Duration values, repeated
optional system IDs and repeated OwnerProducer Choices. RiskSetting and
RiskLevel are records, never invented Choices. Additional positive cases cover
interior cardinalities (three settings and 2/3/4 levels), repeated system IDs and
lexically preserved durations. Empty optional sequences encode by omission.
Constructor/codec negatives cover UUID/version checks, cardinality bounds and
both missing required exposure members.

The UCI 2.6 maximum corpus exercises named Zulu DateTime security fields,
nonempty whitespace-visible strings, preserved ClassifiedBy whitespace and
SAR identifiers. Compile-time field annotations distinguish 2.5
`XmlSchemaDateTime` security members from 2.6 `DateTimeType`. Offset text is
accepted by the unconstrained 2.5 carrier and rejected by the named 2.6 carrier.
Whitespace tests distinguish preserved string text from collapsed DateTime and
Duration text in both model construction and codec execution.

## Single connection and failure boundaries

Each release's live test creates one `SleetRuntime<ServiceCodec>`, registers
three generated typed subscribers, and checks distinct original SIDs. Linux
socket ownership and worker inspection establish one runtime connection and
one worker before external observers connect. Every typed callback records its
actual Rust thread ID; all callbacks must use that same original worker.
Original socket inodes must survive recovery and unsubscribe unchanged.

The generated publishers and an independent pinned `CalClient` publisher
interleave all three families. A separate pinned `CalClient` witness checks
actual broker-routed OWP `MSG` envelopes, witness SIDs and independently authored
JSON. All six wrong-topic pairings are witnessed without typed callbacks.

For each family, one broker-admitted decoder negative is witnessed: Log
hexBinary `0G`, RDMA header `not-a-dateZ`, and Survivability header
`not-a-dateZ`. Each must produce `SubscriptionDecodeError` with its original
SID, exact message name and contextual field/codec diagnostic, with no invalid
typed callback. Both unaffected families deliver first, then the failed family
recovers. All original subscriptions, worker identity and runtime socket remain.

Separately, the complete Task104/105 risk-negative corpus must be rejected by
the broker with the expected category, no witness delivery, no runtime decoder
event and no typed callback. This includes invalid priority/AllowedRisk,
missing required members, under/over cardinality and malformed OwnerProducer
Choices. Broker rejection is never counted as Rust decoder execution.

Log unsubscribe is followed by a generated Log publish: the independent Log
witness must receive it while the removed callback remains silent. Original
RDMA and Survivability subscribers then deliver. RDMA unsubscribe leaves the
original Survivability subscriber functional. Last, Survivability unsubscribes
and one runtime close closes its socket and successfully joins its worker.
External clients close and the broker process/configuration are reaped/removed.
The existing TLS-teardown-barrier lifecycle regressions prove actual joining;
there is no immediate procfs task-disappearance assertion.

The asserted valid delivery totals per release are **Log 9, RDMA 16,
Survivability 23** (48 total). These include nine recovery deliveries and three
post-unsubscribe survivor deliveries. Wire-only wrong-topic/removed-subscription
observations and rejected messages are not counted as typed deliveries.

## Reproduction and CI boundary

```bash
cd /home/zboll/git/ams-gra-codegen-oms-rust-oms-004
export AMS_GRA_UCI_2_5_ROOT="$(scripts/fetch-pinned-uci-2.5.sh /tmp/oms004-uci25)"
export AMS_GRA_UCI_2_6_ROOT="$(scripts/fetch-pinned-uci-2.6.sh /tmp/oms004-uci26)"
AMS_GRA_RUST_SURVIVABILITY_ONLY=1 scripts/run-real-sleet-test.sh
```

The explicit mode reuses one clean pinned Sleet checkout/build, runs existing
RUST-OMS-001/002/003 and external RDMA-consumer regressions, both releases'
three-message tests and the existing lifecycle tests. Exact markers and
non-ignored pass totals are mandatory. Default invocation with both schemas
also includes this qualification, so the existing main-only Deep `real-sleet`
job needs no new job, schema fetch, Sleet build, lane or workflow infrastructure.
Complete hosted qualification remains main-only under CI-RESET-001; PRs retain
the existing lightweight `PR smoke` boundary.

Final source/tree, executed validation totals and exact-head hosted smoke are
reported in the PR handoff rather than embedded self-referential identities.
Limitations remain Linux-specific connection evidence, bounded authentic
corpora, one release per session, no hardware RDMA, TLS, reconnect or performance
claims. Approval, auto-merge and merge are outside delivery.