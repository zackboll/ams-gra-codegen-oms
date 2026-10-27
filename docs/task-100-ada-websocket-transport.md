# Task 100 — Ada WebSocket transport qualification

## Baseline

Isolated worktree `/home/zboll/git/ams-gra-codegen-oms-task100`, branch
`feature/100-ada-websocket-transport`, starts at commit
`4978c1c5728d062b9ea64bbcdb39aa96b18361a3`, tree
`63a37eaeafc38bd7f6ddfadf4dfc99919393000a`. Origin was fetched first;
PR #115 is merged at that commit. Other worktrees and pending PRs are not
implementation targets. No reset, rebase, amend or force push is authorized.

## Phase 1 — source inspection and preliminary decision

These are qualification findings, not an interoperability claim.

| Candidate | Inspected source identity | License | Finding |
| --- | --- | --- | --- |
| AdaCore AWS | `6522c3fed828c6a53b8483f0329dc273f38c58f7` | GPL-3.0 with runtime exception (consult individual source headers and COPYING files) | Real client, but public Connect has no caller headers/subprotocol parameter. Poll explicitly blocks after partial frame arrival, even with zero timeout. Not usable unchanged for this contract. |
| kokhlo/websocket-ada 0.3.0 | `765d9cd612838eb00645037977c8cc9af7171c3e` (Alire origin) | MIT | Pure frame/handshake/UTF-8 utilities: **no TCP implementation**. Request builder does not offer subprotocol; response validator does not check it, extensions, duplicates or strict header syntax. Frame decoder accepts non-minimal extended lengths. Do not use it alone as an RFC-qualified connection. |
| ovenpasta/aht | `664d5ddeb568626b2f9ff66054c7091894b3b8d4` | Apache-2.0 | High-level client embeds a persistent receiver task and callbacks; excluded. Standalone Frames/UTF8/Bytes/Random packages are usable independently: incremental decode, header-time allowance, minimal lengths, reserved opcode/RSV checks, fragment sequencing and control validation. |

AWS inspection: `aws-net-websocket.ads` 151–185,
`aws-net-websocket.adb` 90–138 and 407–457,
`aws-net-websocket-protocol-rfc6455.adb` 132–174. Alire AWS 25.2.0 also
constrains GNATCOLL and XML/Ada to ~25.0.0, incompatible with this runtime's
pinned GNATCOLL 26 graph without a separate dependency/build qualification.
This is not a claim that AWS upstream cannot be adapted.

Both compiler versions execute websocket-ada's upstream frame/UTF-8 and
handshake suites successfully. Both execute AHT's standalone frame and UTF-8
suites successfully. An initial AHT-14 launch raced scratch-directory creation
and failed before compilation; the separately recorded retry passed. Fresh
isolated Task 096 provider setups compile and run GNATCOLL 26.0.0 on GNAT
14.2.0 and 16.1.0, GPRbuild 26.0.1, Linux x86-64 glibc 2.41.

### Candidate contract assessment

