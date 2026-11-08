# TASK109 — authentic UCI 2.5 multi-message interoperability over WSS

## Scope and identities

Worktree: `/home/zboll/git/ams-gra-codegen-oms-task109`.
Branch: `feature/109-ada-real-uci-wss`.

Freshly fetched starting main is `5fb4e8c85f74ad8aed9c1b2a6096ffd757275e54`,
tree `e56a4de0f9fa52ba1e995edbd4fd1f655a2d3282`, newer than the last observed
`e62818e`. Merged baseline capabilities are Task100 (`7958c4e`), Task106
(`18e64c7`), Task107 (`2e3cd31`), and Task108 (`e62818e`).

Clean-source Task109 acceptance is attributed to implementation commit
`4b3d56f1b6d3ee12e5e7f364e4cb69676265b1ee`, tree
`f490c5eeb0cff633be77b0b8c2a20c8d8bb4c57f`. A subsequent ordinary commit adds
only this report and new evidence; final publication HEAD/tree are recorded in
the PR/delivery record to avoid self-reference.

| Immutable source | Commit | Tree |
| --- | --- | --- |
| AHT | `827b0c17dd964e935f8e88912ce0611ee92b6f16` | `d137b924a7b0fa8b71c1ee8c582749e97294335b` |
| websocket-ada | `765d9cd612838eb00645037977c8cc9af7171c3e` | `6198cf7b10346003b91b379b568f26b378654565` |
| Sleet | `e38f61d8ce0d75c8508434a52f2ed77c69cf6a3b` | `7e705f4eb5f658957bd414b4abc8207311f3c1e4` |
| Authentic UCI 2.5 | `093610b7753944059360d3236770ab446d039556` | `f811683ce81a3fcc5c306627cb43c3df5b525b54` |

All four source checkouts were verified clean. The existing general AHT worktree
had advanced to `a1d68ca`; it was **not** used as the qualification pin. A separate
Task109-owned clone was detached at the required `827b0c1`. Builds use committed
archives in isolated scratch directories, never dependency source checkouts.

The authentic schema closure is exactly the existing Task106 root and include:

* `UCI_MessageDefinitions_v2_5_0.xsd`:
  `ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27`.
* `UCI_SecurityMarkings_v2_5_0.xsd`:
  `4a8056f1503234d423a0c2495844ab65387febced62bea58b87d4f343e9e7a0b`.

The freshly built locked production generator has SHA-256
`db0ae0a09cde392d9ccc5860f835bd813fa612f009dadc6b5ca5d054caa64b25`.
Every cell requires Task106's exact six generated-source hashes, unchanged
model/API bytes with and without codecs, three messages, 64 selected types and
63 codec declarations READY. Exact hashes are retained with each cell.

## Reuse, transport and authority

No production Ada transport, generated model/codec, original consumer, Task100–108
test/evidence, broker, schema, shared IR, or language backend is changed.
PRs #184/#185, runner limits, allocation and main-only Deep CI remain untouched.

The new driver invokes the **unchanged** Task106 admission probe, corpus, and live
Sleet driver. Its executable adapter accepts the driver's plaintext loopback URL
but launches the actual Ada binary at `wss://localhost:<port>/`, with explicit
`OMS_WSS_TEST_CA`. Only the scratch consumer copy's `WS.Open` CA argument and
context import change. A lightweight test reverses that adaptation and requires
byte identity with the original consumer; no assertion or oracle is removed.
The representative plaintext regression rebuilds the original consumer source.

The test-only TLS terminator reuses Task108's MemoryBIO peer and forwards HTTP,
WebSocket and OWP bytes unchanged to Sleet's existing plaintext listener. It
splits ciphertext after seven bytes with a bounded pause and flushes pairs of
TLS application records together. Passive bidirectional captures prove the Ada
upgrade, exactly `owp`, schema/service INIT, three SUBs, generated PUBs, actual
matching MSGs, UNSUBs and Ada Close. Independent expected payloads still come
only from Task106's authored corpus, never the passive capture or codec output.

Trust and original-hostname checking are production behavior; there is no
insecure mode. Task108's newly rerun negative trust and hostname cases must fail.
The terminator is not a mutual-TLS client-authentication test.

Each live sequence preserves one Ada connection and INIT, SIDs 1/2/3 for
`Log`/`RDMA_InitializeSetup`/`SurvivabilityRiskLevel`, generated Publisher and
Subscriber operations, borrowed typed dispatch, topic authorization, ownership,
all 62 steps and all original independent values:

* 60 Ada MSG deliveries: 55 typed callbacks (`17 / 19 / 19`) and five recoverable
  codec failures, each followed by valid deliveries to all three families.
* 61 independent witness deliveries, including the retired Log publication.
* 39 broker/XSD/authorization/state rejections and nine binding ownership rejects.
* Log unsubscribe with continued RDMA/Survivability deliveries; all three final
  wire UNSUBs; no lost events; Ada `/proc/self/fd` and `/proc/self/task` baselines
  restored; consumer and Sleet exit zero with broker shutdown completion.

The separate verifier reuses Task106's full independent `verify_cell`, including
per-step counts, exact routing/global identities, all witness values, negative
stages, one INIT and broker shutdown. It additionally compares all 60 actual
Ada-side TLS MSG frames with Task106's original expectations.

## Executed matrix and regressions

Evidence root: `/home/zboll/task109-evidence` (new, separate from prior tasks).
Acceptance directories are `accept-<compiler>-<backend>`. GNAT 16/OpenSSL was
completed successfully before the other three acceptance cells were launched.

