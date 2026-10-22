# PY-005 — Python LA-CAL runtime over OWP/WebSocket

## Provenance and authority

Fetched main and PR #98–101; #101 was rechecked immediately before branching.
All remain open/mergeable, no recorded review approval. Earlier branches are
unmodified; no merges, force-pushes or auto-merge changes.

| Input | Exact source SHA |
| --- | --- |
| `origin/main` | `5f6800d505c6501638c322d20b23cfc36b09d7d6` |
| PR #98 | `93d02dba9bdc398453a29b5dd3b9956ae26793fb` |
| PR #99 | `05d2ac17ec352d6b3b6636d86050245a16dcc7f7` |
| PR #100 | `697617f889a147661142cf244467c2ed6232c928` |
| PR #101 / branch base | `3e849067f7298f1627aab83dc9557912b956a317` |
| pinned Sleet | `e38f61d8ce0d75c8508434a52f2ed77c69cf6a3b` |
| OMS standard | `726272bd0390982a759c91a9cf4e13b81c2b510b` |

Reviewed Tasks 049, 050, 070, 071, 073, 077 and PY-001–004. The official
OMSC-SPC-013 Rev B DOCX at
`docs_official/20_OMSC-SPC-013_RevB_LanguageAgnostic_CAL_Specification_DandD_v2_5.docx`
was hash-verified again and its `word/document.xml` text read directly:
SHA256 `b1c3c07872570fb4f2c84fd819076b1b588148ef28225f5a8e23433efe94b1a7`.

Normative §5.1: UTF-8 text WebSocket messages, negotiated subprotocol `owp`, INIT
with explicit versions/schema/service_id/verbose, first verbose +OK before INFO,
one operation per message, space/tab delimiters, client-owned subscription IDs,
PUB/SUB/UNSUB/MSG and status errors. §6.1: exactly one global-element member,
bare OAM or Clark-qualified non-OAM identity, typed schema validation. Unknown
`$type` is not authority (Task 073); duplicates must not be collapsed before
strict codec ingress (Task 077).

Pinned implementation evidence: Sleet's client `connect_with_options`, typed
INFO/OWP parser, server `handle_awaiting_init`, `process_pub` and `process_sub`.
Explicit topic bindings authorize both publish and subscribe, not contract
direction-specific policy. Missing bindings are permissive; the test config
binds every allowed topic. Unauthorized topics/messages return Illegal-State
and permit subsequent valid traffic. Sleet's non-OAM lookup limitation is not
an OMS prohibition; non-OAM transport is qualified with the mock only.

## Package and dependency boundary

`python/oms-lacal-runtime/pyproject.toml` defines the separately installable
`ams-gra-oms-lacal-runtime` package, Python >=3.11, **websockets==17.2**. It uses
`websockets.asyncio.client.connect`, not legacy APIs. The runtime dependency
never enters generated model/API/codec imports. No generator or model backend
code is changed, no extra contract resolution/schema inference is introduced.

See [package README](../python/oms-lacal-runtime/README.md) for installation,
complete application example, public types, defaults and lifecycle semantics.

`RuntimeConfig`, immutable `CodecBinding`/`CodecRegistry`, `LacalRuntime`,
`SubscriptionHandle`, classified `LacalError` subclasses and `RuntimeEvent`
form the reusable API. Explicit QName registrations carry actual generated
payload class, encode/decode callables and the generated exception class.
Several message QNames may share one payload class. Conflicts fail before I/O.
Runtime adapter coroutine methods have the exact keyword parameters expected
by the unchanged generated facade; applications explicitly await endpoint calls.

## Protocol ownership and lifecycle

One-use runtime, explicit connect or async context. Verbose OWP 1.0 only;
non-verbose is rejected. Handshake owns reads until validated INIT +OK/INFO;
then exactly one receive task owns socket reads. Serialized operations have
one pending acknowledgement future. MSG and INFO can interleave with status.
INFO UUID identity is stable. Binary/unsupported/malformed frames fail closed.

PUB checks topic, registry and exact payload type; actual generated encoder
validates the value, then runtime adds the one-member global-element envelope.
SUB allocates a unique connection-local SID, registers a bounded dispatch queue,
and forwards exact topic/QName/group. Workers dispatch only by SID, verify QName,
decode through actual generated codec and call sync/async handlers. Duplicate
message type subscriptions remain separate. Bad decode or handler failure emits
an event and does not terminate unrelated subscriptions.

