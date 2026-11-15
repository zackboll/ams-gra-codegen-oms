# Ada UCI peer-closure subset acceptance

## Decision and scope

**PEER_CLOSURE_SUBSET_QUALIFIED** applies to the independently reviewed source
`c2643594f41d3b7f30a2d6503b2e023d969aab0f`, tree
`220efe0d2b0455589d4aed416427a7cdba468899`, with the exact five-cell inputs and
executables in [the compact ledger](evidence/task115/r2d/acceptance-ledger.json).
This documentation records existing qualification, not a new five-cell runtime
qualification, merge approval, or completion of original TASK115.

The standalone receive-only Ada application uses authenticated, hostname-verified
WSS and negotiated OWP. Production-generated input-only Subscriber APIs decode
`LogMT`, `RDMA_InitializeSetupMT`, and `SurvivabilityRiskLevelMT` for authentic
UCI 2.6 and representative UCI 2.5. It never publishes application messages.

Recovery is permitted only after authenticated TLS `close_notify` or a valid
WebSocket Close whose required local echo has drained, and only after full
initialization: verified upgrade, INIT acknowledgment with valid INFO, and all
three subscription acknowledgments. Local output acceptance is not peer receipt.
Initial connection/setup failures and failed replacements are terminal, even if
attempts remain. Ambiguous failures are never classified using exception text.

Unsupported: TCP-reset or generic abrupt-network recovery; TLS-integrity or
protocol-failure recovery; replay of missed messages; offline/durable queues;
losslessness; exactly-once delivery; speculative retries after replacement
failure. Disconnected messages are lost.

## Deadline, budget, and epoch ownership

One absolute monotonic deadline is computed after argument/CA-file preparation,
before the first connection. Every epoch receives that same deadline before Open.
It is not renewed by initialization, successful receipt, reconnection, or delay.
The transport's shorter operation cap can expire first. The process-wide recovery
budget is finite (0–8, default 0), decremented before each replacement, and never
replenished. Recovery delay is 50–2000 ms (default 250), bounded by the deadline.

Each lexical epoch owns a fresh transport, connection, receivers, three bindings,
dispatcher list, SIDs, and event counters. Retirement and finalization finish
before the supervisor creates a replacement; no borrowed object graph survives.
Lost events, decode/handler errors, OWP errors, and ambiguous transport errors
are terminal. There is no application background worker or runtime dependency
on the qualification harness.

Established requested-duration expiry completes with exit 0. Initial/replacement
setup expiry or expiry during disconnected recovery delay fails with exit 1.
Transport errors do not become success merely because time has expired. Argument
errors exit 2. This is a bounded I/O contract, not hard real-time preemption of
arbitrary OS scheduling, filesystem access, or synchronous stdout consumers.

## Output and backpressure

INIT and SUB use the common bounded blocking Send path; uncertain/incomplete
writes fail closed. SUB acknowledgment is still required separately. Pending
Pong and Close output advances through bounded nonblocking Poll operations.
Queued controls retain order, and a pending Close echo does not authorize
recovery until its required output drains. The existing transport caps control
and TLS output buffers; this PR changes neither its interface nor behavior.

Intentional duration shutdown may release abortively. **Pending Pong or Close
output does not guarantee delivery at intentional duration expiry.** An
undelivered control at that shutdown does not alone make established duration
completion fail. Diagnostic stdout is synchronous/flushed; blocking stdout is
not independently qualified as deadline-preemptible application output.

## Independent qualification

R2A recovered the pinned environment; R2B and R2C retained their historical
`QUALIFICATION_GAP` decisions. R2D independently reconciled those observations
against the narrower existing receive-only contract without editing receipts:

| Evidence | Accepted / checked |
| --- | ---: |
| Real-Sleet campaign cases | 30/30 |
| Real-Sleet controls | 55/55 |
| Initial TLS deadline probes | 5/5 |
| Supplemental terminal-error probes | 20/20 |
| R2C standalone blocked-call/control observations | 60/60 |

Each of GNAT16/OpenSSL/UCI2.6, GNAT16/mbedTLS/UCI2.6,
GNAT14/OpenSSL/UCI2.6, GNAT14/mbedTLS/UCI2.6, and
GNAT16/OpenSSL/UCI2.5 contributed 6 cases, 11 controls, 1 initial TLS probe,
4 supplemental probes, and 12 R2C observations. Injected refusal exercises the
actual standalone call path, but **injected EAGAIN is not genuine kernel
backpressure**. SUB coverage includes source-equivalent common Send paths and
genuine SUB/ACK captures, not a SUB-targeted socket-fault execution.

