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

Results and exact source identities will be recorded in compact receipts under
`/docs/evidence/task115/`. Full build commands, generated source, wire captures,
broker logs, private keys and binaries remain outside Git. Local results and
hosted exact-HEAD smoke are distinct authorities. A helper timeout is failure,
not a successful reconnect.