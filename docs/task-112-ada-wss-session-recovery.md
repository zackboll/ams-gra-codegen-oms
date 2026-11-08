# TASK112 — application-owned established WSS session recovery

## Scope and starting identity

The dedicated branch starts at freshly fetched `origin/main`
`3986bb19df44cdd841c82c250b4214ce2f0a8206`, tree
`b7319947b0029c359e4d9f1704ae93db9cb06d2f`. This is newer than the request's
last-verified main: PR #191 had already merged. Its branch and files are not
modified. This test uses verified WSS with `CA_Path`, not `CA_PEM`.

No production runtime, generated model/codec, schema, dependency, language
backend, shared IR, runner allocation, or main-only Deep CI policy changes.
The only existing code changed is the PR path router and its unit tests, to
classify `/scripts/task112/` as Ada-owned with conservative boundary tests.

## Owning application

`/tests/ada-wss-session-recovery/recovery_consumer.adb` owns eight lexical epochs
in one Ada process. Each invocation declares a fresh transport, connection,
three generic typed bindings and three receiver objects. Generated Subscriber
and Publisher APIs connect these to the unchanged Task110 combined contract
(or Task109/106's original UCI 2.5 contract for the regression control).
There are no heap owners, unchecked access/deallocation, background workers,
retained borrowed deliveries, retry queues or automatic replay. Bindings
finalize before receivers, then connection, then transport.

Every epoch requires verified TLS, genuine `owp` upgrade, one INIT and correct
INFO (protocol, broker, service UUID), three acknowledged subscriptions, and
six correctly routed typed callbacks: one generated Ada publication and one
independent external publication per family. Full decoded documents are
compared against independent Task102/103/105 goldens (Task104 for UCI 2.5).
Only the existing UCI header Timestamp is varied to identify epoch and phase;
no schema or OWP extensions are introduced. Local observations carry epochs.

Sequence: warm-up normal Close; authenticated TLS EOF; recovered normal Close;
socket reset; recovered normal Close; pending PUB reset; recovered normal
Close; explicit binding/application teardown. Each failed epoch is fully
finalized before the next invocation. SIDs `1,2,3` are intentionally reused,
never considered globally unique. A closed connection is never initialized.

## Real broker and deterministic fault controls

`/scripts/task112/recovery.py` runs the pinned unmodified Sleet binary against
the authentic release-specific schema for the whole eight-epoch campaign.
An independent observer/publisher remains connected throughout. The relay
forwards actual upgrade/OWP/WebSocket bytes. It never fabricates successful
responses or a WebSocket Close. Full captures distinguish broker bytes from
bytes actually delivered to Ada.

The application emits `ARMED` only after all six callbacks and exactly seven
ACKs (INIT, three SUB, three PUB). The controller then sends genuine TLS
`close_notify` or sets `SO_LINGER(1,0)` for a reset. The pending case sends a
fourth generated Log PUB and proves `Busy` before arming. The relay withholds
that command's real broker response, waits until the real eighth `+OK` has
been captured and the observer has independently received the unique PUB,
then resets. These protocol/observation barriers, not sleeps, trigger faults.

The observer proves this particular interrupted PUB **was processed once**.
That is additional observer knowledge, not something inferred from Ada's
missing ACK. From the application connection's perspective its outcome is
ambiguous. Recovery never retransmits it. An ordered observer UNSUB/ACK
barrier rejects latent duplicate deliveries after recovery.

Pinned Sleet exits on receipt of WebSocket Close without sending a reciprocal
Close frame. Normal application Close forwards Ada's Close and the relay
converts genuine broker EOF to authenticated TLS EOF; it does not invent a
WebSocket response. This is not a new reciprocal-Close qualification.

## Failure and cleanup assertions

Authenticated EOF must produce closed behavior without an exception. Reset
and pending interruption permit only the documented `WS.Transport_Error`;
all other exceptions fail the application. Production `Connections.Poll`
stops the owning connection before propagating a transport exception. Each
epoch requires `Closed`, `Busy=False`, no active SIDs, exactly one observable
`Connection_Closed`, zero lost events, no post-retirement callback, and no
phantom ACK. Ten additional closed Polls must remain inert.

After entropy and one complete TLS warm-up, `/proc/self/fd` and
`/proc/self/task` counts must return to the same baseline after every lexical
epoch exits. DNS worker quiescence is condition-driven with a monotonic
deadline, including warm-up. Counts include the plan file, entropy descriptor,
and runtime process-owned tasks; they are not expected to be zero. This
checks descriptor/task accumulation, not arbitrary allocator heap retention.
Application waits have five-second monotonic deadlines, resource quiescence
two seconds. Socket operations, relay epochs, stdout barriers and outer
process groups have independent bounded harness deadlines. Harness timeouts
are failures, not successful transport classifications.

## Reproduction and evidence

Use an external evidence directory, the existing Task096/110 Alire setups,
clean pinned checkouts, the original schemas, and a production generator built
from the current source. Python requires the already used `websockets` package.
The qualifier archives AHT/websocket-ada to isolated scratch copies, copies
Alire providers to avoid cross-compiler contamination, verifies compiler
versions, schema hashes and unchanged generated files, and checks dependency
cleanliness before and after. GNAT16/OpenSSL runs first.

```sh
cd /home/zboll/git/ams-gra-codegen-oms-task112
PYTHONDONTWRITEBYTECODE=1 /home/zboll/task110-evidence/venv/bin/python \
  /home/zboll/git/ams-gra-codegen-oms-task112/scripts/task112/qualify.py \
  --scratch /home/zboll/task112-evidence/accept16-openssl \
  --setup /home/zboll/task110-evidence/setup16 \
  --deps /home/zboll/task110-evidence/deps \
  --schema25 /home/zboll/task110-evidence/regression25/uci25 \
  --schema26 /home/zboll/task110-evidence/deps/uci26.extracted \
  --generator /home/zboll/task112-evidence/target/release/ams-gra-codegen-oms \
  --compiler 16 --backend openssl --release 26 25
```

Repeat for compiler `14`/setup14 and backend `mbedtls` with new scratch paths;
only the representative GNAT16/OpenSSL cell includes `--release 26 25`.
Acceptance rejects dirty sources; `--development` is diagnostic only.

Full commands, exits, stdout/stderr, raw TLS-terminated wire captures, observer
transcripts, schemas, generated sources, isolated builds, and SHA256 artifact
manifests remain outside Git at `/home/zboll/task112-evidence/`. Compact
receipts under `/docs/evidence/task112/` record source/dependency identities,
per-epoch observations and document frame hashes. To independently revalidate
all full artifacts and emit a receipt:

```sh
/home/zboll/task110-evidence/venv/bin/python \
  /home/zboll/git/ams-gra-codegen-oms-task112/scripts/task112/verify.py \
  /home/zboll/task112-evidence/accept16-openssl \
  --receipt /home/zboll/git/ams-gra-codegen-oms-task112/docs/evidence/task112/16-openssl.json
```

Lightweight tests:

```sh
cd /home/zboll/git/ams-gra-codegen-oms-task112
PYTHONDONTWRITEBYTECODE=1 /home/zboll/task110-evidence/venv/bin/python \
  -m unittest discover -s /home/zboll/git/ams-gra-codegen-oms-task112/scripts/task112 -v
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover \
  -s /home/zboll/git/ams-gra-codegen-oms-task112/scripts -p test_pr_smoke.py
git diff --check
```

## Qualification results

Clean-source four-cell acceptance and the UCI 2.5 regression receipt are
recorded separately after execution. Diagnostic runs are never promoted to
acceptance evidence. No Task108/111 safety/trust campaign is repeated because
this task changes neither production transport nor trust handling.