| Compiler | OpenSSL | mbedTLS |
| --- | --- | --- |
| GNAT 14.2.0 | PASS | PASS |
| GNAT 16.1.0 | PASS (executed first) | PASS |

OpenSSL negotiated TLS 1.3; mbedTLS negotiated TLS 1.2. Each OpenSSL cell
recorded 92 partial ciphertext writes and 91 paired-record batches; each mbedTLS
cell recorded 92 and 91 respectively. All four independently verified exactly
60 Ada-side MSG frames and zero consumer/broker exits.

All cells use authentic UCI **2.5**, Task106's combined contract and the complete
sequence, with `-gnat2022 -gnata -gnato -O0`. This is a four-cell qualification,
not a claim of repeating Task106's entire assertion/optimization cross-product
over WSS. Platform is Linux x86-64, glibc 2.41, OpenSSL 3.5.7 / mbedTLS 3.6.7,
Alire 2.1.1, GNATCOLL 26.0.0, GPRbuild crate 26.0.1, websockets 15.0.1.

Completed additional checks:

* Original Task106 plaintext sequence: PASS on GNAT 16/OpenSSL, same counts,
  values, isolation, negatives, witness and resource cleanup.
* Existing Task108 full safety harness: PASS freshly executed in all four
  compiler/backend cells, including its real pinned Sleet smoke. That synthetic
  smoke is additional safety evidence, **not** counted as authentic UCI evidence.
* Task107 borrowed heterogeneous dispatch: PASS both compilers, six cells each,
  64 checks per consumer and two required directionality compilation rejections.
* Existing Task099 runtime campaign: PASS GNAT 16.1.0, all 24 cells across
  namespaces, portable/lossless profiles, policies and optimization levels.
* Changed-path selector/executor: 35 tests PASS. New harness unit checks: 5 PASS.
* Inherited Task100–106 evidence tests: 47 run, 43 PASS, four external-fixture
  SKIPs. Ada packing: 19 PASS. Main lane allocation: 2 PASS.

The existing Ada PR smoke runs only the five new stdlib checks in addition to its
normal generated-consumer smoke: no UCI download, broker build or new hosted
lane. Task109-owned paths route Ada-only; this PR also edits the shared router
and its tests, so the **complete PR diff conservatively routes all existing smoke
backends**, as before. No routing exception hides shared changes.

## Nonblocking safety, cleanup and limits

The consumer retains its production `Connections.Poll` path and shared borrowed
delivery. Authentic UCI traffic exercises deliberately partial ciphertext and
paired TLS records. Detailed idle polling, Ping/Pong/Close ordering, output
progression, forced/deaf/burst backpressure, TLS-generated control output, peer
failures, receive bounds and cleanup are covered by the unchanged Task108
fixtures freshly executed against both TLS backends and both compilers.
This does **not** claim that a naturally congested Sleet session itself forced
each backpressure condition or emitted each WebSocket control frame.

Pinned Sleet exits its loop on received WebSocket Close without flushing a
reciprocal Close. The relay preserves that behavior, sends authenticated TLS EOF,
and does not invent a broker frame. Ordered reciprocal Close is tested with the
existing Task108 controlled peer. No production change was required.

Development attempts are retained separately and are not acceptance passes:
the debug generator exceeded the 300-second step budget; the first new relay
omitted `close_notify`, correctly rejected by production after all deliveries;
the next erroneously demanded a reciprocal Sleet Close; another completed live
WSS/plaintext but rejected concurrent harness edits via its input hash guard.
Those fixture defects were corrected before the clean-source acceptance runs.

`Needs_Read` remains **not runtime-covered** on either backend. No claim is made
for UCI 2.6, CA_PEM, public/system trust success, mutual TLS, other platforms,
arbitrary renegotiation, sanitizers, new allocation-fault injection, or full
main CI. Test certificates are short-lived local fixtures. Commands, streams,
exits, source hashes, dependency identities, wire captures and completion records
are retained; no private key is part of repository evidence.

Hosted smoke and publication disposition are reported for the exact final PR
head in the delivery record. No rebase, amend, force push, self-approval, merge or
auto-merge is performed.

## Reproduction

Run the following with Python providing the already-used `websockets==15.0.1`.
All path arguments must name local provisioned inputs; scratch must not exist.
Use `--plaintext` on the first `16/openssl` cell, then run the other three pairs.

```sh
python /absolute/oms/scripts/task109/qualify.py \
  --scratch /absolute/new-cell --setup /absolute/task096-setup16 \
  --compiler 16 --backend openssl \
  --generator /absolute/production-generator \
  --aht /absolute/clean-pinned-aht --codec /absolute/clean-pinned-websocket-ada \
  --sleet /absolute/clean-pinned-built-sleet \
  --uci25 /absolute/uci25/UCI_MessageDefinitions_v2_5_0.xsd --plaintext
python /absolute/oms/scripts/task109/verify.py /absolute/new-cell
```

Use the unchanged `/absolute/oms/scripts/task108/qualify.py` with the same
compiler/backend/setup/dependency/generator arguments (without `--uci25` and
`--plaintext`) to repeat detailed WSS safety. Exact executed local commands are
also embedded in `/absolute/oms/docs/evidence/task109/*.json`; raw records live
under `/home/zboll/task109-evidence`. The JSON bundles retain streams, commands,
exits, authored plans and wire evidence, with original stream SHA-256s in the
manifest. Original full scratch trees are retained outside Git, including
generated-code hashes and failed development attempts.
