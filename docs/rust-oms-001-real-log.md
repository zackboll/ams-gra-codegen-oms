# RUST-OMS-001 — real UCI Log through the existing Rust runtime

## Scope and readiness

Base: current main `070a9f8b8d40b7e8a48c093af655ca882034f987`.
Branch: `feature/rust-oms-001-real-log`. Final commit/tree identities belong in
the PR handoff rather than this self-referential source document.

Production `service-check --language rust --world closed-schema --with-codec`
on the **unchanged Task102 contracts and authentic complete schemas** reports:

| Release | Selected message | Selected/renderable types | Codec declarations | Result |
| --- | --- | --- | --- | --- |
| UCI 2.5 | `Log`, payload `LogMT` | 42/42 | 41/41 | READY, no blockers |
| UCI 2.6 | `Log`, payload `LogMT` | 44/44 | 43/43 | READY, no blockers |

No production backend or runtime fix was necessary. The existing
`service_codec.rs`, `service_api.rs`, `OmsJsonCodec`, `SleetRuntime`, global
element envelope and pinned `sleet-client` are reused without modification.
No UCI schemas or generated model bytes are committed.

The harness generates each version independently, with and without codecs.
It requires byte-identical Rust model source and API source identical except
for the sibling codec module declaration. Separate `real_uci_log25` and
`real_uci_log26` modules retain genuine differences: 2.5 has
`Option<Vec<u8>>` and direct `XmlSchemaDuration`; 2.6 has
`Option<HexBinaryType>` and named `DurationType`. Neither model is aliased to
the other. Runtime INIT and message headers use `002.5.0` / `002.6.0` respectively.

The 41 emitted UCI 2.5 declarations are `AtomicEnergyMarkingsEnum`,
`CUIBasicEnum`, `CUISpecifiedEnum`, `CapabilityIDType`, `ClassificationEnum`,
`ComponentIDType`, `DateTimeType`, `DeclassExceptionEnum`,
`DisseminationControlsEnum`, `FGISourceOpenChoiceType`, `FGISourceOpenEnum`,
`FGISourceProtectedChoiceType`, `FGISourceProtectedEnum`, `HeaderType`,
`HighWaterNATOEnum`, `IDType`, `LogMDT`, `LogMT`, `LogSeverityEnum`,
`MessageModeEnum`, `MissionIDType`, `NATOSpecialWordsType`, `NonICMarkingsEnum`,
`NonUSControlsEnum`, `OwnerProducerChoiceType`, `OwnerProducerEnum`,
`ReleasableToChoiceType`, `ReleasableToEnum`, `SCIControlsEnum`,
`SecondBannerLineEnum`, `SecurityInformationType`, `ServiceIDType`,
`SubsystemIDType`, `SystemIDType`, `UCISchemaVersionStringType`,
`UniversallyUniqueIdentifierType`, `VersionedIDType`, `VisibleString1024Type`,
`VisibleString256Type`, `WhitespaceVisibleString1024Type` and
`WhitespaceVisibleString4096Type`. UCI 2.6 emits those names plus `DurationType`
and `HexBinaryType`, each resolved from its own release. The selected closure
also includes the ancestry-only `MessageType`; generated Log fields flatten it.

## Pinned inputs

Use the Task102 source pins, obtained by the existing fetch scripts:

| Input | Revision / SHA-256 |
| --- | --- |
| UCI 2.5 release | `093610b7753944059360d3236770ab446d039556` |
| 2.5 MessageDefinitions | `ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27` |
| 2.5 SecurityMarkings | `4a8056f1503234d423a0c2495844ab65387febced62bea58b87d4f343e9e7a0b` |
| UCI 2.6 release | `78eb61b6112c8bffa40820c33124b57787fc5bd9` |
| 2.6 MessageDefinitions | `af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b` |
| 2.6 SecurityMarkings | `ee6e58d5db9fd80d526bc964b8244ec296d106301640d34c3cb2c5c00b710b88` |
| Sleet revision | `e38f61d8ce0d75c8508434a52f2ed77c69cf6a3b` |
| Sleet tree | `7e705f4eb5f658957bd414b4abc8207311f3c1e4` |
| Rust compiler | 1.95.0 |

The build script verifies **both root and include hashes** before generation.
An incorrectly pinned configured input always fails; ordinary builds without
UCI compile explicit skip stubs. `AMS_GRA_REQUIRE_REAL_UCI_LOG=1` makes either
missing root a hard build failure. The focused runner requires both roots
before obtaining/building Sleet, verifies the clean pinned checkout, builds
with Sleet's own locked dependencies, and requires all four tests and all
four version-specific pass markers. A skip cannot satisfy qualification.

## Consumer and transport regression

The external integration test `real_uci_log.rs` consumes generated modules
through the facade crate exactly as a Rust application does:

```rust,ignore
use ams_gra_oms_runtime_rust::{OmsJsonCodec, RuntimeConfig, SleetRuntime};
use ams_gra_oms_runtime_rust_facade_tests::real_uci_log25::{
    model::LogMT, service_api::function_log as log, service_codec::ServiceCodec,
};

fn consume(url: &str, body: &serde_json::Value) {
    let value: LogMT = ServiceCodec.decode_payload(body).unwrap();
    let mut runtime = SleetRuntime::connect(
        RuntimeConfig::new(url, "task102-log", "002.5.0"), ServiceCodec,
    ).unwrap();
    let (tx, rx) = std::sync::mpsc::channel();
    let subscription = log::exchange_receive::subscribe(
        &mut runtime, move |value: &LogMT| { tx.send(value.clone()).unwrap(); },
    ).unwrap();
    log::exchange_publish::publish(&mut runtime, &value).unwrap();
    let received = rx.recv_timeout(std::time::Duration::from_secs(10)).unwrap();
    assert_eq!(ServiceCodec.encode_payload(&received).unwrap(), *body);
    subscription.unsubscribe().unwrap();
    runtime.close().unwrap();
}
```