| Concern | AWS inspected API/implementation | websocket-ada 0.3.0 | Selected standalone AHT closure + connection layer |
| --- | --- | --- | --- |
| Compiler/build | Full AWS not built after public-API rejection; no compatibility claim | Upstream suites execute on 14.2/16.1; GPR dynamic library has no external dependencies | Upstream frame/UTF-8 tests and full selected source closure execute on both, all policies/O0/O2 |
| Network/client | Real HTTP/TCP client, no caller subprotocol/header hook | None; explicitly pure text/bytes | GNAT.Sockets, explicit socket owner, no DNS for loopback |
| Receive contract | Poll blocks on partial frame; public callbacks | Incremental single-frame decoder only | Incremental independent codec with bounded synchronous driver |
| Text/UTF-8/fragments | Read_Message accumulates unbounded Unbounded_String and validates UTF-8 on completion | UTF-8 validator supplied; no message reassembly | Bytes preserved; bounded adapter accumulation; independent complete-message validator |
| Controls | AWS internal state/receive/control operations | Validates control structure; no I/O/control driver | Independent control validation, exact bounded Pong/Close driver |
| Mask/entropy | Client mask from AWS.Utils.Random, backed by GNATCOLL.Random in inspected source | Encoder applies caller mask; Generate_Key uses Ada.Numerics.Discrete_Random, not selected | Independent encoder applies fresh OS-random mask/nonce; entropy failure raises, no fallback |
| Size/partial I/O | High-level receiver has no Task 099 allowance; socket API needs separate qualification | No caller-configured per-message allowance or socket partial-I/O handling | Decoder allowance checked at header time; bounded input/fragment/output; partial writes loop/all-or-error |
| Ownership | Own HTTP/socket/protocol state, callback lifecycle | Value containers, no connections | Limited controlled connection; independent controlled byte buffers; explicit/finalization cleanup |
| TLS | AWS has SSL implementations; not executed or security-qualified here | None | AHT has OpenSSL peer verification/default trust paths and identity checks in source, but excluded from this closure; no TLS execution claim |
| Dependencies/license | Published AWS 25.2 requires ~GNATCOLL 25; GPL/runtime exception distribution review needed for selected version | MIT, no GNATCOLL dependency | Apache-2.0 standalone AHT + MIT handshake utilities, no GNATCOLL version conflict; existing GNATCOLL 26 tested |

The source inspection includes the build manifests, actual frame decoders,
handshake request/validation, UTF-8 validators, entropy source, controlled
buffer allocation/finalization, and AHT TLS verification call sites. No full
AWS or high-level AHT build success is inferred from the standalone tests.

### Prototype gate

A scratch executable uses GNAT.Sockets, unmodified AHT framing/UTF-8 and OS
randomness, and websocket-ada's request/accept utilities behind strict OWP
handshake policy. A deterministic Python socket peer verifies byte-fragmented
HTTP upgrade, requested/negotiated `owp`, rejection of missing/wrong protocol,
wrong accept and invalid Upgrade; masked client text; fragmented server text
with interleaved Ping; exact binary-safe Pong payload; 100 idle polls; and
connection release. The initial GNAT 14 execution passes all five scenarios.
This is only a prototype, not the mandatory adversarial suite or real Sleet.

**Provisional selection:** thin single-threaded GNAT.Sockets connection layer
over independently tested AHT framing, not a new frame encoder/decoder.
Use only the standalone dependency closure, never AHT.Websocket.Client.
Pin source identities and preserve upstream license notices. Qualify the full
transport before claiming production readiness. No TLS implementation is
selected: reject `wss://` and restrict plaintext to explicit loopback URLs.

## Architecture and ownership

`ada/oms-lacal-runtime/oms_lacal_websocket.gpr` is an **optional** companion
project. The original core project and Task 099 protocol/ownership sources
are unchanged. Set `OMS_WS_AHT_SOURCE` and `OMS_WS_CODEC_SOURCE` to the exact
pinned source checkouts above. The qualification runner fetches and verifies
these identities itself. `oms_websocket_dependencies.gpr` explicitly lists
the standalone source closure; neither upstream connection client, receiver
task, TLS, HTTP stack nor callback machinery can enter this project.

`OMS_LACAL.Transports.WebSocket.Client` is a limited controlled implementation
of the existing `Transports.Transport` interface. It owns its GNAT socket,
unconsumed wire bytes, fragment accumulation and close state. `Open` performs
the upgrade; `Protocol` reports `owp` only after validated negotiation. The
existing connection borrows it and continues to own all OWP state and events.
Only the typed binding invokes generated handlers. No numeric parsing,
serialization or envelope rewriting occurs inside the transport.

Both source libraries are permissively licensed (MIT / Apache-2.0), without
GNATCOLL dependencies. The project uses GNATCOLL 26 only through the unchanged
OMS support/core closure. The AWS alternative's published Alire version pin
is not installed into this graph. No vendored or modified upstream protocol
implementation is introduced.

