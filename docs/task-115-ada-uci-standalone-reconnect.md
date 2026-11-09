# TASK115 — standalone Ada UCI receiver with bounded peer-closure recovery

## Baseline

Freshly fetched `origin/main` and starting HEAD:
`2c560d00f406e378cc15f151e3d18a1e79f5c906`; tree
`41ce490665f10272a40cafdc7ac2b8be930f9962`.
Branch: `feature/115-ada-uci-standalone-reconnect`.
Worktree: `/home/zboll/git/ams-gra-codegen-oms-task115`.
External execution records: `/home/zboll/task115-evidence`.

TASK109 provides verified real WSS; TASK110 qualifies the combined authentic
three-message UCI2.6 closure (and UCI2.5 regression); TASK111 adds trust negatives;
TASK112 establishes fresh lexical epochs and cleanup after established loss;
TASK113 qualifies pending subscription ownership; TASK114 proves complete-frame
pre-broker interruption and fresh recovery without ambiguous command replay.
The application does not use their test consumers or Python runtime controllers.
The qualification helper reuses their TLS peer/wire parser and independent golden
payloads, without changing any historical tests or evidence.

## Implementation

Application source and release-specific input-only contracts are under
`/examples/ada-uci-client/`; independent builder, real interoperability runner,
verifier and focused tests are under `/scripts/task115/`.
See `/examples/ada-uci-client/README.md` for commands and immutable dependency
identities. No production generator, shared IR, runtime, transport, dependency,
schema, historical fixture or hosted-runner policy changes are made.
The existing PR selector registers both new application and helper paths as Ada.

The Ada supervisor owns one process-wide budget and one monotonic deadline.
One scoped epoch owns one WS client, one connection, three generated Subscriber
instances, three receivers and three bindings. Borrowed dispatcher references do
not escape. The connection is initialized once and never reused after closure.
Event failure dominates connection loss; loss counters and unknown subscriptions
are terminal. Only an established epoch's unambiguous peer closure authorizes
another attempt. Bindings, receiver access values, deliveries and SIDs cannot
cross epoch masters. Receive-only contracts and the application contain no PUB.

## Blocking API constraints (not hidden as successful qualification)

The unchanged production transport specification
`/ada/oms-lacal-runtime/websocket/oms_lacal-transports-websocket.ads` documents
separate two-second resolve/connect/TLS/upgrade budgets. `Open` has no remaining
deadline parameter; `Send` likewise owns fixed write deadlines, and orderly
`Close` adds a bounded wait. Checking the application's monotonic deadline before
and after each blocking operation cannot make those operations stop exactly at
the global deadline. This is a blocking mismatch with TASK115 sections 7 and 12.
No asynchronous abort, additional worker or unsafe resource cancellation is added
to disguise it. A future focused runtime change would need a caller deadline
propagated into resolve, connect, handshake, upgrade, writes and close.

`Open` and `Poll` catch `others` and rethrow `Transport_Error`, including malformed
WebSocket frames, invalid upgrade responses, TLS failures, socket errors and
unexpected runtime exceptions. Treating every such exception as retryable would
violate the requested failure classification. No backend-specific string matching
is used: these ambiguous exceptions fail closed. Thus authenticated TLS EOF/valid
peer close can recover, but socket-reset exceptions can remain terminal and failed
replacement connection attempts are fail-fast. The task allows bounded replacement
connect retries but this version deliberately does not guess their safety.

**Disposition: BLOCKED_WITH_EVIDENCE** until strict deadline propagation is
available and qualified. Real peer-closure recovery results must not be interpreted
as full TASK115 acceptance. No dependency or transport upgrade is proposed here.

## Validation record

Results and exact source identities are recorded in compact receipts under
`/docs/evidence/task115/`. Full build commands, generated source, wire captures,
broker logs, private keys and binaries remain outside Git. Local results and
hosted exact-HEAD smoke are distinct authorities. A helper timeout is failure,
not a successful reconnect.