For each release, the regression decodes the existing Task093 golden and
independently inspects classification, both repeated `OwnerProducer` choices,
UUID, header timestamp/schema/mode, data timestamp, duration lexical text,
severity, label and details. It constructs absent, empty and populated binary
states through the generated model; JSON is compared against the independently
authored golden with explicit absent/empty/`0080FF` expectations. Lowercase input
also encodes canonically. No expected document is obtained from the decoder.

Each transport test starts unmodified Sleet with the matching full schema.
Generated input SUB and output PUB use **`Log`, not `LogMT`**. A separate pinned
`CalClient` observer receives and compares actual OWP `MSG` payloads, while
the generated decoder independently invokes a typed callback. All three binary
states cross Sleet. The test proves UNSUB stops callbacks and closes the runtime
worker; child-server cleanup runs even on test panic.

## Observed negative boundaries (both versions)

| Input | Pinned boundary | Typed delivery |
| --- | --- | --- |
| Odd hex `0` | Sleet delivers MSG; generated decoder rejects odd length | None |
| Invalid digits `0G` | Sleet delivers MSG; generated decoder rejects digits | None |
| Internal whitespace `00 80 FF` | Sleet delivers MSG; generated decoder rejects lexical form | None |
| Array instead of binary JSON string | Sleet `Invalid-Message`, occurrence mismatch | None |
| Missing Timestamp, Severity, Label or Details | Sleet `Invalid-Message`, missing required member | None |
| Missing MessageHeader, SecurityInformation or MessageData | Sleet `Invalid-Message`, missing required member | None |
| Global element `LogMT` | Sleet `Invalid-Message`, unknown global element | None |
| Topic containing spaces | Pinned client `InvalidInput` before send | None |
| Unauthorized PUB / SUB topic | Sleet `Illegal-State`, not authorized | None |

The negative publisher is the existing pinned `CalClient` with verbose status,
not an alternate WebSocket implementation or replacement codec. Every malformed
hex MSG is also observed in raw form; the runtime must emit
`RuntimeEvent::SubscriptionDecodeError` containing `MessageDecodeError::Codec`
with the Data path, never invoke the handler.
Every broker rejection must be an observable `ClientError::Server`, with no MSG
and no callback. After **every document case**, a valid generated Log must
arrive on the same still-active subscription. A final valid delivery also
checks recovery after topic errors. Validation is never weakened to match
Sleet's incomplete hexBinary lexical checks.

## Reproduce qualification

From the repository root (inputs/checkouts remain outside it):

```bash
export AMS_GRA_UCI_2_5_ROOT="$(scripts/fetch-pinned-uci-2.5.sh /tmp/rust-log-uci25)"
export AMS_GRA_UCI_2_6_ROOT="$(scripts/fetch-pinned-uci-2.6.sh /tmp/rust-log-uci26)"
AMS_GRA_RUST_LOG_ONLY=1 bash scripts/run-real-sleet-test.sh
```

An optional existing clean Sleet checkout can be passed as the script's first
argument. The script builds and verifies the same pinned revision in either
case. Optimizing build dependencies makes repeated full-schema generation
practical; it changes no generator semantics.

The existing main-only Deep `real-sleet` job now fetches both roots, requires
real Log generation, pins Rust 1.95.0 and runs these cases alongside its existing
qualifications. There is no new job, PR Deep matrix, artifact acceptance gate,
selector policy or runner restructuring. Existing lightweight PR smoke remains
the PR check. Full hosted qualification happens on main after independent review
and user merge, not as part of this handoff.

This qualifies these two closed-schema Log services, not arbitrary messages,
open-world dispatch, TLS/authentication deployments or other broker revisions.
Local results and final hosted PR smoke status are recorded in the PR handoff;
no main-after-merge result is claimed before merge.

## Local qualification results

Rust 1.95.0, authentic Task102 roots/includes and clean pinned Sleet:

| Check | Result |
| --- | --- |
| Production readiness / generation, both releases | Passed; model/API parity verified |
| Separate out-of-workspace Cargo consumer of actual generated files | Compiled and ran; both goldens matched |
| Focused Rust backend, runtime API and runtime tests | 110 passed |
| Explicit pinned-Sleet Log runner | 4 passed, 0 failed, 0 ignored; both release markers present |
| Facade suite with both authentic roots | 105 passed, 2 existing ignored |
| Facade suite with real Sleet configured | 102 passed, 2 existing ignored |
| Ordinary no-UCI workspace tests | 1502 passed, 8 existing ignored; includes explicit real-input skip stubs |
| Workspace all-targets check | Passed |
| Workspace all-targets Clippy, `-D warnings` | Passed |
| Authentic-root facade all-targets release Clippy, `-D warnings` | Passed |
| Formatting (including shared included test source), diff whitespace | Passed |
| Existing Python, Rust and Ada PR smoke suites | Passed |
| Existing smoke router regression tests / workflow actionlint | Passed |
| Missing root, wrong root hash, wrong include hash probes | All hard failures, never skips |

Local evidence is retained outside the source at
`/home/zboll/rust-oms-001-evidence`, including production readiness reports,
generated source pairs, broker/decoder diagnostics and recovery markers,
Sleet locked build output, external consumer source/output and test logs.
These are local execution results, not a new CI artifact gate or a claim of
hosted main qualification. Existing ignored tests were not reclassified as
passed; the explicit four-case Log qualification has no ignored/skipped tests.