# Task110 — authentic UCI 2.6 heterogeneous Ada over verified WSS

## Baseline and scope

Worktree: `/home/zboll/git/ams-gra-codegen-oms-task110`.
Branch: `feature/110-ada-uci26-multimessage-wss`.
Freshly fetched starting main: `1c48bcbff22535ef92b870d7a12ef6be7fdbf90b`,
tree `7059845473ad84876760a38995c838341a04bd92`. This is the merge of Task109
PR #188. No historical qualification evidence is modified.

The production generator, Ada transport/runtime/codec support, shared IR and
other language backends are unchanged. No dependency is upgraded or patched.
Task110 owns its combined contract, independent corpus, admission probe, live
Sleet driver, verifier and dedicated Ada consumer. The latter is Task106's
consumer with only release/service identity, explicit CA input, named binary
carrier and additional UCI 2.6 security checks. An offline test removes precisely
those authorized differences and requires byte equality with the original.

The shared Task109 relay has two opt-in expected-INIT arguments, defaulting to
the original UCI 2.5 schema/service. Forwarding, TLS, fragmentation, WebSocket
handling and Close behavior are unchanged. Tests exercise both defaults and
UCI 2.6 opt-in, including rejection of mismatched identities. The unchanged
Task109 campaign is the live release-regression control.

## Immutable authorities

| Source | Commit |
| --- | --- |
| AHT | `827b0c17dd964e935f8e88912ce0611ee92b6f16` |
| websocket-ada | `765d9cd612838eb00645037977c8cc9af7171c3e` |
| Sleet | `e38f61d8ce0d75c8508434a52f2ed77c69cf6a3b` |
| UCI 2.6 release | `78eb61b6112c8bffa40820c33124b57787fc5bd9` |

The release is fetched with the existing pinned UCI fetch script; source commit,
tree and cleanliness are recorded. The authentic two-document closure is:

* `UCI_MessageDefinitions_v2_6_0.xsd`:
  `af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b`.
* `UCI_SecurityMarkings_v2_6_0.xsd`:
  `ee6e58d5db9fd80d526bc964b8244ec296d106301640d34c3cb2c5c00b710b88`.

Builds use committed dependency archives in isolated scratch directories, not
dependency source trees. Python uses the inherited `websockets==15.0.1`.
The contract retains OMS version `2.5` while explicitly selecting UCI schema
version `2.6`, as do the independently qualified Task102/103/105 contracts.
Session and payload schema versions are strictly `002.6.0`.

## Readiness and independent admission gates

Before consumer adaptation the unchanged, freshly built locked release generator
reported **3/3 messages, 65/65 selected types, 64/64 codec declarations READY**.
These are freshly measured UCI 2.6 counts, not inherited UCI 2.5 counts. Model/API
files generated with and without `--with-codec` are byte-identical. Six generated
source hashes are pinned in `/home/zboll/git/ams-gra-codegen-oms-task110/scripts/task110/qualify.py`
and recorded with the execution evidence.

Independent single-family UCI 2.6 services are regenerated from the unchanged
Task102/103/105 contracts. All their non-discriminated record representations
(41 Log, 44 RDMA, 41 Survivability, including nested records) occur unchanged in
the combined model. Each family's full generated-file hashes and individual
record hashes are retained. Task105's five pre-existing baseline source hashes
are additionally required exactly. Whole combined files are not falsely compared
for equality with smaller single-family selections.

The independent admission probe uses no Ada codec. It wraps verbatim the Task093
Log26 and Task092 RDMA26 fixtures and Task105's authentic Survivability26 fixture.
It observed one `owp` connection, one INIT, correct INFO, three acknowledged
subscriptions with distinct SIDs/topics, eight ordered actual deliveries,
retired-Log isolation, surviving RDMA/Survivability delivery, three successful
unsubscribes and clean broker shutdown. The gate is repeated in every cell.

## Authored heterogeneous corpus

Task110 imports only `positives('26')`, `decoder_invalid('26')`, goldens and
broker-negative cases from Tasks102, 103 and **105**, never Task104's UCI 2.5
Survivability corpus. It retains Task106's 62-step structure:

* Six variants per family, each delivered externally and through the generated
  Publisher, interleaved across all three typed Subscribers.
* Three invalid Log hexBinary inputs and one invalid timestamp per other family;
  each broker-accepted decoder error is followed by recovery to all three types.
* Log unsubscribe, witnessed publication to the retired topic, then external and
  generated publications to both surviving families.

Counts derive from the authored plan: **17 Log + 19 RDMA + 19 Survivability = 55
typed callbacks**, five decoder errors, 60 deliveries to Ada, 61 independent
witness deliveries (including the retired topic), and nine SID ownership
rejections. Separately, 48 broker/XSD or OWP authorization/state rejections are
checked with their independent expected diagnostics. They are not Ada decoder
rejections.

The UCI 2.6 Log binary field uses `HexBinaryType.Value`, not the UCI 2.5 anonymous
vector. Security assertions require named UTC `DeclassDate` and
`CUI_DecontrolDate`, exact whitespace-preserved `ClassifiedBy` value
`"  Task105  authority  "`, and the constrained `SAR_Identifier` carrier/value.
Negative security cases include empty authority/SAR strings and offset spellings
of the named dates. Pinned Sleet rejects these before delivery; only the timestamp
and hexBinary negatives are classified as recoverable live decoder errors.