All five clean-source builds and real-WSS campaigns use implementation HEAD
`760e4a429d043386fd8236793dc049b775aed2e2`, tree
`ff28539a723a8508a21697a30072a331e67a2b22`. Receipt/document-only delivery
changes do not relabel those campaigns as runs on a different HEAD.

| Compiler | OpenSSL / UCI2.6 six cases | mbedTLS / UCI2.6 six cases |
| --- | --- | --- |
| GNAT 14.2.0 | PASS, deadline blocker observed | PASS, deadline blocker observed |
| GNAT 16.1.0 | PASS, deadline blocker observed | PASS, deadline blocker observed |

GNAT16/OpenSSL ran first. Authentic UCI2.5 GNAT16/OpenSSL also passes all six
cases. Production service-check reports READY for all three selected messages:
UCI2.6 has 65/65 model types and 64/64 codec declarations; UCI2.5 has 64/64 model
types and 63/63 codec declarations. Each executable reports its compiled schema.

Each campaign independently confirms:

- Default: one epoch, three callbacks, established loss, exit 1, no replacement.
- Budget two: three epochs, nine callbacks, fresh INIT/INFO and three SUB ACKs
  per epoch; no fourth connection; requested duration ends with exit 0.
- Budget one: two epochs, six callbacks, two losses, exit 1; no third attempt.
- Invalid trust: TLS rejected before HTTP/OWP; exit 1 and no retry/fallback.
- Unauthorized service: real Sleet -ERR, no subscriptions/callbacks/retry, exit 1.
- Normal: explicit CA_PEM input, three typed callbacks, duration shutdown,
  exit 0 and no retry despite positive recovery budget.

All successful retirement samples have 5 FDs / 1 task (the FD directory sample
includes its own temporary directory handle). Counts remain stable through all
three recovery epochs. Application code adds no background task. Relay, publisher,
application and Sleet processes/threads finish, and broker listening sockets close.
Every captured application operation is exactly INIT followed by three intended
SUBs; no PUB/UNSUB/replay is sent. Epoch-specific generated header timestamps
match actual broker deliveries and callback output. Every terminal event is
drained, lost-event count stays zero, and no retired epoch has later callbacks.

Deadline probe elapsed times (one second requested): GNAT16/OpenSSL 2.012s,
GNAT16/mbedTLS 2.003s, GNAT14/OpenSSL 2.020s, GNAT14/mbedTLS 2.016s.
These are observed **failures of strict deadline compliance**, not harness timeouts
or acceptance PASS. The stalled initial connection fails closed without retry.

Actual Ada policy/parser tests execute 35 checks in every build. Python focused
source tests: 4 PASS; selector tests: 40 PASS. Unchanged TASK109/110/112/113/114
unit suites pass (5/12/12/10/12 tests). No historical helper modifications require
re-running their entire real campaign. Seven mutations of actual Task115 evidence
reject missing/stale callbacks, extra attempts, changed wire bytes, missing broker
registration, resource growth and duplicate independent publication.

Hosted PR smoke on implementation `760e4a4` passed:
https://github.com/zackboll/ams-gra-codegen-oms/actions/runs/42539862992 .
Final delivery HEAD hosted status is reported separately in PR203 and the final
report; it is not inferred from this earlier successful run.

## Remaining limitations

In addition to the blocking deadline and ambiguous-transport classification
constraints, Linux FD/task sampling does not establish arbitrary heap leak freedom.
The relay is the verified TLS endpoint and Sleet is unchanged loopback WS upstream.
Only authenticated peer closure is exercised for successful recovery; socket-reset
exceptions remain terminal. The build helper expects already provisioned isolated
Alire dependencies and authentic local source, not an automatic downloader. PEM
input storage is capped at 1 MiB. There is no signal-driven shutdown feature,
publication, replay or recovery of messages sent while disconnected.