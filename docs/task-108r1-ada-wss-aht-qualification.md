# TASK108R1 — downstream AHT nonblocking WSS acceptance

## Disposition and identities

**Required capability matrix: PASS in the tested configurations.** Independent
review is still required before preparing an AHT upstream output PR. No AHT or
Sleet source or existing PR was changed. No upstream submission was made.

The isolated worktree is `/home/zboll/git/ams-gra-codegen-oms-task108r1`, branch
`feature/108-ada-wss-aht-fork-r1`. Freshly fetched OMS base:
`27e9c3d3d656e3724490f0743f9ab93f9060bae1`, tree
`dda892d2039ee9ff361f8a296bdaf6710124331b`.

| Input | Immutable revision |
| --- | --- |
| AHT fork `https://github.com/zackboll/aht` | `4361ba5c9f4d187955dcd484fffef9296a49362a` |
| AHT tree | `cb7e8e08ea5db376fe4443fdcddc1c803826cb3a` |
| websocket-ada | `765d9cd612838eb00645037977c8cc9af7171c3e` |
| Unmodified Sleet | `e38f61d8ce0d75c8508434a52f2ed77c69cf6a3b` |
| Sleet tree | `7e705f4eb5f658957bd414b4abc8207311f3c1e4` |

The previous TASK108 blocker report was inspected in the preserved worktree at
`/home/zboll/git/ams-gra-codegen-oms-task108/docs/task-108-ada-wss-aht-fork.md`.
Its SHA-256 is `3ca3f9c6605b1f1ecd027e831d37863b676fe4e1c2ec267a12f318f539bee08f`.
It was an untracked report there, not a document on this task's fresh base.
The old absence of nonblocking output is resolved by this exact AHT revision;
the presence of the API was followed by real downstream execution, not treated
as sufficient acceptance evidence.

## Integration and ownership

`OMS_LACAL.Transports.WebSocket.Client` retains its existing framing, bounds,
masking, UTF-8 validation, handshake validation, and single-task ownership.
No receiver task or generated-code/interface change was introduced. Plaintext
remains IPv4 loopback-only. WSS supports an explicit port and DNS/IPv4 host;
IPv6 URL syntax, proxies, compression, authentication and automatic reconnect
are not added.

The companion imports AHT's GPR project once; the codec project no longer
recompiles AHT framing/root units. The captured GPR closure has no diagnostics
and the executable binds no AHT high-level receiver. OpenSSL 3.5.7 links as
`libssl.so.3` and `libcrypto.so.3`. The mbedTLS 3.6.7 environment was available
and was exercised, not skipped. GNATCOLL remains 26.0.0; GPRbuild is 26.0.1.

`Open (Client, URL)` uses verified backend trust defaults. The overload accepting
`CA_Path` or `CA_PEM` supports explicit trust without an insecure option. AHT
checks the original host (DNS name/SNI or IP SAN); both CA inputs together are
rejected by AHT. No successful test uses `Insecure => True`. Resolve, TCP connect,
TLS handshake and HTTP upgrade each have bounded two-second setup budgets.
`owp` negotiation and strict HTTP upgrade checks remain mandatory.

`Poll` calls only TLS `Try_Read`, `Try_Flush`, `Try_Write`, and `Pending_Output`.
It takes at most one 4 KiB plaintext receive step and consumes at most one
WebSocket frame. There is one masked control frame of at most 131 bytes queued
in OMS; its offset advances only by AHT's accepted plaintext count. AHT owns
unsent ciphertext. Further frame consumption pauses until prior output drains,
preserving queued control order. TLS-generated output progresses at the start
of every Poll even without input. A 64 KiB TLS-output limit fails closed after
a receive step; input and fragment bounds are retained. There is no readiness
wait, blocking TLS I/O or recursive receive/write progression in Poll.

`Needs_Read` explicitly leaves the frame offset unchanged and takes at most
one receive step before returning. **No real fixture produced `Needs_Read`** on
either backend; that branch is source-reviewed, not runtime-qualified. The
upstream fork's own output tests describe the same fixture limitation. This
is not a claim of coverage for TLS renegotiation/other post-handshake features
that the engines do not permit. No workaround or AHT correction was required.

`Send` retains the interface's bounded blocking whole-publication semantics,
draining queued controls first; uncertain or incomplete output closes the
session rather than returning success. Peer WebSocket Close retains the TLS
session until its response flushes. Failed/abandoned sessions and finalization
explicitly call `Abortive_Close` before deallocation/inherited finalization.
This deliberately does not promise a local TLS `close_notify`. Incoming TLS
truncation remains an error, not clean peer closure. Repeated Close is safe.

