# Task 099 — Ada LA-CAL runtime core

## Baseline and implementation contract (written before implementation)

Branch `feature/099-ada-lacal-runtime-core`, isolated worktree
`/home/zboll/git/ams-gra-codegen-oms-task099` starts at commit
`a03684eb963e1eaa8a9df3db17c9d1783b7add03`, tree
`af76aa248a8dcef9cec23dc3b83ee5ce4956c4e1` after normal origin fetch.
PR #102 merged as `d1fd77cf007e1c9845dd44b70df99226c554eab2`;
PR #104 merged as `a849b87d94948980133038ba909c0b6528cadde6`.
Both merges are ancestors of the baseline. Other worktrees are not modified.

### Authority

Read roadmap Phase 3, Tasks 048/049/097/098, Ada generator, guarded and
lossless JSON support, compiled consumer harnesses, and Rust runtime.
Protocol authority is OMSC-SPC-013 Rev B at `open-arsenal/oms`
`726272bd0390982a759c91a9cf4e13b81c2b510b`:

* Official DOCX SHA-256:
  `b1c3c07872570fb4f2c84fd819076b1b588148ef28225f5a8e23433efe94b1a7`.
* Unofficial Markdown SHA-256:
  `5798da43991d5b554ed7575fa55c350aacece483d36841738e57db123f4ee98f`.

Both artifacts were fetched at that revision and hash-verified locally.
Pinned Sleet `e38f61d8ce0d75c8508434a52f2ed77c69cf6a3b` and the existing
Rust runtime are references, not new dependencies or authority for extensions.

### Planned API, ownership and failures

* Transport supplies whole OWP text frames, nonblocking poll, and idempotent
  close. Send must be all-or-error; a partial/uncertain transport write fails
  the connection. No WebSocket/TLS implementation is introduced.
* A limited controlled connection borrows transport through a checked access
  discriminant. Transport outlives connection. Finalization closes it.
* INIT uses version `1.0`, caller schema and service ID. Only INFO establishes
  initialized state. One command may await acknowledgment in verbose mode;
  successful send is not acceptance. Pending SUB is not dispatchable until
  +OK. A rejected pending SUB has no handler mapping. Nonverbose SUB is locally
  active after send, with acceptance explicitly unknown: uncorrelated late
  -ERR cannot identify or retroactively reject a particular SID.
* SIDs are local monotonic integers, never reused; capacity is separately
  bounded. UNSUB removes local dispatch immediately, independent of peer ack.
* Runtime owns global identity/envelope and framing. Generated codecs own
  payloads only. OAM names are bare; other qualified names use `{uri}local`.
  Validate the original JSON before envelope extraction; count original root
  members, not DOM fields. Pass an original substring to Decode, preserving
  every number lexeme. Encode fully before invoking transport send.
* Receive progress is explicit, single-threaded and synchronous. Core poll
  returns a validated-SID delivery, not a retained callback. Application routes
  it to endpoint-specific typed bindings. Each binding uses checked named
  access conversion of the generated anonymous receiver parameter, before SUB.
  A receiver too short-lived for its binding must be rejected, never retained.
  No Unchecked_Access, Unchecked_Conversion, or raw address escapes.
* Typed bindings invoke handlers only after envelope and payload validation.
  Invalid documents and handler exceptions are observable separately and do
  not remove valid subscriptions. Reentrant dispatch/runtime operations are
  rejected. No automatic tasks, thread safety, reconnect, timeout progress or
  real-time/resource-exhaustion recovery is claimed.

## Qualification status

Local implementation qualification is complete. Exact-final-head hosted CI is
still required before independent-review readiness. WebSocket subprotocol
negotiation, TLS, real peer behavior and unmodified Sleet execution require a
separate transport qualification; no such interoperability is claimed here.

## Implemented architecture and generated binding