### Separate R2D genuine kernel-pressure experiment

One additional GNAT16/OpenSSL/UCI2.6 experiment observed **4,546 actual kernel
EAGAIN attempts**, socket transmit-queue occupancy (maximum observed procfs
queue 7,379 bytes), and one established duration-completion epoch. There was no
unauthorized reconnect. After reap, the peer received an ordered prefix of
**76 Pongs** out of 512 supplied Pings. No claim is made that undelivered Pongs
reached the peer. Duration expiry performed normal abortive cleanup; the
post-reap TLS EOF was not authenticated peer closure or recovery permission.

This single real-pressure experiment is **not five-cell genuine-pressure
qualification** and is not a 61st R2C observation. No kernel-blocked INIT/SUB or
selector park/wakeup claim is authorized. The receiver's Poll pressure path is
nonblocking, not a blocking-selector measurement. The first pressure fixture
attempt rejected expected post-reap abortive EOF and is retained as unsuccessful;
only the corrected fresh run counts as the accepted proof.

## Exact identities and reproduction

GNAT versions are 14.2.0 and 16.1.0; GPRbuild package 26.0.1 reports binary
26.0.0; GNATCOLL/GNATCOLL_MINIMAL are 26.0.0. The external harness used Python
3.13.5 and websockets 15.0.1. Commit/tree identities, executable SHA-256 values,
the 53 build-input hashes, schema hashes, and evidence/tool hashes are in the
compact ledger. Dependencies remain:

| Source | Commit |
| --- | --- |
| AHT | `827b0c17dd964e935f8e88912ce0611ee92b6f16` |
| websocket-ada | `765d9cd612838eb00645037977c8cc9af7171c3e` |
| Sleet | `e38f61d8ce0d75c8508434a52f2ed77c69cf6a3b` |
| UCI 2.5 | `093610b7753944059360d3236770ab446d039556` |
| UCI 2.6 | `78eb61b6112c8bffa40820c33124b57787fc5bd9` |

Use the portable variable-based build/campaign commands in the unchanged
[application README](../examples/ada-uci-client/README.md#independent-build)
and [evidence reproduction notes](evidence/task115/r2d/README.md). Build only in
new external scratch directories with authentic schemas and those exact pins.
No packages need be installed or dependencies resolved in the recovered
environment. Rehash identical archived inputs/evidence rather than rerunning
expensive campaigns solely to duplicate them. Changed executable-relevant inputs
require an explicit impact assessment and fresh qualification; old acceptance
cannot automatically be transferred.

The publication branch starts at main
`c6ddb747d22ec3c9092a87755465c63a52ebad84`; ordinary merge
`ed77046c002730ef623bf95441ffe15a95aa9721` has that first parent and the
qualified candidate as second parent. Its pre-documentation tree equals the
qualified tree exactly. New acceptance documentation is outside all three
build-input directories; the original 53-file input map is unchanged.

## Non-gating limitations and original task relationship

The original TASK116 raw archive is unavailable in the recovered environment.
Its compact receipts and unchanged production input hashes are supporting
historical evidence, not a claim of a newly verified full archive. Available R2B
transport raw captures were reparsed for eight observations. No arbitrary
heap-leak freedom, hard real-time scheduler guarantee, or cancellation of the
pinned detached OS DNS resolver is claimed. Raw external qualification tooling
and captures require the preserved evidence bundle; they are not committed here.

[PR #203](https://github.com/zackboll/ams-gra-codegen-oms/pull/203) remains the
broader unfinished original TASK115 task. All original historical receipts,
including every `task115_acceptance=false`, are preserved verbatim. This named
subset does not relabel them or complete original reset-recovery requirements.
The ordinary merge preserves implementation authorship and candidate ancestry.
Consequently, eventually merging this new branch makes PR #203's commits
reachable from main; GitHub may then change that PR's state. This task leaves it
open and unmerged and does not promise it will remain open after a future merge.
Closing or superseding it requires a separate explicit disposition.

No unpublished AHT-TLS-005 work is integrated or submitted upstream. Pinned AHT
and existing transport interfaces remain unchanged. This PR excludes shared
production WebSocket/OMS runtime changes, AHT/Sleet changes, dependency-pin or
authentic-schema changes, other-language production features, CI policy changes,
generated artifacts, binaries/objects, private keys, raw captures, and unrelated
historical evidence. PR #208 is untouched. Independent review and hosted PR smoke
and trusted signals on the actual published HEAD remain required; no self-approval,
auto-merge, or merge is authorized by this acceptance record.
