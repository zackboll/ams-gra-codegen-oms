# TASK115R1 — deadline corrected; CORRECTIVE_WORK_REQUIRED

These are new receipts, not replacements for the original TASK115 evidence.
All five cells built clean source `7fa13386fb97cec44ac739d7c017c76ca30dd801`,
tree `5c5b6daa04bbdf0e548b02a2c2b97f9b29cd89b3`, using the production
generator rebuilt from the merged repository and unchanged pinned dependencies.
GNAT16/OpenSSL ran first. Each cell executes the standalone application, not
a TASK112–114 consumer. Subsequent delivery changes are documentation/evidence
only; the source/input identities here are not relabeled as a later HEAD run.

| Compiler / backend / authentic UCI | Six Sleet cases | Eleven controls | 1 s stalled TLS elapsed | 2 s replacement deadline elapsed |
| --- | --- | --- | --- | --- |
| GNAT16.1.0 / OpenSSL / 2.6 | PASS | PASS | 0.335993 s | 2.004192 s |
| GNAT16.1.0 / mbedTLS / 2.6 | PASS | PASS | 0.334947 s | 2.002337 s |
| GNAT14.2.0 / OpenSSL / 2.6 | PASS | PASS | 0.337965 s | 2.004557 s |
| GNAT14.2.0 / mbedTLS / 2.6 | PASS | PASS | 0.335907 s | 2.003201 s |
| GNAT16.1.0 / OpenSSL / 2.5 | PASS | PASS | 0.336366 s | 2.003601 s |

The six cases are default fail-fast, two authenticated-EOF recoveries, exhausted
budget, invalid trust, unauthorized service and successful normal duration.
Controls are established TCP reset, malformed WS frame, invalid TLS record,
valid WS Close recovery, stalled replacement TLS, deadline during replacement
upgrade, expiry during reconnection delay, missing INFO, missing SUB ACK,
invalid upgrade and wrong hostname. A separate accepted-but-silent TCP peer
captures an actual ClientHello for the one-second initial stalled-TLS probe.

The original 2.003–2.020 s initial setup overrun is corrected. All new initial
stalls exit 1 with exactly one connection, no INIT/SUB/callback/retry, and no
harness kill. Conservative production setup allocation permits the earlier
timeout. All duration measurements use Python's external monotonic clock and
allow 150 ms external measurement/scheduling overhead, not hard-real-time claims.
Failed replacement TLS exits after about 1.503–1.505 s of a four-second invocation,
with one unused attempt remaining and no third epoch. The two-second replacement
upgrade control expires at the original deadline, not two seconds after recovery.

Authenticated EOF and valid WS Close recover within budget. TCP reset is observed
as `OMS_LACAL.TRANSPORTS.WEBSOCKET.TRANSPORT_ERROR` and is **terminal**, as are
TLS integrity and WS protocol failures. Replacement-attempt retries are
**intentionally unsupported/fail-closed**, not generically PASS for broader
network recovery. Every receipt retains `task115_acceptance: false` and
`CORRECTIVE_WORK_REQUIRED`. A separate structured transport/TLS classification
correction is required; see the R1 section of
`/home/zboll/git/ams-gra-codegen-oms-task115/docs/task-115-ada-uci-standalone-reconnect.md`.

Raw artifacts remain under `/home/zboll/task115-r1-evidence/accept-*-wss`;
isolated builds are the corresponding directories without `-wss`. Each
`artifacts.json` binds wire bytes, commands, broker/observer/application logs and
results, excluding private keys. The verifier independently reparses captured
application commands, broker acknowledgments, real authentic payloads, typed
callback timestamps and actual Sleet subscription owners. No client PUB/replay,
duplicate INIT/SUB, stale SID or post-retirement callback was observed.

All established/replacement retirement samples are 5 FDs / 1 task, including
the temporary procfs directory FD. Early OpenSSL hostname rejection has 4 FDs
before socket initialization; this is not resource growth. Cleanup joins harness
threads, gracefully terminates Sleet, checks its listening socket is closed, and
finalizes every application epoch. Expired normal duration uses transport
abortive release, with no TLS close_notify wait. Samples support bounded resource
lifecycle, not proof of arbitrary heap-leak freedom. No application task is added.

Ada parser/policy tests execute 37 checks in every cell, plus real CLI negatives.
Python suites TASK109/110/112/113/114/115/116 execute 5/12/12/10/12/6/9 tests;
selector tests execute 41. Ten evidence mutations are rejected, including extra
attempts, missing/stale callbacks, duplicate INIT, resource growth, missing broker
registration, duplicate independent publication, deadline overrun, reset retry and
false success. `validation.json`, `mutations.json` and `reconciliation.json` bind
these observations. Production Ada transport, unrelated backends, original
contracts, builder and historical receipts remain unchanged.

Hosted checks are separately observed on the exact published delivery HEAD;
local receipts do not imply hosted CI success or full TASK115 acceptance.