## Executed campaigns

All final campaigns completed with unchanged input-hash verification. Test
negative exits are intentional and checked for the actual transport/TLS error,
not accepted merely because the process failed.

| Campaign | GNAT 14.2.0 | GNAT 16.1.0 |
| --- | --- | --- |
| OpenSSL WSS consumer + transport + Sleet | PASS | PASS |
| mbedTLS WSS consumer + transport + Sleet | PASS | PASS |
| Full TASK100 plaintext matrix, OpenSSL dependency graph | 24/24 cells | 24/24 cells |
| TASK100 local scenarios | 1,044 | 1,044 |
| TASK100 pinned plaintext Sleet | 6/6 | 6/6 |

Each of the four WSS campaigns executes:

* Six minimal-consumer scenarios: idle, deaf, resumed sink, untrusted CA,
  wrong hostname, and truncated TLS. AHT backpressure accepts **1,802,240**
  plaintext bytes; the sink validates a position-dependent repeating 0..250
  pattern and SHA-256, preventing same-size duplicated/reordered chunks from
  masquerading as success. Accepted counts are bounded to 16 KiB per call;
  20 stalled flushes retain pending ciphertext exactly. Abortive cleanup and
  repeated abortive cleanup are timed.
* Thirteen focused Poll/security scenarios: idle, ciphertext fragmentation,
  three TLS records batched into one socket send, two WebSocket frames in one
  record, payload-exact Ping/Pong, exact Close response, controlled EAGAIN plus
  three-byte ciphertext sends, two Pongs then Close queued together under that
  backpressure, a peer that never reads, invalid `owp`, untrusted CA, wrong
  hostname, truncation, and 20 failed-handshake cleanup cycles. Several of these
  properties share a scenario. Poll calls have a 200 ms liveness backstop;
  this is not a real-time latency guarantee.
* Thirty-seven TASK100 scenarios over verified WSS: header rejection, framing
  rejection, UTF-8/fragments, message bounds, publication failure, generated
  handlers and ownership, and 30 resource lifecycles with zero FD/task growth.
* TLS-generated output with zero application plaintext, independently at the
  AHT API and through **actual OMS Poll**. A TLS 1.2 HelloRequest causes the
  engines to queue refusal output (OpenSSL 31 bytes, mbedTLS 23 bytes). The
  server observes the refusal; then the fatal renegotiation result is rejected.
* Real pinned Sleet through the test-only TLS terminator, including teardown.

The control-output interposer is test-only and never linked to production.
It uses real sockets and TLS while forcing EAGAIN for 300 ms followed by
three-byte ciphertext writes. Tests require nonzero blocked/partial counters
and peer-side payload/order validation. Separate consumer tests establish
**natural** kernel backpressure against a peer that stops reading, then recover
and validate every accepted byte. These are distinct forms of evidence.

The existing plaintext matrix retains both namespaces, portable/lossless
profiles, Normal/Check/Ignore policies and O0/O2: **48 cells, 2,088 scenarios,
24 Float64 probes, and 12 pinned-Sleet sessions**. No unrelated full CI campaign
was run. Thirty-eight dependency/evidence unit tests ran: 35 passed and three
existing external-artifact tests skipped (not used as interoperability evidence).

## Real OMS/Sleet evidence

Pinned Sleet's server uses `WebSocketStream<TcpStream>` directly and has no
native TLS listener in this revision. The test-only terminator forwards bytes
to its unchanged listener. Ada verifies `localhost` against the explicit test
certificate. The generated model, API and codec come from the CLI without
source modifications; separately generated model/API files are byte-identical.

All four WSS runs record:

1. Validated TLS and `owp` upgrade.
2. INIT, `+OK`, INFO and Active state.
3. SUB for two generated API handlers, with ownership checks.
4. Generated PUB and real Sleet MSG delivery to typed A/B Ada handlers.
5. Payload validation: Count=42, Label=`task100`, temporal codec round trip.
6. UNSUB, publication after UNSUB without another delivery, second UNSUB.
7. Rejected-topic server error (transmission is not server acceptance).
8. WebSocket close and resource teardown; terminator authenticates TLS EOF.

The completion marker is
`TASK100_GENERATED_CONSUMER_COMPLETE typed_A=1 typed_B=1 rejected=1`.
This is the generated TASK100 schema, **not** a claim of real-UCI WSS coverage.
Plaintext real Sleet interoperability is independently retained above.

## AHT suitability matrix