`ada/oms-lacal-runtime/oms_lacal_runtime.gpr` is a standalone reusable project
importing the existing portable JSON support project. Its six cohesive units
(root, Transports, Connections, Envelopes, Typed, Transports.Mock) contain no
schema-specific model, generated codec, WebSocket stack, TLS dependency or task.
The root owns one shared `QName` formatter. The formatter admits a bounded ASCII
local NCName subset and a nonempty valid UTF-8 namespace without OWP delimiters,
controls or braces. It does not normalize namespace URIs.

The executable template `tests/ada-lacal-runtime/runtime_consumer.adb` uses:

1. Actual generated `Model.SharedPayload`, not a handwritten surrogate.
2. Actual `Service_Codec.Encode_SharedPayload`/`Decode_SharedPayload` functions.
3. Actual generated `Service_API.Function_Io.Exchange_Out_{A,B}.Publisher`
   and `Exchange_In_{A,B}.Subscriber` generic instances.
4. One `OMS_LACAL.Typed` instance for each input endpoint's distinct Handler
   interface. Hook functions close over a binding/connection and forward the
   unchanged generated signatures; no generator modification is needed.

The synthetic XSD has two global messages (`MessageA`, `MessageB`) sharing one
payload type; the contract has two inputs and two outputs on one topic, with and
without a subscription group. The qualification driver generates both OAM and
`urn:task099` variants and adds a direct `xs:double` field for the lossless
profile. `--with-codec` model/API bytes are compared to model-only generation.

Connection `Poll` produces at most one SID-tagged delivery. The application offers
it to its bindings. A binding selects its own registered SID, validates against
its own stored QName (not caller-supplied delivery metadata), extracts the raw
payload, calls the generated decoder, and invokes the generated dispatching
Handler primitive. The connection synchronously borrows a processing callback
for this one call; it never retains callback access. The callback guard cannot
be manually released by application code.

## OWP frame, state and event contracts

* One whole transport text message is one OWP operation, without added CRLF.
  Outbound separators are single spaces. Inbound spaces and horizontal tabs,
  including repeated separators, follow §5.1.1.1.2. Newlines are not operation
  separators; valid JSON whitespace inside a message is preserved.
* `New_Connection -> Initializing -> Active -> Closed` is one-way. INIT is
  unique and first. Version is exactly `1.0`; schema version and service ID are
  explicit caller arguments. Topics, groups, service IDs and SIDs use the OWP
  ASCII identifier grammar, with a local 256-byte bound.
* Default verbose INIT needs `+OK` **before** INFO. Nonverbose INIT needs INFO
  alone. INFO validates required object/string fields, negotiated version,
  server identifier, UUID spelling and optional mapping/URL kinds. Valid later
  INFO is an information event; it does not reconnect or reconfigure routes.
* Verbose mode allows one outstanding command. Publish success is transmission,
  never server acceptance; `Command_Accepted` reports the subsequent ordered
  `+OK`. A second command while waiting raises `Runtime_Error` without send.
  A pending SUB is not dispatchable until +OK; -ERR removes its pending entry.
  This association relies on the peer's ordered per-command response contract;
  arbitrary unsolicited errors do not provide an operation identity on the wire.
* Nonverbose SUB is locally active after send but has **unknown server
  acceptance**. An uncorrelated late -ERR is always an observable server event,
  not evidence to remove a particular SID. Applications requiring accepted
  subscriptions must use verbose mode. No invented acknowledgment or SID is
  added to OWP.
* SIDs are positive decimal integers local to one connection, increasing even
  across rejected subscriptions. IDs are never reused; exhaustion fails closed.
  Only registered SIDs produce deliveries. Unknown/removed/pending SIDs report
  `Unknown_Subscription` and never invoke a handler.
* Explicit UNSUB sends the frame, then immediately removes the local mapping.
  Later peer rejection does not restore callbacks. Binding close removes its
  mappings locally without attempting asynchronous wire cleanup; explicit UNSUB
  or connection close is required to release peer-side subscriptions.
* Invalid protocol frames/states, unknown error codes and oversized incoming
  frames report `Protocol_Error` and close, clearing mappings. Malformed JSON
  within a correctly framed MSG is instead a `Decode_Error`: the subscription
  stays active and subsequent valid delivery remains possible.