PUB/SUB server rejection is accurately raised without poisoning a connection
that the server keeps open. Operation timeout or cancellation after starting
closes the connection because an untagged acknowledgement is ambiguous. No
replay/reconnect/resubscribe. Lock-wait cancellation has no protocol side effect.
UNSUB immediately invalidates its handle; failure closes rather than retaining
a remotely live ghost subscription. Connection closure invalidates every handle.
Close is idempotent/cancellation-safe and joins receive and dispatch tasks,
including cancellation during connect, pending operations and handler dispatch.

## Strict envelope boundary

`wire.unwrap` uses stdlib JSON raw_decode for duplicate-aware syntax validation
while recording the original body substring offsets. Its parse_int/parse_float
hooks preserve lexemes as strings for the discarded temporary tree. The original
substring goes directly to the strict generated PY-004 decoder. There is no
ordinary json.loads → collapsed dict → reserialization path. Exactly one outer
member, expected QName and complete trailing consumption are required; duplicate
escape aliases, nested duplicate unknown subtrees, numeric exponent forms and
unsupported `$type` remain rejected. Accepted huge integers never pass through
float or the default int-string digit conversion. Tests deliver >5000-digit ints.

## Resource/security policy

All timeouts and queue/subscription/message/pending-operation bounds are explicit,
validated RuntimeConfig fields. Pending OWP operation admission is capped, not an
unbounded lock wait queue. Queues are bounded per subscription and for events;
exhaustion is ResourceLimit and fails the connection rather than silently dropping
messages. Event overflow retains an explicit resource error and terminal state.
WebSocket message size is enforced on ingress by websockets and egress before
send. Parser recursion/memory and async-handler timeout are resource failures,
not malformed-payload evidence. Synchronous handlers/codecs must not block the
event loop; no claim of preemptive CPU isolation or general DoS sandbox is made.

Identifiers are nonempty ASCII `[A-Za-z0-9_.-]+`. No service ID inference from
contract display names. WSS verifies certificate and hostname by default; trusted
CA contexts must retain CERT_REQUIRED/check_hostname, rechecked at connect.
Actual TLS controls cover untrusted peer, trusted peer and hostname mismatch.
No payload/server details or credentials are interpolated into ordinary runtime
error messages; registered internal codec and handler failures are separately
classified. RuntimeEvent is explicit; applications should consume events and
inspect `terminal_error`, not assume every sent PUB succeeds.

## Qualification and limitations

Fast: 19 real-network unittest cases against a local asyncio WebSocket peer,
Python 3.11/3.13, normal and `-O`, warnings treated as errors. Actual generated
models/API/codecs; successful INIT/INFO/PUB/SUB/MSG/UNSUB, SID/group/QName routing,
non-OAM Clark transport, interleaving, escaped/nested duplicate rejection,
numeric lexemes, subsequent valid traffic, unknown SID, handler/internal-codec
isolation, timeout/cancellation/close, bounds, TLS and no leaked runtime tasks.
The shell gate requires the exact test count and OK, not a zero-match success.

Deep: `check-py005-sleet.sh` runs the focused must-execute OAM test normally and
under `-O`. `run-real-sleet-test.sh` reuses its single exact-revision unmodified
broker build, then runs the pre-existing Rust campaigns unchanged. Test uses
two Python runtime connections and `tests/fixtures/python-runtime/oam.xsd`:
generated payload → generated codec → runtime envelope/PUB → actual pinned
Sleet → MSG → strict envelope → generated decoder → typed handler. Both broker
debug receipt marker and application handler are required. Topic/message PUB
and SUB policy rejection is tested, followed by valid delivery on the connection.
No handcrafted payload codec is used; missing broker is failure, not skip.

No schema/model subset expansion, non-verbose mode, auto retry/reconnection,
resubscription, replay, generic private-namespace Sleet interoperability, real
UCI PositionReport or deployment authentication support. TLS mock qualification
does not claim a TLS-configured Sleet deployment. Local pinned Sleet synthetic
qualification is separate from hosted CI and externally pinned real-UCI tests.

Exact candidate source/tree SHAs, package/dependency versions, generated-file
parity digest, local totals and hosted statuses are recorded in the delivery PR.

Locally verified Sleet tree: `7e705f4eb5f658957bd414b4abc8207311f3c1e4`;
release executable SHA256:
`4a2e9ee872d1deaa1a6ce1a1494a2f601660eb57c0df28a447b28a08eba21c94`.
The task-owned checkout was clean before build and remained clean afterward.
The full shared Sleet harness passed, including unchanged legacy Rust synthetic
and duplicate-ingress campaigns. Real-UCI campaigns were not claimed locally
because no verified UCI root was supplied to that harness invocation.