## RFC 6455 behavior and limits

* URLs: explicit `ws://127.0.0.1:<port>[/path?query]` or `localhost` (dialed as
  IPv4 loopback without DNS). Port zero, other hosts, controls, spaces,
  fragments, schemes, credentials and IPv6 URL forms are rejected. Maximum
  URL length 2,048 bytes. Remote plaintext is deliberately unsupported.
* Upgrade: HTTP/1.1 status 101, Upgrade websocket, Connection Upgrade token,
  SHA-1/Base64 accept computed from a fresh OS-random 16-byte nonce, exactly
  one case-sensitive `owp` response protocol. Missing/wrong/duplicate protocol,
  duplicate required headers, malformed names/values, any extensions,
  incorrect accept, non-101 responses and redirects fail closed. No redirects
  or compression are followed/negotiated. Maximum response header block
  16,384 bytes, including CRLFs; one-byte reads avoid consuming first-frame
  bytes during handshake. Required header interpretation uses the pinned
  library, with stricter syntax/duplicate/subprotocol checks in the adapter.
* Send: validate UTF-8 and whole-message size **before** frame construction or
  publication, then encode one FIN text frame. A fresh four-byte mask comes
  from AHT's OS CSPRNG for every frame. GNAT partial writes are advanced until
  complete. Any uncertain/failed write closes the socket and raises
  `Transport_Error`; sending is never server acceptance. There is no retained
  application output queue. A validation failure publishes nothing and
  leaves an already-open connection usable.
* Poll: socket remains nonblocking. Each call has at most one zero-time
  readiness check, one read of at most 4,096 bytes, and one consumed frame.
  The codec may inspect the pending bytes before and after that read, but
  consumes only one frame. CPU/copy work is bounded by the fixed message limit,
  not a real-time guarantee. Partial bytes and fragments remain private.
  An incomplete message or control-only progress returns Idle. A complete
  validated UTF-8 text message returns Frame_Ready; controls never enter OWP.
* AHT validates RSV, opcode, minimal extended lengths, 64-bit length high bit,
  control FIN/125-byte bound, continuation sequencing and close structure.
  The adapter rejects server masks at the first two bytes, binary messages,
  invalid close status/reason and invalid complete-message UTF-8. UTF-8 can
  span fragments; Ping may interleave. Pong echoes the exact arbitrary-byte
  control payload, not a text conversion.