| Capability | Result | Evidence |
| --- | --- | --- |
| Nonblocking TLS receive | PASS | Four API idle tests; Poll idle/fragments/batched-record tests |
| Nonblocking TLS plaintext acceptance | PASS | Accepted counts, 16 KiB bound, verified patterned sink |
| Partial ciphertext retention | PASS | Stable Pending_Output across 20 refusals; forced three-byte sends |
| Resumable output flushing | PASS | Natural stalled sink resumes, exact byte count/pattern/hash |
| TLS-generated control output | PASS | HelloRequest refusal observed at API and through OMS Poll, plaintext=0 |
| WebSocket Ping/Pong | PASS | Exact 125-byte payload including NUL; ordered two-Pong burst |
| WebSocket Close | PASS | Exact close payload, delayed teardown until response flush |
| Backpressure recovery | PASS | Natural TLS sink plus controlled WSS response-order tests |
| Certificate verification | PASS | Trusted localhost accepted; untrusted and IP/hostname mismatch rejected |
| Real OMS/Sleet interoperability | PASS | Four verified WSS generated-handler sessions through terminator |
| Existing plaintext regression | PASS | 48 cells, 2,088 local scenarios, 12 pinned-Sleet sessions |

**NOT TESTED:** runtime `Needs_Read`, public Internet/system trust-store success,
CA_PEM integration, IPv6 URL syntax, remote deployment, other operating systems,
other TLS implementations, and hosted exact-final-head CI. No readiness claim
extends to those configurations or branches. Independent review is required.

## Commands and retained evidence

Machine-readable evidence is in `docs/evidence/task108r1`. Each campaign JSON
maps original relative artifact names to their exact text (not just a summary).
`manifest.json` records full absolute commands, pins and scratch paths;
`oms-inputs.json` and each campaign's `input-sources.json`/`sources.json` retain
SHA-256 identities. The full build trees and raw output remain under
`/home/zboll/task108r1-scratch`. Test private keys are deliberately not committed.
The dependency checkout itself is not built in or modified: immutable archived
copies are used. Unmodified Sleet source/cleanliness and binary hash are recorded.

From the worktree, the focused command pattern (all paths absolute in the
manifest) is:

```sh
python3 -B /home/zboll/git/ams-gra-codegen-oms-task108r1/scripts/task108/qualify.py \
  --scratch /home/zboll/task108r1-scratch/acceptance-16-openssl \
  --setup /home/zboll/task106-scratch/setup16 --compiler 16 --backend openssl \
  --aht /home/zboll/git/aht-tls-003 \
  --codec /home/zboll/task106-scratch/deps/websocket-ada \
  --generator /home/zboll/task106-scratch/target/release/ams-gra-codegen-oms \
  --sleet /home/zboll/task106-scratch/sleet
```

Use a new scratch path for each invocation; substitute compiler/setup 14 and/or
backend mbedtls for the other three cells. TASK100 commands and all individual
consumer, compiler, peer and Sleet commands are retained in the evidence.

Existing transport qualification drivers and evidence validators were updated
to the same immutable AHT pin and single-owner GPR import. This is necessary to
avoid breaking the other existing Ada WebSocket consumers, not expansion into
their unrelated test campaigns. Existing transport CI jobs only gain
`libssl-dev`; runner/concurrency/scheduling policy is unchanged.

## Failure evidence and limitations

`development-failures.json` preserves earlier failed attempts:

* Initial sink fixture aborted immediately after local flush, before peer
  accounting: reset was observed. It now requires a peer acknowledgement.
* Initial Sleet terminator truncated TLS after full typed delivery; the client
  correctly rejected it. The terminator now sends authenticated close_notify.
* First plaintext campaigns rejected their final hash guard because sources
  were edited during execution. They are not completed qualification evidence.
  The final frozen campaigns pass that guard.
* Initial WSS send-failure fixture reset before TLS upgrade consumption; the
  reset now follows the upgrade. Initial mbedTLS-14 excessive-header fixture
  saw BrokenPipe because the client rejected before the oversized write ended;
  that expected peer-side outcome is allowed only for excessive headers and
  the precise client rejection is still required.

Actionlint 1.7.12 rejects the baseline's existing `concurrency.queue: max` keys.
Baseline and candidate diagnostics were compared and are identical; no new
diagnostics were introduced. No concurrency policy was changed to silence it.
`git diff --check` passes. This is not an unconditional actionlint-pass claim.

The matrix establishes the requested downstream acceptance contract in the
listed configurations. It is not an authorization to alter fork PR #2 or
submit an AHT upstream output PR, and does not replace independent review.