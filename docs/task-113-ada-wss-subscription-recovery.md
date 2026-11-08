# TASK113 — pending SUB/UNSUB and application-owned WSS recovery

## Baseline and scope

Freshly fetched `origin/main`: `a6a0f575ccdb6a850205d3e93e6fbd21823dfeaa`,
tree `a31c2e4305464701d01610209eb00766999ad3e2`, including Task112 PR #194.
Worktree: `/home/zboll/git/ams-gra-codegen-oms-task113`; branch:
`feature/113-ada-wss-subscription-recovery`.

**Zero production changes.** No reconnect/replay, protocol, generated model/codec,
dependency, schema, language backend, shared IR, runner, or main-only Deep CI
policy changes. Task112's historical consumer and evidence remain unchanged.
Small optional profile hooks in its builder, controller, and verifier preserve
defaults and avoid copying the campaign. The existing PR smoke selector classifies
`/scripts/task113/` as Ada-owned; no new runner lane is introduced.

## Application lifetime and production state

`/tests/ada-wss-subscription-recovery/recovery_consumer.adb` owns five lexical
epochs in one Ada process: warm-up, pending SUB/reset, fresh recovery, pending
UNSUB/reset, fresh recovery. Each invocation creates a new WebSocket client,
connection, typed bindings and receivers. Bindings finalize before receivers,
connection and transport. No borrowed delivery escapes its epoch; no heap owner,
unchecked access or reconnect machinery is introduced.

SUB accepts Log and RDMA, then issues the third subscription through the generated
Survivability Subscriber API. It requires SID 3, `Busy=True`, SID 3 inactive,
SIDs 1 and 2 active, and three accepted-command events (INIT and two SUBs).

UNSUB accepts all three subscriptions and exchanges authentic messages before
calling the existing Survivability binding's `Unsubscribe`. SID 3 must already
be inactive, `Busy=True`, SIDs 1 and 2 active, with seven accepted commands
(INIT, three SUBs, three PUBs). A retained authentic delivery cannot invoke the
unsubscribed receiver. Cross-binding UNSUB authority is rejected without sending.

Every retired epoch requires `Closed`, no outstanding command, all old identities
inactive, zero lost events and exactly one terminal event. Repeated polling/Close,
dispatch of a retained authentic delivery and explicit binding Close cannot add
callbacks or ACKs. The lexical master prevents callbacks into finalized receivers.
Numeric SIDs 1,2,3 are reused only in new epoch bindings.

Each fresh normal epoch verifies TLS with `CA_Path`, negotiates `owp`, receives
genuine valid INFO, explicitly accepts three SUBs, publishes all three families
using generated APIs, receives independent external publications and compares all
decoded values against authentic independent fixtures. Each has six callbacks.
Timestamp is an existing UCI field identifying epoch/phase, not a wire extension.

## Broker processing versus client knowledge

`/scripts/task113/recovery.py` reuses Task112's persistent independent observer,
single unmodified pinned Sleet process, Task108 TLS MemoryBIO peer, Task109 wire
parser, and Task110/106 fixture infrastructure. It never manufactures ACKs or
successful responses. At the observed third SUB or `UNSUB 3` boundary it withholds
broker responses, captures the real fourth/eighth `+OK`, then resets only after
that complete ACK and Ada's pending-state/ARMED observations.

Full captures distinguish Ada-sent, Sleet-sent and Ada-delivered bytes. Independent
Sleet registration/removal logs identify the broker connection and processed
command. Ada does not read these logs. The harness therefore knows the commands
were processed; Ada has no ACK and must treat the remote outcome as uncertain.
Transport recovery does not resolve application-level command ambiguity. New
subscriptions are explicit application actions, never retries of old commands.
Exact ordered command lists reject replay, duplicate SUB/UNSUB or unintended PUB.

### Separately reported extension: failure before broker receipt

**Not exercised.** The inherited relay forwards incoming plaintext immediately.
Observing complete commands before forwarding would need a new frame-buffering
interception path, including partial/coalesced frames. This is deliberately not
folded into the focused ACK-withholding relay. No absence-of-processing inference
is made from missing ACKs. A future extension must capture ingress versus forwarded
commands, terminate upstream at the boundary and independently verify broker
non-receipt before claiming a command did not execute remotely.

## Immutable inputs and records

| Dependency | Commit |
| --- | --- |
| AHT | `827b0c17dd964e935f8e88912ce0611ee92b6f16` |
| websocket-ada | `765d9cd612838eb00645037977c8cc9af7171c3e` |
| Sleet | `e38f61d8ce0d75c8508434a52f2ed77c69cf6a3b` |
| UCI 2.5 | `093610b7753944059360d3236770ab446d039556` |
| UCI 2.6 | `78eb61b6112c8bffa40820c33124b57787fc5bd9` |

The builder checks identities, clean dependencies, authentic schema hashes and
exact historical generated-source hashes; it builds isolated copies. Full records
remain outside Git at `/home/zboll/task113-evidence/`: commands, stdout/stderr/exits,
schemas, generated sources, compiler/platform/lock identities, broker logs,
observer transcript, wire bytes, application observations and SHA256 manifest.
Acceptance requires clean source and unchanged inputs throughout. Diagnostic
runs are not acceptance evidence. Compact receipts belong in
`/docs/evidence/task113/`.

Example qualification and independent verification:

```sh
/home/zboll/task110-evidence/venv/bin/python \
  /home/zboll/git/ams-gra-codegen-oms-task113/scripts/task113/qualify.py \
  --scratch /home/zboll/task113-evidence/accept16-openssl \
  --setup /home/zboll/task110-evidence/setup16 \
  --deps /home/zboll/task110-evidence/deps \
  --schema25 /home/zboll/task112-evidence/accept16-openssl/uci25 \
  --schema26 /home/zboll/task112-evidence/accept16-openssl/uci26 \
  --generator /home/zboll/task112-evidence/target/release/ams-gra-codegen-oms \
  --compiler 16 --backend openssl --release 26 25
/home/zboll/task110-evidence/venv/bin/python \
  /home/zboll/git/ams-gra-codegen-oms-task113/scripts/task113/verify.py \
  /home/zboll/task113-evidence/accept16-openssl \
  --receipt /home/zboll/git/ams-gra-codegen-oms-task113/docs/evidence/task113/16-openssl.json
```

The verifier rehashes artifacts, reparses raw captures, checks source/dependency
identities, exact commands, authentic payloads, independent broker processing,
application ACK/callback counts, terminal events and resources. Unit-only fake
captures and deliberate evidence mutations are never integration evidence.
Monotonic deadlines bound waits; polling intervals are not processing evidence.
Linux FD/task baselines are warmed by the first TLS epoch and required after
every later retirement.

## Qualification results

All acceptance cells ran clean source
`d5ac16fbded173ab06e5815601c51d980d3f2ab9`, tree
`60d96cfa0ea9f458102ae72dafff607d4d69f67d`. The later evidence/documentation
commit does not change any qualified input; all input hashes are rechecked
against the delivery checkout. GNAT16/OpenSSL ran first.

| UCI 2.6 compiler | OpenSSL | mbedTLS |
| --- | --- | --- |
| GNAT 14.2.0 | PASS | PASS |
| GNAT 16.1.0 | PASS | PASS |

Original UCI 2.5 (original schema/model and Task102/103/104 fixtures):
**PASS**, GNAT16/OpenSSL, the same five-epoch sequence. UCI 2.6 uses independent
Task102/103/105 fixtures. Every full artifact set passes independent verification.

Per release/cell: five verified WSS sessions, five genuine INIT/INFO exchanges,
15 broker-registered subscriptions on distinct Ada connections, 24 authentic
Sleet deliveries/callbacks, 12 acknowledged generated Ada publications, 12
independent external publications, and five successful cleanup receipts.
One persistent observer uses broker connection 1; Ada connections 2..6 identify
epochs. The single broker survives each campaign and exits gracefully with code 0.

| Epoch | State before retirement | Ada ACKs | Callbacks Log/RDMA/Risk | Result |
| --- | --- | --- | --- | --- |
| 1 warm-up | three accepted SUBs, three PUBs | 7 | 2/2/2 | clean Close |
| 2 pending SUB | SID 3 pending, SIDs 1/2 active, busy | 3 | 0/0/0 | reset; fourth ACK withheld |
| 3 explicit fresh recovery | three new accepted SUBs, three PUBs | 7 | 2/2/2 | clean Close |
| 4 pending UNSUB | SID 3 inactive, SIDs 1/2 active, busy | 7 | 2/2/2 | reset; eighth ACK withheld |
| 5 explicit fresh recovery | three new accepted SUBs, three PUBs | 7 | 2/2/2 | clean Close |

Independent broker logs record `subscription registered sid=3` on connection 3
and `subscription removed sid=3` on connection 5. Sleet's fourth/eighth `+OK`
exists in broker wire captures but not Ada-delivered captures. No phantom ACK,
replay, duplicate SUB/UNSUB/PUB, stale-epoch document, or retired callback occurs.
Both reset cases produce the expected transport exception:

```text
OMS_LACAL.TRANSPORTS.WEBSOCKET.TRANSPORT_ERROR : TCP read failed: [104] Connection reset by peer
```

Every retired connection has one terminal event, zero active old identities,
`Busy=False`, and zero lost events. Every warmed cleanup restores **8 file
descriptors and 3 Linux tasks**. No unexpected exception, harness timeout,
forced broker kill, or consumer failure occurs. Application and broker exits
are all zero; relay/controller threads terminate and the broker socket closes.

Lightweight validation: 10 Task113 verifier/mutation tests (multiple deliberate
wire/output mutations), 12 Task112 tests, 5 Task109 tests, 12 Task110 tests, and
38 PR smoke selector/execution tests pass. A real Task112 default-controller
compatibility run also passes eight epochs/48 callbacks using the unchanged
historical GNAT16/OpenSSL consumer binary. Its new records are outside Git;
historical evidence is untouched. `git diff --check` passes.

Relevant unchanged GNAT16 Ada runtime regressions pass: all 24 runtime-core
cells (two namespaces, portable/lossless profiles, Normal/Check/Ignore assertion
policies, O0/O2) and all six heterogeneous-dispatch cells (64 checks each),
including both expected compile-time directionality rejections. These focused
runtime/dispatch regressions do not rerun the historical Task100–111 transport,
trust or full real-UCI campaigns. Additional GNAT14 regression outcomes, when
complete, are reported in the final PR delivery.

Changing the shared selector correctly invokes all existing backend smoke suites
for this PR; Task113 helper-only changes route to Ada. No runner lane or workflow
policy changes were made. Hosted exact-final-HEAD results are reported in the PR
and final delivery, not falsely attributed to the earlier local qualification.

## Limits

The authenticated TLS peer is the relay; Sleet is its unchanged plaintext upstream,
as in Task109–112. Pinned Sleet does not reciprocate WS Close; normal teardown
forwards real TLS EOF rather than fabricating a WS reply. Resource qualification
measures Linux descriptors/tasks, not arbitrary heap allocations. This tests an
application-owned epoch policy, not every network race or automatic reconnection.