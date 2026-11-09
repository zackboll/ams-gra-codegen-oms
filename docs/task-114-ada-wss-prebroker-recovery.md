# TASK114 — complete-frame pre-broker interruption and fresh WSS recovery

## Baseline and scope

Freshly fetched main: `603c4bddbeaccac45fc68373a20ff75c457a95aa`, tree
`0ac3c8ae3d5f50d6d2c6ff829e4a80b6db5f618e`; PR #197 is merged.
Worktree: `/home/zboll/git/ams-gra-codegen-oms-task114`, branch
`feature/114-ada-wss-prebroker-recovery`.

Zero production changes. Historical consumers, tests and receipts are unchanged.
No dependency, schema, generated code, protocol, reconnect, replay, backend,
toolchain, runner or workflow-policy changes. The PR selector owns Task114 as Ada.
The only shared harness changes are optional `on_armed`, `on_probe` and
`validate_owners` profile hooks in `/scripts/task112/recovery.py`. Without these
attributes the original controller paths remain in use.

## Architecture and independent evidence

`/scripts/task114/framing.py` parses incrementally, retaining an entire masked
client frame before allowing forwarding. It handles split upgrade/header/mask/
payload reads, canonical 16/64-bit lengths and coalesced frames. Preceding bytes
are forwarded unchanged. The selected frame and all its bytes are withheld;
unexpected later frames, fragmentation, RSV bits, unsupported opcodes, unmasked
frames, invalid UTF-8, truncated input and oversize input fail qualification.
Limits are 16 KiB upgrade and the existing 1 MiB OWP frame limit. The TLS caller
uses at most 64 KiB reads; frame buffering is bounded independently of captures.

`/scripts/task114/recovery.py` is an opt-in Task112 profile, not a replacement
default relay. It uses the existing real TLS MemoryBIO peer, real WebSocket
upgrade and `owp` negotiation, with a pinned unchanged Sleet plaintext upstream.
Ada verifies the relay's certificate and localhost hostname. It never fabricates
OWP responses, deliveries or Close frames. Every broker response is forwarded.

Each epoch preserves four separate raw captures: `ada` (received from Ada),
`forwarded` (sent to Sleet), `sleet` (received from broker), and `delivered` (sent
to Ada). The independent verifier reparses the complete original client stream
and requires `forwarded == ada[:target_offset]`, with the one complete target
occupying exactly the remainder. Target length, text and SHA-256 are checked.
This rejects even a single target byte forwarded, missing frames and truncation.
Broker-returned and Ada-delivered streams must match, including raw hash checks.

Broker owners are independently reconstructed from logs. Source-backed `recv`,
registration, removal, ACK and disconnect observations must agree with exact
forwarded commands. At pinned Sleet `/sleet/src/server.rs:814` registration is
logged after successful router subscribe; `:818–843` implements application UNSUB
and logs removal after router unsubscribe. `:257–259` calls `remove_connection`
and then logs disconnected. `/sleet/src/router.rs:258` documents disconnect
cleanup. Thus generic disconnect cleanup is never treated as an application UNSUB.

## Five lexical Ada epochs

The focused `/tests/ada-wss-prebroker-recovery/recovery_consumer.adb` derives from
TASK113 without modifying it. All epochs create new transport, connection,
bindings and receivers; closed connections are never reinitialized.

| Epoch | Operation and evidence | Ada ACKs | Typed callbacks |
| --- | --- | --- | --- |
| 1 | normal, third SUB + UNSUB positive controls | 8 | 2/2/2 |
| 2 | complete third SUB withheld; only broker SIDs 1/2 | 3 | 0/0/0 |
| 3 | fresh recovery, third SUB + UNSUB positive controls | 8 | 2/2/2 |
| 4 | complete UNSUB 3 withheld; broker retains SID3 | 7 | 2/2/2 |
| 5 | fresh recovery, third SUB + UNSUB positive controls | 8 | 2/2/2 |

Pending SUB requires production Busy, allocated SID3 not active, SIDs1/2 active,
and INIT plus two accepted SUBs. Pending UNSUB requires local SID3 inactive,
SIDs1/2 active and Busy, after authentic generated publications and independent
external deliveries for all three families. Retained authentic deliveries cannot
invoke the unsubscribed binding. Cross-binding unsubscribe is rejected locally.

The controller waits for both complete interception and the application's ARMED
barrier. For pending UNSUB it then publishes an authentic Survivability document
on the independent observer connection. Sleet's real `MSG 3` proves continued
broker ownership before disconnect. Ada polls it as Unknown_Subscription and
emits `PROBE_REJECTED` only with Busy, unchanged seven ACKs and unchanged typed
callback counts. Only this independent barrier releases the reset. No timing
sleep establishes processing. All waits have monotonic or bounded API deadlines;
timeouts are failures, not successful interruptions.

All terminal states require Closed, not Busy, no active old SIDs, one terminal
event, no late ACK/callback and no lost events. Saved authentic deliveries are
dispatched after retirement and binding Close. Lexical finalization prevents
callbacks into finalized receivers. Resource restoration compares warmed Linux
FD/task counts after every epoch. Fresh epochs explicitly request three new
subscriptions, publish with generated APIs and receive independently authored
epoch-specific fixture values. Numeric SIDs are connection-local.

