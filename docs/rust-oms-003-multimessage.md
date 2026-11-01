# RUST-OMS-003 — heterogeneous generated Rust services

## Source and scope

Started from fetched main `f2991de209b9fd7c7de1681e79411e75afd93784`, tree
`0e712ad5477b3e7d5ce88cdb820a04a97f1bb9f8`, in the dedicated worktree
`/home/zboll/git/ams-gra-codegen-oms-rust-oms-003` on
`feature/rust-oms-003-multimessage`. This includes the supplied reference main
`f1aa82782fd0b0f2b0ace1311d7f16afe8e11d7f` and merged dependencies #147,
#157 and #159. Other PR branches were not changed.

No production runtime, generator, codec framework, transport or dispatcher
changes were necessary. Only test consumers, contracts, the existing pinned
Sleet runner and its existing Deep CI input setup change. No schema-version
mixing, hardware RDMA, TLS/authentication, reconnect or performance claim is made.

## Authentic inputs

| Input | UCI 2.5 | UCI 2.6 |
| --- | --- | --- |
| Release commit | `093610b7753944059360d3236770ab446d039556` | `78eb61b6112c8bffa40820c33124b57787fc5bd9` |
| MessageDefinitions SHA-256 | `ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27` | `af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b` |
| SecurityMarkings SHA-256 | `4a8056f1503234d423a0c2495844ab65387febced62bea58b87d4f343e9e7a0b` | `ee6e58d5db9fd80d526bc964b8244ec296d106301640d34c3cb2c5c00b710b88` |
| INIT/header schema version | `002.5.0` | `002.6.0` |

Sleet remains unmodified at `e38f61d8ce0d75c8508434a52f2ed77c69cf6a3b`,
tree `7e705f4eb5f658957bd414b4abc8207311f3c1e4`, built with its own locked
dependencies. Rust qualification uses toolchain **1.95.0**.

The existing Task102/103 pins and fetch scripts are reused. The facade build
script hashes both root and include before generation. Explicit
`AMS_GRA_REQUIRE_REAL_UCI_MULTIMESSAGE=1` requires both releases; absent inputs,
wrong hashes and missing files fail, rather than compiling diagnostic stubs.
Ordinary builds without authentic inputs retain diagnostic skip behavior;
those diagnostics are not qualification evidence.

## Combined production generation

Contracts in `/home/zboll/git/ams-gra-codegen-oms-rust-oms-003/tests/fixtures/rust-oms-003/`
select one service identity, `rust-oms-003-multimessage`. Both versions have
`log` / `Log` / `rust-oms-003-log` and
`rdma` / `RDMA_InitializeSetup` / `rust-oms-003-rdma`, each with one input
`receive` and one output `publish`.

Production `service-check` and `service-generate` with `--language rust
--world closed-schema --with-codec` were run before consumer implementation:

| Release | Selected/renderable messages | Selected/renderable types | Codec declarations | Status |
| --- | --- | --- | --- | --- |
| 2.5 | 2/2 | 55/55 | 54/54 | READY |
| 2.6 | 2/2 | 57/57 | 56/56 | READY |

No blocking diagnostics or production defects were found. Shared declarations
are emitted once: combined counts are not sums of single-message closures.
Each release has one `oam.rs`, one `ServiceCodec` implementing the existing
`OmsJsonCodec<LogMT>` **and** `OmsJsonCodec<RDMAInitializeSetupMT>` traits, and
one service API exposing both functions. Compilation explicitly checks both
trait bounds. Build-time generation without codecs proves source parity for
every model/API Rust file, removing only the expected codec-module inclusion.
The release-specific binary carrier and whitespace-constrained string types
are exercised separately; 2.6 is not a module alias of 2.5.

## Live acceptance

`real_uci_multimessage.rs` includes the same consumer cases against the two
independently generated release modules. Each release uses exactly one
`SleetRuntime<ServiceCodec>`, two generated typed subscriptions and both
generated output endpoints. The existing pinned `CalClient` is reused for an
external publisher and independent witness; no WebSocket implementation or
proxy is introduced.

Linux `/proc/self/task`, `/proc/self/fd` and `/proc/net/tcp` independently
verify one new runtime worker and one established client socket to the broker
after INIT and after both subscriptions. Adding the external clients yields
three sockets. Distinct original runtime subscription IDs are checked against
the generated-decoder errors. Separate witness SIDs identify actual `MSG`
envelopes by message and topic.

Per release the positive corpus has **18 typed deliveries**: absent, empty
and `0080FF` Log binary states, plus RDMA golden, IPv4, IPv6, reversed,
alternating and omitted-endpoint cases, each published through generated and
external endpoints. Expected JSON is authored independently from unchanged
Task093/Task092 goldens, never derived from generated encoding. Direct model
inspection checks octets, optional state, Choice discriminants, endpoint
ordering, repeated OwnerProducer alternatives, UUIDs, timestamps and schema
versions. Valid cross-topic publications are independently witnessed and
must not invoke either runtime handler.

Two decoder-negative publications are broker-admitted and witnessed as actual
`MSG`: Log `Data="0G"` and RDMA header `Timestamp="not-a-dateZ"`. Each must emit
`SubscriptionDecodeError` with the correct original SID, message name and
generated-codec diagnostic, and no invalid callback. The other family is
delivered first, then the failed family recovers on its original subscription.
Broker-level unauthorized-topic rejection is a separate assertion, not decoder
failure evidence.

Log is unsubscribed without closing the runtime. Another generated Log publish
is observed on the independent witness while the removed handler stays silent.
RDMA then delivers again on the original runtime. RDMA unsubscribe leaves the
runtime socket active; one runtime close joins the worker and removes only its
socket. Both external clients are then dropped and all broker sockets disappear.
There are **23 total valid typed deliveries per release**, including four
recovery deliveries and the post-Log-unsubscribe RDMA delivery.

## Reproduction and acceptance boundary

```bash
cd /home/zboll/git/ams-gra-codegen-oms-rust-oms-003
export AMS_GRA_UCI_2_5_ROOT="$(scripts/fetch-pinned-uci-2.5.sh /tmp/oms003-uci25)"
export AMS_GRA_UCI_2_6_ROOT="$(scripts/fetch-pinned-uci-2.6.sh /tmp/oms003-uci26)"
AMS_GRA_RUST_MULTIMESSAGE_ONLY=1 scripts/run-real-sleet-test.sh
```

The focused mode runs unchanged real Log and RDMA regressions, the independent
RDMA consumer and both combined tests using one pinned broker checkout/build.
It requires exact pass markers and exact non-ignored test totals; a skip cannot
produce acceptance. The existing Deep `real-sleet` job reuses its already-fetched
schemas and the same runner/broker build; no workflow, lane, concurrency or
artifact-gate subsystem is added.

Socket/thread proof is Linux-specific, matching the qualification environment.
This is a bounded live corpus, not proof for arbitrary UCI messages or shared
topics. Final commit/tree, executed local totals and exact-head hosted smoke
are recorded in the PR handoff rather than embedding a self-referential commit
identity in this document.