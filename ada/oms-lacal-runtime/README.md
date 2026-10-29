# Ada LA-CAL runtime

## Optional WebSocket transport (Task 100)

The core project remains transport-independent. Import the separate
`oms_lacal_websocket.gpr` companion to use
`OMS_LACAL.Transports.WebSocket.Client`. The client implements the existing
transport interface; call `Open` before `Connections.Initialize` and drive
receive explicitly using the existing connection's `Poll`.

Set `OMS_WS_AHT_SOURCE` to the clean AHT checkout at
`664d5ddeb568626b2f9ff66054c7091894b3b8d4` and `OMS_WS_CODEC_SOURCE` to the clean
websocket-ada checkout at `765d9cd612838eb00645037977c8cc9af7171c3e`. The
qualification runner provisions these pins itself. Only standalone framing,
UTF-8, randomness, bytes and handshake units are compiled, not either library's
high-level client. GNATCOLL 26 remains the unchanged support dependency.

This adapter supports explicit IPv4 loopback `ws://` URLs only. `wss://` and
remote hosts are rejected. It is not secure for remote deployments. Poll is
nonblocking/bounded-work, never calls application handlers, and has no
background task. See the complete contract, limits, compiler/Sleet evidence,
license identities and unsupported features in
`docs/task-100-ada-websocket-transport.md` at the repository root.

## Transport-independent core (Task 099)

Transport-independent OWP 1.0 core. The core project has no production
WebSocket/TLS dependency or automatic receive task; the optional transport
and its separately scoped Sleet qualification are described above.

Import `oms_lacal_runtime.gpr`. The existing sibling JSON support project uses
GNATCOLL 26.0.0; the lossless generated codec additionally imports
`../oms-json-support/ams_gra_oms_json_lossless.gpr` on its qualified platform.
No generated model or codec is copied into this library.

* `OMS_LACAL`: limits, errors, events and shared global-element QName formatter.
* `OMS_LACAL.Transports`: whole-frame send, nonblocking poll, close interface.
* `OMS_LACAL.Connections`: INIT/INFO, PUB/SUB/UNSUB, ACK/error processing,
  monotonic SIDs, bounded registration/event state, explicit poll and cleanup.
* `OMS_LACAL.Envelopes`: exactly-one-original-member OMS envelope, untouched
  payload substring extraction; no numeric DOM round trip.
* `OMS_LACAL.Typed`: generic binding to generated payload codecs and endpoint
  Handler interfaces, checked persistent receiver access, guarded dispatch.
* `OMS_LACAL.Transports.Mock`: deterministic bounded in-process test peer.

For a complete application calling actual generated `Service_API` Publishers
and Subscribers, see `../../tests/ada-lacal-runtime/runtime_consumer.adb` and
`../../scripts/task099/qualify.py`. The script generates and executes both
namespaces/profiles; it does not substitute fake codecs or facade declarations.

Declare transport, then aliased connection, then endpoint-specific generic
instances and bindings. Declare aliased receiver objects in the generic
instance's master or an enclosing master. The adapter hook passes the generated
receiver parameter into the binding's `Subscribe`; a checked conversion rejects
shorter-lived receivers **before SUB is sent**. Do not free a registered receiver
with application-owned deallocation. Explicitly unsubscribe/close its binding
before deliberately finalizing or destroying a receiver.

The application explicitly calls connection `Poll`, then offers the delivery
to its endpoint bindings' `Dispatch`. Unknown SIDs are not delivered. A binding
selects by its registered SID and verifies its stored QName before decoding.
No callback access is stored by the connection. The connection synchronously
borrows the binding's processing callback during dispatch and guards reentrancy.

Default verbose mode permits only one outstanding command. Poll `+OK` before
the next command; INIT also requires INFO. Publish success means **sent**;
acceptance is a later event. Pending subscriptions cannot dispatch until +OK.
Nonverbose mode does not establish acceptance; late -ERR carries no correlating
SID and cannot identify a rejected subscription. The runtime reports it without
inventing correlation or removing arbitrary registrations.

Close is deterministic and connection close is idempotent. Binding close removes
local mappings without wire traffic; use explicit unsubscribe for peer cleanup
while retaining a connection. Scope finalization closes a connection and clears
bindings. No background work or callbacks run after these operations.

Typed `Unsubscribe` accepts only a SID registered by that binding, even when
multiple generated Handler bindings share a connection. Unknown, foreign, zero
and already removed SIDs raise `Runtime_Error` without sending an OWP frame or
changing any subscription state. The local handler mapping is cleared only
after the connection's unsubscribe succeeds.

See `../../docs/task-099-ada-lacal-runtime-core.md` for protocol, ownership,
resource, compiler and qualification limitations.