## TLS and behavioral boundaries

The actual Ada binary connects to `wss://localhost:<port>/`, with explicit
`OMS_WSS_TEST_CA`, certificate verification and original-hostname verification.
The reused Task109 relay uses Task108's MemoryBIO TLS peer, forwards unchanged
HTTP/WebSocket/OWP bytes to pinned Sleet and records real TLS negotiation,
fragmented encrypted writes, paired TLS records and both plaintext wire streams
at the TLS endpoint. The passive captures are observations, never value oracles.
No insecure mode, mock broker or fabricated reciprocal Close is used.

Pinned Sleet closes its loop without flushing a reciprocal WebSocket Close. The
relay records Ada's actual Close and supplies authenticated TLS EOF, preserving
this existing limitation. Ordered reciprocal Close is covered only by Task108's
controlled peer. `Needs_Read` remains **not runtime-covered**. Fixture-forced
backpressure is not claimed as natural Sleet congestion. No reconnect, scheduling,
thread-pool, authorization, TLS API or codec feature is introduced. UCI 2.5 and
UCI 2.6 payloads are not interchangeable.

## Evidence and reproduction

### Observed qualification results

All acceptance cells ran on clean implementation commit
`65eaeca07af5acc7252aefa1da6365e04bd2ce1a`, tree
`1ded5a7e02472ee96e0eabd4741ce0297607334a`. The report/evidence commit does not
change any qualification input. The locked production generator SHA-256 is
`9bf13020e1cb970da73c8004c3ff417ad6bb71533336cf122feeda90ff7c039f`.

| Compiler | OpenSSL | mbedTLS |
| --- | --- | --- |
| GNAT 14.2.0 | **PASS**, TLS 1.3 | **PASS**, TLS 1.2 |
| GNAT 16.1.0 | **PASS**, TLS 1.3, first vertical | **PASS**, TLS 1.2 |

Every cell produced the exact authored counts above, independently reverified
from the callback transcript, full step progression, broker registrations/logs,
independent witness wire and actual TLS-endpoint WebSocket captures. Each relay
recorded 93 partial encrypted writes and 92 paired-record batches; both consumer
and broker exited zero and cleanup passed. No error was hidden as a positive
delivery. Generated source hashes matched freshly measured UCI 2.6 pins in all
four cells.

The unchanged Task109 GNAT16/OpenSSL UCI 2.5 control passed **both WSS and
plaintext**: the original 62 steps, 17/19/19 callbacks, five decoder errors,
39 broker/state rejections and 61 witness deliveries in each transport, with
60 actual Ada WSS deliveries. Original UCI 2.5 generated hashes are still exact.

Existing Task108 GNAT16/OpenSSL and GNAT16/mbedTLS safety campaigns both passed,
including untrusted CA/hostname/truncation controls, poll/fragment/record/control
output, bounded fixture backpressure, transport regressions and real Sleet.
Task110 does not duplicate that safety system or claim a fresh GNAT14 safety
campaign beyond its two full authentic UCI 2.6 matrix cells.

Offline validation: **12/12 new harness tests**, **36/36 selector tests**, and
**5/5 unchanged Task109 helper tests** passed. The inherited Task101–106 helper
suite ran 52 tests with five explicitly opt-in disposable-UID ownership fixtures
skipped (47 passed); these skips are not qualification passes. `git diff --check`
passed. Early diagnostic runs and the scratch-only mbedTLS fixture-path error
are classified separately in the evidence manifest, never as acceptance.

Hosted smoke and final publication identity must be read from the final PR head
and delivery record, not inferred from these local passes.

Full commands, streams, exits, generated sources, schemas, plans, consumer/broker
logs, TLS endpoint captures and failed development attempts are external under
`/home/zboll/task110-evidence`. Concise source-backed receipts and artifact hashes
are committed under `/home/zboll/git/ams-gra-codegen-oms-task110/docs/evidence/task110`.
No private key is committed. Clean-source campaign identity is separate from the
later report-only commit; final publication HEAD/tree and hosted smoke status are
reported in the PR/delivery record to avoid self-reference.

Provision exact GNAT 14.2.0 and 16.1.0 setups with the unchanged
`/home/zboll/git/ams-gra-codegen-oms-task110/scripts/task096/setup.py`. Run
GNAT16/OpenSSL first, then GNAT16/mbedTLS, GNAT14/OpenSSL and GNAT14/mbedTLS:

```sh
/absolute/python /absolute/oms/scripts/task110/qualify.py \
  --scratch /absolute/new-cell --setup /absolute/setup16 \
  --compiler 16 --backend openssl --generator /absolute/production-generator \
  --aht /absolute/pinned-aht --codec /absolute/pinned-websocket-ada \
  --sleet /absolute/pinned-built-sleet --uci-release /absolute/uci26-release \
  --uci26 /absolute/uci26.extracted/UCI_MessageDefinitions_v2_6_0.xsd
/absolute/python /absolute/oms/scripts/task110/verify.py /absolute/new-cell
```

The existing PR selector routes Task110-owned helpers/fixtures to Ada only. Since
this PR necessarily also edits the shared selector and its tests, the **complete
PR diff conservatively selects all existing backend smoke suites**. The new
offline tests run in the Ada smoke; no full hosted Sleet matrix or changes to
main-only Deep CI, runner limits or lane allocation are added.