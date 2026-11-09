# TASK116 — caller-owned absolute deadline for Ada WSS

## Baseline and original failure

Freshly fetched `origin/main`: `2c560d00f406e378cc15f151e3d18a1e79f5c906`,
tree `41ce490665f10272a40cafdc7ac2b8be930f9962`.
Worktree: `/home/zboll/git/ams-gra-codegen-oms-task116`.
Branch: `fix/ada-wss-caller-deadline`.

Before editing the production transport, the actual PR203/TASK115 executable
(`/home/zboll/task115-evidence/final16-openssl/build/bin/ada-uci-client`) was run
twice against a TCP listener that accepts but never completes TLS. Independent
Python `time.monotonic()` measurements were **2.020470 s** and **2.020745 s** for
`--duration-seconds 1`; both exit 1. Original PR203 recorded 2.012225 s.
These are failing deadline observations, not successful bounded operation tests.
Full reproduction commands/output: `/home/zboll/task116-evidence/reproduction/`.

Blocking path: standalone `Epoch` -> `WS.Open` -> pinned AHT `TLS.Connect` ->
TLS handshake's TCP read, with historical `Handshake_Deadline => 2.0`. The
application's deadline check after `Open` cannot interrupt that wait.

## API and lifecycle

```ada
WS.Set_Absolute_Deadline (Wire, Deadline); -- Ada.Real_Time.Time
WS.Open (Wire, URL, CA_Path => Trusted_CA);
```

The optional deadline is stored on `WebSocket.Client`, uses the monotonic Ada
clock, and constrains subsequent blocking `Open`, `Send` and graceful `Close`.
There is no change to the abstract transport or LA-CAL core interfaces, and no
reconnect supervisor. Existing clients which never call the setter retain their
established timeout settings. The deadline is **not reset** by failure, explicit
Close, abortive Drop or attempted client reuse. Calling the setter again explicitly
replaces it; it does not reconnect or revive an existing failed session. A new
client begins unconfigured. No clear/reset facility or insecure option is added.

Expired/failed Open and Send fail closed and release sockets/TLS/uncertain output.
Expired Close immediately uses the existing abortive release path, without sending
a Close frame or awaiting a reciprocal response. Repeated Close and finalization
are safe. Finalization still only drops resources; no waiting path is added.

## Exact AHT allocation

The five authorities are unchanged:

| Source | Commit |
| --- | --- |
| AHT | `827b0c17dd964e935f8e88912ce0611ee92b6f16` |
| websocket-ada | `765d9cd612838eb00645037977c8cc9af7171c3e` |
| Sleet | `e38f61d8ce0d75c8508434a52f2ed77c69cf6a3b` |
| UCI2.5 | `093610b7753944059360d3236770ab446d039556` |
| UCI2.6 | `78eb61b6112c8bffa40820c33124b57787fc5bd9` |

AHT's `Resolve_Timeout`, `Connect_Timeout`, and `Handshake_Deadline` are separate
durations. Immediately before calling AHT Connect, configured clients compute:

```
R = remaining monotonic caller time
B = min(2 seconds, (R - 2 milliseconds) / 3)
Resolve_Timeout = Connect_Timeout = Handshake_Deadline = B
```

An exhausted/nonpositive allocation fails before dialling. The 2 ms reserve
covers the pinned DNS shim's millisecond rounding and small setter/conversion
costs, not arbitrary scheduler pauses. The sum of the three granted network-wait
budgets is less than the remainder; the full remainder is **not granted three
times**. Unused shares are not redistributed. This deliberately permits early
failure (a one-second stalled handshake normally fails around 0.33 s). In both
pinned TLS bodies `HS_Bound` is actually calculated at TLS Connect entry before
DNS/TCP, making the current implementation even more conservative; the allocation
does not depend on treating that detail as three independent full remainders.
Legacy clients continue passing exactly 2.0 for each AHT setup duration.

After setup, the caller is checked again. Upgrade reads use the earlier of the
existing two-second upgrade bound and the absolute caller bound. Every TLS read
setter gets freshly computed remaining time, not a renewed whole-second budget;
trickled headers cannot extend it. Initial request and later frame writes use the
earlier of their existing two-second operation bound and caller bound. One AHT
compound write deadline covers previously queued controls **and** the new frame;
it is not renewed between these writes. Plaintext selector waits are capped by
remaining time. Validation of frame size/UTF-8 remains before publication.