## Reproduction

Full artifacts are outside Git at `/home/zboll/task114-evidence/`. Reuse the
qualified `/home/zboll/task110-evidence/venv/bin/python`, dependency directory,
isolated setup14/setup16 and generator; do not install or upgrade anything.
The builder checks exact compiler version, five clean dependency pins/trees,
authentic schema hashes, generated model/codec hashes and unchanged input hashes.

```sh
cd /home/zboll/git/ams-gra-codegen-oms-task114
/home/zboll/task110-evidence/venv/bin/python -B scripts/task114/qualify.py \
  --scratch /home/zboll/task114-evidence/accept16-openssl \
  --setup /home/zboll/task110-evidence/setup16 \
  --deps /home/zboll/task110-evidence/deps \
  --schema25 /home/zboll/task113-evidence/final16-openssl/uci25 \
  --schema26 /home/zboll/task113-evidence/final16-openssl/uci26 \
  --generator /home/zboll/task112-evidence/target/release/ams-gra-codegen-oms \
  --compiler 16 --backend openssl --release 26 25
/home/zboll/task110-evidence/venv/bin/python -B scripts/task114/verify.py \
  /home/zboll/task114-evidence/accept16-openssl \
  --receipt /home/zboll/task114-evidence/16-openssl-receipt.json
```

Use fresh scratch directories for compiler14 and mbedTLS cells; only the
GNAT16/OpenSSL cell also needs release25. Acceptance requires clean committed
source; development runs are explicitly non-acceptance. Full command JSON,
stdout/stderr, hashes and artifacts are retained externally. Compact checked
receipts belong under `/docs/evidence/task114/`.

## Qualification results

The full matrix was repeated after the ordinary main merge and final verifier
hardening, on `ec4c95b3ad073a181784a10b684f511710bffad4`, tree
`bd2b3bcaff073b015606c1455c90b33ab7e862d3`. All four UCI2.6 cells and
the GNAT16/OpenSSL UCI2.5 campaign passed again. Committed receipts below
`/docs/evidence/task114/` refer to this later source, not the initial run.
The subsequent receipt-only commit does not change qualification inputs.

All four authentic UCI2.6 cells passed on clean committed implementation
`5fc2f7ee7c81e772935d11a407cc2a30aa4506ce`, tree
`0984c070e4935d14a2bd371b0bfbf40fe777fbd5`. GNAT16/OpenSSL ran first and
also passed the UCI2.5 five-epoch regression. Compact receipts include the
source identities, dependency trees, schema/generated hashes, artifact-manifest
hashes, exact wire boundaries, broker operation counts/logs and application
observations. Full artifacts remain outside Git.

| Compiler | OpenSSL / UCI2.6 | mbedTLS / UCI2.6 |
| --- | --- | --- |
| 14.2.0 | PASS | PASS |
| 16.1.0 | PASS | PASS |

Every campaign has five terminal events, 24 typed callbacks, zero lost events,
no stale ACKs/replay, and restoration to 8 FDs / 3 tasks at every retirement.
The complete SUB3 frame is withheld at byte offset 356 (56 bytes); UNSUB3 is
withheld at byte offset 1991 (13 bytes). These offsets refer to decrypted raw
WebSocket streams including the genuine HTTP upgrade. No target byte is in the
forwarded capture. Broker connection3 receives INIT/SUB/SUB only (three ACKs).
Connection5 receives INIT/three SUBs/three PUBs only (seven ACKs), retains SID3
for the independently published probe, then performs disconnect cleanup.
Normal connections2/4/6 register SID3, process UNSUB3 and emit eight real ACKs.

Historical GNAT16/OpenSSL TASK113 passes unchanged (five epochs, 24 callbacks),
and TASK112 default controller passes unchanged (eight epochs, 48 callbacks).
Parser/mutation tests: 12 PASS; TASK112: 12 PASS; TASK113: 10 PASS; TASK109: 5
PASS; TASK110: 12 PASS; PR smoke selector: 39 PASS. Sixteen additional mutations
of actual UCI2.6/2.5 artifacts reject missing/truncated targets, partial forwarding,
missing probes and duplicate independent publications. `git diff --check` passes.

Main advanced to `aadf62203761bdafb0d89f8b3c714b97bdd7c5f0` (merged PR196).
It was incorporated by ordinary merge `4eb1feddb99b5f0b6b92e7e4f184c5ba827d1b10`,
tree `4196c35f4bffc033b373f066c1080b7044ad9cfb`. No open unrelated branch was
incorporated. Final revision qualification and hosted CI are separately recorded
outside Git and in the PR; these earlier receipts are not relabeled as final-HEAD
runs.

## Limitations

The TLS endpoint is the relay, not Sleet; the unchanged upstream is loopback WS.
Normal teardown forwards genuine TLS EOF because pinned Sleet does not reciprocate
WS Close; no Close is synthesized. Resource qualification covers Linux FD/tasks,
not arbitrary heap leaks. This is an application-owned five-epoch recovery policy,
not exhaustive race exploration or any automatic reconnect/replay implementation.
Local qualification and exact-final-HEAD hosted CI are reported separately.