* Maximum **complete** message is Task 099's unchanged 1,048,576 bytes, across
  all fragments. Remaining allowance is supplied to the independent decoder,
  which rejects the declared oversize at header time, before buffering its
  body. Input is bounded by 1,048,576 + 4,110 bytes (message, maximum frame
  header and one read's spill); fragment contents by 1,048,576. Each control
  payload is at most 125 bytes, each output control frame at most 131 bytes.
  AHT buffer capacity grows geometrically, so allocation capacity may exceed
  logical contents (under twice its finite configured ceiling). Temporary
  codec/copy arrays add a bounded constant number of message-sized buffers.
* Poll control replies use one nonblocking write, with no pending-output
  queue or wait. If a tiny Pong/Close cannot be completely written, the
  transport closes and raises rather than risking protocol corruption or
  blocking receive progress. This is a fail-closed backpressure policy, not a
  guarantee to survive arbitrary output congestion.
* EOF returns Peer_Closed and releases the socket. A valid peer close is
  echoed exactly once and released. Explicit Close sends 1000, polls up to
  250 ms for the peer, and releases even if the peer never answers. Its close
  write has the normal two-second write deadline. Finalization drops the
  descriptor without waiting or attempting a close handshake. Close is
  idempotent. Failed Open also releases the descriptor.
* Open uses two-second connect and upgrade budgets (the upgrade clock starts
  at Open entry); Send uses a two-second whole-write budget. These are
  implementation deadlines, not real-time guarantees. No retry, reconnect,
  heartbeat scheduler or automatic receive progress exists. AHT retains one
  close-on-exec entropy-source descriptor per process, not per connection;
  resource tests warm it before measuring lifecycle FD growth.

### Error classification

WebSocket handshake/framing/encoding/size/socket errors are transport
exceptions, close the affected connection (except pre-publication Send
validation), and never masquerade as OWP server acceptance/rejection. The
unchanged connection stops on transport exceptions. Valid OWP `-ERR` remains
`Server_Error` with its original details, distinct from transport failure.
Application malformed envelopes/codec errors are handled by existing typed
dispatch and need not tear down a valid WebSocket session.

## Executed real pinned Sleet result

The first server attempt failed before listening: Task 099's schema lacks a
root `version` attribute required by Sleet. The separately identified
`lacal-websocket.xsd` fixture adds `version="000.1.0"` without changing payload
types or message identities. Fresh CLI generation from the versioned schema
produced byte-identical model/API/codec sources to the Task 099 fixture.
This is an authorized synthetic schema, **not real UCI**.

`scripts/task100/real_sleet.py` checks exact clean Sleet commit/tree, creates
loopback configuration and service authorization (`task100`, only
`common-topic`), starts the unmodified pinned binary, runs the generated Ada
consumer, retains command/exit/stdout/stderr/config, and terminates/joins the
server. Local GNAT 14 execution completed:

```
GENERATED_UPGRADE_OWP_COMPLETE
OWNERSHIP_WEBSOCKET_COMPLETE
ORIGINAL_ENVELOPE {"MessageA":{"At":"12:34:56.5000Z","Count":42,"Label":"task100"}}
ORIGINAL_ENVELOPE {"MessageB":{"At":"12:34:56.5000Z","Count":42,"Label":"task100"}}
REJECTED_TOPIC_TRANSMITTED_NOT_ACCEPTED
OWP_EVENT SERVER_ERROR Illegal-State service 'task100' is not authorized to publish to topic rejected-topic
TASK100_GENERATED_CONSUMER_COMPLETE typed_A=1 typed_B=1 rejected=1
TASK100_REAL_PINNED_SLEET_COMPLETE
```

The actual generated Publisher and Subscriber generic packages are
instantiated. INIT +OK and INFO, verbose SUB/PUB acknowledgment, independent
typed A/B routing despite one shared payload type, exact envelopes, UNSUB/no
further delivery, and an observable rejected-topic error all execute. A
foreign typed binding is rejected before wire traffic; both subscriptions
remain active and subsequently deliver. Generated handlers validate integer
42 and string `task100`, not merely raw JSON receipt.

Qualified non-OAM global identity is exercised against the local peer.
Pinned Sleet's existing bare-global-name limitation is already recorded in
Task 051: no qualified-name Sleet success or workaround is claimed. No
separate real UCI round trip has been qualified here.

## Qualification and reproducibility

Task-owned evidence root: `/home/zboll/task100-scratch/`. Failed/superseded
attempts remain there and do not count as completed campaigns. Source hashes
detect input edits during a campaign; an incomplete campaign has no successful
`result.json`. Each command's exit and streams are retained.

```
python3 /home/zboll/git/ams-gra-codegen-oms-task100/scripts/task096/setup.py \
  --scratch /absolute/new/setup --compiler 14
python3 /home/zboll/git/ams-gra-codegen-oms-task100/scripts/task100/qualify.py \
  --scratch /absolute/new/campaign --setup /absolute/new/setup --compiler 14 \
  --generator /absolute/path/to/ams-gra-codegen-oms \
  --sleet /absolute/path/to/unmodified/pinned/sleet-checkout
```

Omit `--sleet` only for the deterministic Fast CI gate; repeat with compiler
16 and its separate setup. The Sleet checkout must already have been built
using `cargo build --release --locked -p sleet` under its pinned toolchain.

The transport matrix requires GNAT 14.2.0 / 16.1.0, Normal / Check / Ignore,
O0 / O2, OAM / qualified, portable / lossless: 48 cells. Portable cells require
43 real-socket/URL scenarios; lossless cells additionally require the
four-value Float64 transport probe. Numeric inputs include negative zero,
minimum subnormal, maximum finite and a halfway boundary; the peer compares
the echoed original numeric lexemes byte-for-byte, and generated codecs check
binary64 values, signed zero and encode/decode. No second numeric parser is
introduced. Invalid TimeZulu, malformed/duplicate envelopes and invalid typed
payloads precede correct delivery in the same connection.

Other scenarios include failed negotiation/accept/upgrade/extensions,
byte-split and deliberately stalled partial input, coalesced controls/text,
UTF-8 split across fragments, interleaved Ping/exact binary-safe Pong, close,
EOF, reserved opcodes/RSV, masking, invalid UTF-8/binary, declared/frame and
aggregate oversize, non-minimal/high-bit lengths, fragment sequencing and
bad controls/close. A test-only Linux send interposer forces successful short
TCP writes and must report a nonzero short-write count. It is never linked
into the transport. Send reset failure and pre-publication validation are
separate cases. Thirty reconnect/close cycles require stable `/proc/self/fd`
and task counts after entropy-source warmup; finalization is independently
observed as peer EOF. No transport receive task exists.

Fast CI adds only the local transport campaign; Deep CI adds the unmodified
pinned-Sleet campaign. Each new job uses a pinned Debian image, pinned Alire,
compiler/GNATCOLL graph and source dependencies, checks exact PR head/tree and
clean checkout, requires all cells and completion markers, preserves command
failures through pipefail, and uploads source/compiler/execution artifacts.
Existing jobs, runners, concurrency and Fast/Deep coverage remain unchanged.

## Security and unsupported features

Plaintext loopback is insecure for remote deployment and does not protect
against another local process or compromised host. `wss://` always fails;
there is no TLS certificate-verification bypass or silent downgrade. The
presence of TLS in AWS/AHT does not qualify it in this adapter. TLS with
hostname/chain verification, remote URLs, proxies, authentication policy,
compression/extensions, binary application messages, automatic reconnect,
background receive and cross-platform support remain unsupported. No thread
safety, real-time or automatic progress claims are made. Future reconnect
must be explicit and define OWP reinitialization and subscription replay;
future deadline policy must preserve uncertain-write closure and bounded Poll.

## Final status

The fresh `release14` and `release16` campaigns completed successfully:

| Gate | Executed local result |
| --- | --- |
| Transport/compiler matrix | **48/48 cells**, 2,088 local scenarios (1,044 per compiler), no required-case skips |
| Real pinned Sleet | **12/12 runs**, both compilers × Normal/Check/Ignore × O0/O2; generated OAM typed A/B handlers and rejected topic |
| Float64 framing | **24/24 probes**, each four values and four byte-exact lexeme echoes, both namespaces |
| Task 099/R1 | **48/48 cells**, **2,760 checks**, existing mock ownership/safety suite unchanged |
| Task 098 TimeZulu | **36/36 cells**, **5,616 documents**: 1,368 accepted / 4,248 rejected, both compilers/namespaces and all three runtime profiles/policies |
| Rust fmt/check/clippy | All locked workspace gates pass |
| Locked Rust workspace tests | **1,480 passed, 0 failed, 8 existing external-evidence tests ignored**; ignored tests are not interoperability evidence |
| CI split/layout | Structural guards, 476-check split harness and layout adversarial tests pass |
| actionlint / whitespace | Pass |

The source/API/codec copies used by these completed campaigns are hash-recorded
and unchanged from actual CLI output. All original runtime core, Ada generator
and JSON support files remain byte-identical to the baseline. Failed earlier
campaigns were caused by qualification-harness edits or the documented harness
issues; none has a successful completion record. Additional inherited codec
and full Float64/TimeZulu campaigns are still running; hosted exact-final-head
evidence is also required. Local success alone is not review readiness.