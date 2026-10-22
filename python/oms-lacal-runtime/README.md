# Python OMS LA-CAL runtime

Python **3.11+**, qualified with **websockets==17.2**, using
`websockets.asyncio.client.connect` (not the legacy API). Install this separate
package; generated models, service APIs and codecs have no WebSocket dependency:

```sh
python3.11 -m venv /absolute/path/to/venv
/absolute/path/to/venv/bin/python -m pip install /absolute/path/to/repository/python/oms-lacal-runtime
```

Generate a supported service with `service-generate --language python --with-codec`
under an explicit generation world. Explicitly bind each **message QName**, not
its topic or the payload class name, to the actual generated functions:

```python
import asyncio
from oms_lacal import CodecBinding, CodecRegistry, LacalRuntime, RuntimeConfig
from generated import model, service_api, service_codec

OAM = 'https://www.vdl.afrl.af.mil/programs/oam'

async def main():
    registry = CodecRegistry([
        CodecBinding(OAM, 'RuntimeNotice', model.Payload,
                     service_codec.encode_Payload, service_codec.decode_Payload,
                     service_codec.CodecError),
    ])
    config = RuntimeConfig('ws://127.0.0.1:9000',
                           service_id='python-runtime', schema_version='000.1.0')
    received = asyncio.Queue(maxsize=4)
    async with LacalRuntime(config, registry) as runtime:
        # Generated adapter operations return awaitables. Explicitly await them.
        async with await service_api.function_loop.exchange_input.subscribe(
                runtime, received.put_nowait):
            value = model.Payload(count=model.Count(42), flag=True, text='Python λ')
            await service_api.function_loop.exchange_output.publish(runtime, value)
            delivered = await asyncio.wait_for(received.get(), timeout=5)
            print(delivered.count.value)

asyncio.run(main())
```

The example matches `tests/fixtures/python-runtime`; topic/service authorization
must be supplied in Sleet deployment configuration. Never infer deployment IDs
from a contract's human service name. Multiple QNames may use one model type and
codec; duplicate identical registrations are idempotent and conflicting ones fail.
Registry entries and the registry mapping are immutable; registrations are
explicit trusted Python callables, not sandboxed or dynamically discovered.

## Lifecycle and failures

- Runtime is single-use; `async with` connects and deterministically closes.
  `await close()` is idempotent, joins receive/dispatch tasks, and invalidates all
  handles. `await handle.unsubscribe()` sends UNSUB and invalidates that handle.
- Verbose **OWP 1.0 only**: INIT requires `+OK` then valid INFO. Non-verbose mode
  is explicitly rejected. Runtime verifies negotiated `owp`, INFO version and
  UUID metadata. Later INFO must preserve initialized UUID identity.
- One receive owner per active connection, one serialized status-bearing
  operation at a time. INFO and MSG may interleave while waiting for `+OK`/`-ERR`.
  A pending-operation timeout/cancellation makes acknowledgement association
  ambiguous, so the connection is terminated, never reused/replayed. Cancellation
  while merely waiting for the operation lock does not terminate another request.
- PUB/SUB `-ERR` becomes `ServerRejection(code)` without arbitrary server details.
  The connection stays usable where the server permits it. UNSUB invalidates the
  local handle immediately; rejected/cancelled UNSUB closes the connection so
  there is no remotely live, locally forgotten ghost subscription.
- Per-subscription dispatch workers isolate handlers and preserve SID order.
  A decoded/handler error reports `RuntimeEvent(error, subscription_id)` through
  `await runtime.next_event()`. No other SID's handler is called. Async handlers
  are bounded by `handler_timeout`; **synchronous handlers must not block**.
- Exceptions distinguish `InvalidInput`, `NotConnected`, `ProtocolError`,
  `ServerRejection`, `TransportFailure`, `OperationTimeout`, `CodecFailure`,
  `CodecInternalFailure`, `UnknownSubscription`, `HandlerFailure`, `ResourceLimit`.
  Internal decoder failures are not misreported as malformed payloads. Synchronous
  encoder programming errors propagate; only the registered generated codec
  exception type is translated to `CodecFailure`.
- Fatal receive errors set `terminal_error`, invalidate handles and abort the
  transport. `close()` still joins tasks and releases resources. Events do not
  include raw payloads, credentials, or arbitrary remote/handler error text.

## Strict envelope / lossless codec boundary

Runtime owns the one-member global-element envelope. OAM names are bare;
non-OAM names use `{namespace}local`, qualified by mock tests only. It verifies
the exact expected QName for the SID.

`JSONDecoder.raw_decode` validates the **original** body slice with duplicate
decoded-name rejection at every depth and lexeme-preserving numeric hooks.
After exactly-one-member/trailing-content checks, it passes that original
substring to the generated strict decoder. It does **not** reserialize a
collapsed dict. Thus escaped duplicates, float/exponent lexemes, null, unknown
fields and `$type` cannot be laundered through an envelope parse. This is a
two-pass accepted-input strategy, not an independent schema interpretation.

## Resource bounds and security

Config defaults: connect/operation/async-handler timeout 10 s; close timeout 2 s;
1 MiB max incoming **and outgoing** message; WebSocket frame queue 16; pending
message queue 16 **per subscription**; event queue 64; max subscriptions 64.
At most 64 callers may be inside/waiting for an OWP operation; additional
operations fail admission with ResourceLimit before sending.
All bounds must be positive finite numbers/integers. `websockets` supplies bounded
frame buffering; no unbounded runtime message/event queue is created. Queue
exhaustion is `ResourceLimit`, fails the connection rather than silently dropping
application messages. Event overflow replaces the oldest event with an explicit
ResourceLimit and sets terminal state. Parsing recursion/memory failures are
resource errors, not malformed payload claims.

OWP identifiers use nonempty ASCII `[A-Za-z0-9_.-]+`. Invalid service/topic/group
and invalid/unknown registrations fail before network operations. SIDs are
connection-owned monotonically increasing identifiers; no caller-selected SID.
QName local names use the currently qualified ASCII schema-name subset. There
are no environment-proxy connections (`proxy=None`) or embedded URL credentials.
WSS uses default certificate and hostname verification. A caller-supplied trusted
CA context is allowed **only** with CERT_REQUIRED and check_hostname enabled.

No automatic retry, reconnect, resubscribe, replay, offline queue, unknown-type
retention, dynamic schema discovery, authentication configuration, or real UCI
PositionReport support. The Python model/codec subset remains PY-004. Non-OAM
mock success does not establish pinned-Sleet Clark-QName compatibility.

## Qualification

Build the production CLI first, then install the package with the pinned
dependency in the chosen Python environment:

```sh
cargo build --locked -p ams-gra-codegen-oms
python -m unittest discover -s python/oms-lacal-runtime/tests -p test_mock.py -v
python -O -m unittest discover -s python/oms-lacal-runtime/tests -p test_mock.py -v
```

`test_sleet.py` is a separate **must-execute** campaign requiring
`AMS_GRA_SLEET_BIN`; missing broker is an error, not a skip. The existing Deep CI
Sleet harness builds the pinned broker once and invokes the focused Python
campaign against that same binary. All payloads use actual generated PY-004
models and codecs. See `docs/task-py-005-python-lacal-runtime.md` for exact evidence.