* Handler exceptions report `Handler_Error`, separately from decode errors.
  Reentrant poll/publish/subscribe/unsubscribe/close/mapping removal/dispatch are
  rejected. No callback executes during shutdown. Transport exceptions propagate
  after closing and removing registrations; close state is set before transport
  cleanup. Finalization suppresses cleanup exceptions, explicit close does not.

`Next_Event` drains a bounded FIFO; event kinds are Initialized, Information,
Command_Accepted, Server_Error, Protocol_Error, Decode_Error, Handler_Error,
Unknown_Subscription and Connection_Closed. Diagnostic text is capped at 4 KiB.
At capacity the oldest event is discarded and `Lost_Events` increments (saturating
at Natural'Last). Applications must drain events and inspect this counter; the
runtime does not claim lossless diagnostics under unbounded peer error traffic.

## Codec versus envelope ownership

Generated codecs produce/consume a **payload body only**. The runtime wraps the
global element as exactly one original object member. OAM namespace
`https://www.vdl.afrl.af.mil/programs/oam` uses a bare global name; every other
admitted namespace uses `{namespace}local-name`, shared by SUB and envelope.
Two messages sharing a type and topic still have different envelope names.

Inbound JSON is validated by the existing bounded UTF-8/SAX validator, without
number conversion. A span scanner over known-valid JSON rejects a second root
member, including duplicate spelling and escaped-equivalent names. Only the
single key is decoded through GNATCOLL JSON for QName comparison; **no payload
number is read or written through a DOM**. The extracted original substring goes
directly to the generated codec. Internal payload duplicate semantics remain the
codec's existing policy; Task 099 does not broaden or change it.

Outbound encoding completes before envelope construction, validation and the
single transport Send. Invalid model encoding, malformed encoded JSON, invalid
identity/topic/group and oversize failure cannot publish a partial frame. The
transport contract is all-or-error whole-frame send; uncertain network writes
must close and raise, not be reported as acceptance.

## Handler ownership and lifetime

The generated API remains byte-unchanged. Its anonymous
`not null access Handler'Class` argument is converted to the binding generic's
named general access type using an ordinary **checked** access conversion before
SUB. The generic instance's master determines the minimum receiver lifetime.
Same-master/enclosing aliased objects are supported. A receiver in a nested
shorter-lived block raises `Program_Error` before publication/registration; this
is exercised through actual generated Subscribers under both compilers and all
assertion policies. Accessibility checks are language semantics, not assertions.

Bindings are limited controlled objects with a checked connection access
discriminant; connections likewise borrow a transport through a checked access
discriminant. Declare transport before connection and bindings after connection.
Bindings do not own/deallocate receivers. Applications must not manually free or
finalize registered receivers, or suppress accessibility checks. Normal scope
exit is safe: no background task exists, and binding finalization removes local
registrations without invoking receivers. Explicitly unsubscribe/close a binding
before intentionally ending receiver use, and close the connection before
transport destruction. There is no Unchecked_Access, Unchecked_Conversion, or raw
address mechanism in the runtime or its consumer.

## Resource and concurrency limits

* Whole OWP frame: 1,048,576 bytes; envelope/payload also bounded by the existing
  JSON support's 1 MiB document, depth 128 and 100,000-node caps.
* Connection registrations: 128; each typed binding: 128 local slots.
* Event FIFO: 128, with explicit loss counter; bounded diagnostic truncation.
* Identifier: 256 bytes; namespace: 2,048 bytes. Local names deliberately admit
  an ASCII NCName subset, not all XML Unicode names.
* Mock input/output: 256 frames each. Its sent-frame history is retained bounded
  test evidence; close releases queued input. No mock is a production transport.

This is a single-threaded explicitly driven runtime, not thread-safe. Poll is
nonblocking by interface contract and processes one frame; automatic progress,
timeouts, rate fairness, backpressure scheduling, real-time bounds, reconnect,
transport buffering/fragmentation and authentication policy are not implemented.
Unbounded strings and generated models allocate. Arbitrary storage-exhaustion
recovery and failure-atomic model assignment are not guaranteed (Task 097's
existing limitation remains unchanged). An injected transport must bound data
before constructing a returned oversized frame to bound transport-side memory.

## Qualification evidence ledger

Task-owned local evidence is in `/home/zboll/task099-scratch/`. Every campaign
records command/cwd, stdout, stderr, exit and completion markers. Missing final
`result.json` or any failed step is incomplete, never a pass. Pinned dependency
inputs remain Alire 2.1.1, GNATCOLL/gnatcoll_minimal 26.0.0 and GPRbuild package
26.0.1 (binary reports GPRBUILD 26.0.0). Platform is Linux x86_64 glibc 2.41.
Fresh Task 099 setups are under `float64-{14,16}/setup`; setup lock files,
provider identities, compiler binary hashes and generated source hashes are
captured rather than inferred from PATH.

Initial `matrix{14,16}-02` passed 24 cells per compiler (1,368 assertions total).
Expanded `matrix14-03` passed 24 cells with 44/53 checks by profile. These are
intermediate source results, not the final-head claim. Reusing another task's
setup directories was not durable: their disappearance interrupted the GNAT 16
expanded matrix and the next campaign. These failures are preserved in
`matrix16-03`, `matrix{14,16}-04`, and `inherited-time16`; subsequent campaigns
use fresh Task 099-owned setups. The first `matrix*-01` attempts incorrectly
applied `-gnat2022` to the third-party provider and failed on ambiguous provider
aggregates; the harness now preserves each project's language mode while
applying assertion policy/optimization to the whole build.

Preservation against an independently built archive of exact starting commit:

* `preservation-all`: 472/472 identical combinations, 349 successes and 123
  identical atomic rejection diagnostics; no newly admitted schema constructs.
* `preservation`: 120/120 identical outcomes (86 legacy successes, 19 legacy
  atomic rejections, 15 lossless successes).
* Existing generator/model/codec/support sources and dependency locks are not
  modified; all existing CI jobs/commands are retained.
* CI integrity: split guard, split selftest (218 checks), Task 077B layout and
  layout tests pass; actionlint passes. The initial layout invocation without
  its required filename failed and is preserved; the corrected command passes.

Final compiler, inherited regression and Rust counts are recorded below. Hosted
results are not implied by local completion or a scheduled job.

### Final implementation local runtime matrix

`matrix14-08` and `matrix16-08` both completed successfully with unchanged
input hashes throughout: **48 cells, 2,328 checks, zero failed steps**. Each
compiler executed OAM/qualified × portable/lossless × Normal/Check/Ignore × O0/O2.
Portable cells contain 44 checks; lossless cells contain 53, including signed
zero, minimum subnormal, maximum finite and above-halfway binary64 rounding
through the actual typed generated codec/handler path, and re-encoding/decoding.
The core envelope separately preserves exact original numeric lexemes.

Both compilers exercise pending and rejected SUB non-dispatch, valid delivery
after invalid envelope/payload, generated encoder failure before send, duplicate
root members, wrong QName, unknown/removed SID, TimeZulu rejection, handler
exception, reentrancy, nonverbose late errors, oversize/send failure, scope
cleanup, and bounded event-loss reporting. Completion markers and exact generated
sources are mandatory hosted artifacts in the new two-compiler CI job.

Locked Rust 1.95.0 workspace fmt/check/clippy and all-target tests pass:
**1,477 passed, 0 failed, 8 existing ignored**, 171 test binaries. The GNAT
availability requirement was enabled. Runtime/generator/codec/lock source
preservation against the baseline is independently checked. Every inherited
Fast CI byte is preserved apart from insertion of the new job.

Inherited TimeZulu qualification completes under both compilers: **36 cells,
5,616 documents, 1,368 accepted and 4,248 rejected**, covering portable, Float32
and lossless profiles in both namespaces under all assertion policies.
The 20 synthetic inherited campaigns per compiler (Tasks 087B–095, including
helper-demand, qualified, numeric and environment controls) all complete with
zero failures. Their task-owned `inherited{14,16}/checkpoint.json` records retain
each successful step. The optional inherited real-schema stages are run
separately with the exact-source release generator rather than waiting on
debug-mode full-schema preflight; no gate or document expectation is changed.
Those separate real-schema stages also pass on both compilers: Task 092 has
three real selections × three policies (18 executions across compilers), and
Task 093 Log has two pinned releases × three policies (12 executions). They
remain local-only evidence. The initial Log launcher passed `25`/`26` where the
existing script requires `2.5`/`2.6`; its argument failures are retained and the
corrected complete runs are under `inherited-log{14,16}`.

Task-owned compiler setup lock SHA-256 identities:

| Compiler | alire.lock |
|---|---|
| GNAT 14.2.0 | `5485669bbc0b51b00c48b258dddeb95cdac133c4630c11fc0b167cc272f1b1e7` |
| GNAT 16.1.0 | `532b439128f97b3ee28302e2393f409004608e898ceabe40b3bda6bc929926a6` |

### Local-only real-schema DLZ smoke

`local-dlz14-05` and `local-dlz16-05` both completed using the already available
pinned UCI roots, never fetching UCI or launching Sleet. The real UCI 2.5 and
2.6 DLZ closures are 53/53 lossless codec READY; portable (47/53) and Float32
(48/53) remain correctly rejected on existing Float64 boundaries. Models/APIs
match exact-main baseline generation. Across both compilers and all three
assertion policies: **12 compiled/executed cells, 312 documents, 120 accepted,
192 rejected**. Cases cover exact TimeZulu lexical preservation, Float64 kinds
and values, malformed/wrong-kind data, real Choice and constrained model checks.

Pinned root SHA-256 identities:

* UCI 2.5: `ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27`.
* UCI 2.6: `af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b`.

This evidence is **local-only generated compiler/codec smoke**, not the runtime
transport qualification and not a Sleet interoperability assertion.

The exact-main baseline archive was independently built, not substituted by an
unidentified developer binary. Additional local real-schema probes compare to
that baseline, not to the historical Task 097 parent. Earlier attempts referring
to removed Task 098 scratch inputs and superseded debug full-schema campaigns
remain retained, incomplete evidence. The final runtime harness freezes the
consumer/schema once and verifies all implementation input hashes before writing
success, preventing mixed-source campaigns. Container source recording uses
exact-directory Git trust even inside the private provisioning HOME.

### Full inherited Float64 production qualification

`float64-14/result.json` and `float64-16/result.json` both complete with zero
failed steps (17/16 respectively; the shared C boundary runs once under 14).
These are the unchanged Task 097 production harnesses with freshly provisioned
Task 099-owned compilers/providers. They include frozen oracle/independent review,
C O0/O2 boundary, checkpoint, both-namespace composition, generated numerical
corpora, independent floating-environment controls, conformance and optimized
OAM generated/conformance repeats. Each generated corpus cell checks 27,167
conversions and 20,474 encodings across 15 finite positions under all three
assertion policies. Existing qualified-platform/locale and invalid-data gates
remain unchanged.

Local debug generator SHA-256:
`4d7bf955ab3612e42ee06cadcbc5e712818dc30ebf4a386a7c4389181bfe3842`.
Local release generator SHA-256:
`c21544e8c400cac883c3fd9d0b90660814d126110d6644cc443649f43e268e91`.
Both are built from this branch's unchanged generator sources. Runtime input
hashes and generated/compiled source hashes are recorded separately in both
`matrix*-08/input-sources.json` and `sources.json`. Final commit/tree identity
is recorded by normal Git commit and PR/CI source artifacts, avoiding a
self-referential commit hash in its own content.

## Follow-on WebSocket/Sleet qualification

Implement a separate whole-text-frame WebSocket transport, including `owp`
subprotocol negotiation, UTF-8 validation, fragmentation/message limits,
ping/pong and close handshakes, send-failure semantics, TLS certificate and
authentication policy. Qualify that exact transport/runtime/generated consumer
against unmodified pinned Sleet with actual frame/acceptance/error/close
evidence. Mock success and local real-schema codec smoke tests are **not**
WebSocket or Sleet interoperability evidence.