Close caps its 250 ms graceful wait by caller time; its write is also caller-capped.
An already expired Close performs no blocking write. No reciprocal server Close
is synthesized, including in compatibility tests with pinned Sleet.

## Qualification and reproduction commands

Use an already provisioned TASK110-style isolated Alire setup (GNATCOLL26,
GPRbuild26, selected GNAT14.2 or GNAT16.1), clean dependency checkouts, and a new
external scratch directory. Neither source dependency checkout is built in place.

```sh
python3 /home/zboll/git/ams-gra-codegen-oms-task116/scripts/task116/qualify.py \
  --scratch /home/zboll/task116-evidence/final16-openssl \
  --setup /home/zboll/task110-evidence/setup16 \
  --deps /home/zboll/task110-evidence/deps --compiler 16 --backend openssl
python3 /home/zboll/git/ams-gra-codegen-oms-task116/scripts/task116/verify.py \
  /home/zboll/task116-evidence/final16-openssl \
  --receipt /home/zboll/task116-evidence/16-openssl-receipt.json
```

The Python interpreter needs the existing `websockets` dependency only for
historical real-Sleet compatibility runners; focused deadline tests use stdlib.
The build helper uses Alire `printenv`, copies GNATCOLL/provider sources externally,
archives immutable AHT/codec source, and selects the TLS backend via GPR external.
All five dependency commits/trees and cleanliness are checked before and after.
Development runs are explicitly not acceptance receipts.

The actual production WS client is tested against a controlled real TLS
MemoryBIO peer with a trusted localhost certificate, real upgrade, captures and
external monotonic timing. No mock TLS, alternate transport, insecure switch,
background application task, cancellation, or new unchecked access is used.

Focused cases include stalled TLS (one second and 50 ms), stalled upgrade,
trickled upgrade, cumulative delayed-TLS/delayed-upgrade, blocked outbound data,
queued Pong plus blocked data, stalled graceful Close, expired Open/Send/Close,
1 ms setup, successful verified echo/reuse, nine-second caller time (retains
two-second stage cap), and legacy handshake/echo/Close. Failed writes retain
at most a partial final publication in peer capture and never replay it; numbered
complete frames and the queued Pong's position are independently checked.

External tolerance is **150 ms** for scheduler/process-exit overhead, not another
two-second timeout. Upgrade/write/Close cases must run through at least 0.8 s to
prove the intended operation actually stalled; conservative TLS setup is allowed
to fail earlier. Harness timeout, wrong exit, peer exception, leaked task, missing
operation, replay or receipt disagreement fails verification.

## Remaining limitations

This is a bound on network waits, not hard-real-time preemption of local CPU,
certificate-file I/O, memory allocation, provider initialization, or OS scheduling.
All return paths check caller time and fail closed when exhausted. The existing
pinned AHT DNS implementation owns a detached resolver worker; an abandoned DNS
lookup may outlive its bounded caller wait until the OS resolver finishes. No new
worker is introduced here. Localhost qualification requires resolver quiescence;
it does not claim cancellation of arbitrary OS DNS calls or exhaustive DNS/TCP
fault injection. Cleanup uses AHT's existing nonwaiting abortive release.

Exception classification is intentionally unchanged: a `Transport_Error` does
not become retryable. Passing the optional TASK115 integration probe would remove
only its demonstrated timeout overrun, not qualify non-peer-closure recovery.

## Results

Committed clean-source matrix, compatibility, optional integration and hosted
exact-HEAD results are recorded separately below and in compact receipts under
`/docs/evidence/task116/`; full logs/captures/builds/keys remain outside Git at
`/home/zboll/task116-evidence`.

### Clean-source four-cell acceptance

All 19 cases pass in each cell on HEAD
`865bb824db0f84033fffaa5c9fdf40808baa5144`, tree
`ce2b7443d2419ea69dd862807aaa17b678cb4f42`. GNAT16/OpenSSL ran first.
The following values are externally measured seconds for one-second caller time:

| Compiler/backend | TLS stall | Upgrade stall | Write stall | Close stall | Result |
| --- | ---: | ---: | ---: | ---: | --- |
| GNAT16.1/OpenSSL | 0.365 | 1.059 | 1.032 | 1.032 | PASS |
| GNAT16.1/mbedTLS | 0.340 | 1.012 | 1.007 | 1.009 | PASS |
| GNAT14.2/OpenSSL | 0.361 | 1.028 | 1.029 | 1.024 | PASS |
| GNAT14.2/mbedTLS | 0.350 | 1.018 | 1.019 | 1.017 | PASS |

Trickled and delayed cumulative upgrades also remain within 1.150 s. Legacy
handshake stalls remain approximately two seconds; long caller budgets retain the
old two-second stage cap. Each configured expired Open and 1 ms setup makes no
connection; 50 ms setup may expire before ClientHello under system load and is
classified only as a short-budget control, not evidence of a completed TLS stage.
One-second handshake cases independently require actual ClientHello bytes.

Both backends fail blocked writes closed with ordered numbered complete frames,
one possible partial tail, no replay, and exactly one preceding queued Pong in
the queued-control case. Repeated Close/finalization and an explicit new deadline
before client reuse pass. Final resource samples are 4 or 5 FDs / 1 task; the
directory handle is included and first-use TLS can retain one provider entropy FD.

An initial acceptance attempt correctly failed because a 50 ms setup exhausted
before ClientHello. The short-budget control was corrected to distinguish that
valid early expiration from the one-second real handshake case, committed normally,
and the entire matrix rerun. Initial recovery/trust campaigns overlapping that edit
were rejected for changed/dirty source and are not claimed as PASS. No historical
test or receipt was edited to turn those rejected campaigns into acceptance.

### Unchanged compatibility

GNAT16/OpenSSL representative runs pass:

- TASK108 full focused WSS safety/API/control/backpressure/regression and real
  Sleet runner: PASS on `0b5e5f2` (production source identical to `865bb82`).
- TASK111 trust: 147 attempts, 20 lifecycle cycles; 5 FDs / 1 task baseline/final.
- TASK112 recovery: eight epochs, 48 callbacks; stable original 8 FD / 3 task
  accounting at every retirement.
- TASK113 recovery: five epochs, 24 callbacks; stable original resource counts.
- TASK114 pre-broker recovery: five epochs, 24 callbacks; stable original counts.

TASK111–114 clean-source runs use `865bb82`. Original verifiers recheck their
source/artifact hashes, wire captures and real broker evidence. The TASK108 legacy
runner reports all steps successful. No unexpected safety regression required
broader reruns. Pinned Sleet remains unchanged and non-reciprocal WS Close is not
replaced by a fabricated response.

Focused verifier tests: 9 PASS; selector tests: 40 PASS. Unchanged TASK109–114 unit
suites pass (5/12/10/12/10/12 tests). Five mutations of actual deadline case records
reject the original two-second overrun, harness timeout, missing exercised stall,
cleanup failure and shortened legacy budget. `git diff --check` passes.

### Disposable TASK115 integration

Detached `/home/zboll/git/ams-gra-codegen-oms-task116-integration` combines TASK116
production source from `0b5e5f2` with PR203 application source at `6b8bdf0` and one
call to `Set_Absolute_Deadline (Wire, Deadline)` immediately before Open. It builds
the authentic production UCI2.6 client independently. Two stalled-TLS trials with
one-second requested application duration exit in **0.351854 s / 0.352432 s**,
both fail closed with exit 1 and no reconnect. No changes are pushed to PR203.
This removes the specific timeout overrun only; it does not qualify all TASK115
requirements or alter transport exception classification.

### Hosted delivery

PR204 is open and non-draft with auto-merge disabled. Hosted smoke passed on
qualified implementation `865bb82`:
https://github.com/zackboll/ams-gra-codegen-oms/actions/runs/42554158044 .
The later receipt/document-only commit is checked on its own exact hosted HEAD;
that final status is reported in PR204 and the final delivery report, not inferred
from this earlier successful run. No rebase/amend/force-push/self-approval occurs.