# Task 107 — reusable heterogeneous Ada LA-CAL dispatch

## Production API

The transport-independent runtime adds `OMS_LACAL.Dispatchers` in
`ada/oms-lacal-runtime/src/`. It depends only on the existing connection runtime:

```ada
type Participant is limited interface;
function Is_For
  (Self : Participant; Core : not null access Connections.Connection)
   return Boolean is abstract;
procedure Dispatch
  (Self : in out Participant; Message : Connections.Delivery) is abstract;
type Reference (Value : access Participant'Class := null) is limited null record;
type Binding_List is array (Positive range <>) of Reference;
procedure Poll_And_Dispatch
  (Core : not null access Connections.Connection; Bindings : Binding_List);
```

`Typed.Binding` remains a limited controlled type and additionally implements
`Participant`. Its `Is_For` compares its existing connection discriminant with
the borrowed connection parameter. A predicate avoids exporting a connection
access discriminant through an access-valued function (Ada accessibility rules).
Its original public `Dispatch`, `Subscribe`, `Unsubscribe`, `Close` and `Publish`
calls remain source-compatible. `Connections.Poll` is unchanged.

The operation validates **all** entries before its single poll: null entries,
duplicate object identities and bindings from another connection raise
`OMS_LACAL.Runtime_Error`. It then offers the one delivery to each participant.
The original typed implementation remains the sole authority for SID ownership,
active status, expected global-element QName, generated codec and typed callback.
There is no new SID registry, connection, decoder or handler access storage.
`Decode_Error` and `Handler_Error` retain their original reporting semantics;
unexpected exceptions are not caught by the dispatcher.

## Three distinct generated types

The production generator consumes `tests/ada-lacal-dispatch/messages.xsd` and
`service.yaml`, emitting three different concrete records and their OMS JSON
codecs: `Programs.Oam.LogPayload` (string), `RdmaPayload` (integer), and
`RiskPayload` (Boolean). These are compact synthetic types, not claims of real
UCI message qualification. The complete consumer instantiates:

```ada
package Log_Binding is new OMS_LACAL.Typed
  (Programs.Oam.LogPayload, IL.Handler, IL.Handle,
   Service_Codec.Encode_LogPayload, Service_Codec.Decode_LogPayload, Is_Codec);
package Rdma_Binding is new OMS_LACAL.Typed
  (Programs.Oam.RdmaPayload, IR.Handler, IR.Handle,
   Service_Codec.Encode_RdmaPayload, Service_Codec.Decode_RdmaPayload, Is_Codec);
package Risk_Binding is new OMS_LACAL.Typed
  (Programs.Oam.RiskPayload, ISK.Handler, ISK.Handle,
   Service_Codec.Encode_RiskPayload, Service_Codec.Decode_RiskPayload, Is_Codec);
LB : aliased Log_Binding.Binding (Core'Access);
RB : aliased Rdma_Binding.Binding (Core'Access);
SB : aliased Risk_Binding.Binding (Core'Access);
--  Subscribe using each generated endpoint's Subscriber, then poll ACKs.
--  Application receive step:
OMS_LACAL.Dispatchers.Poll_And_Dispatch
  (Core'Access,
   ((Value => LB'Access), (Value => RB'Access), (Value => SB'Access)));
```

Here `IL`, `IR`, and `ISK` rename the generated input endpoint packages;
`Is_Codec` recognizes only `Service_Codec.Codec_Error`. All declarations,
receiver implementations and generated facade hooks appear in the runnable
`tests/ada-lacal-dispatch/dispatch_consumer.adb`.

## Lifetime and limitations

Anonymous access discriminants in limited reference elements allow borrowing
bindings at the caller's scope while preserving checked Ada accessibility.
Neither the collection nor any element escapes the call. Transport, connection,
typed generic instances, bindings and receivers must have the existing compatible
lifetimes. Registered receivers are still retained by their typed binding until
unsubscribe/close/finalization, never by dispatchers. Explicitly close/unsubscribe
before destroying a receiver. Do not deallocate participating objects or finalize
them within a callback. Core and binding reentrancy guards remain in force.

No background work, threading, synchronization, scheduler, reconnect loop, replay
or delivery queue is added. Use from one thread/task. Validation is O(n²), dispatch
is O(n) plus existing bounded per-binding lookup and codec costs. The collection
has no fixed size; empty lists still poll once. Missing owners/empty lists discard
that poll's delivery. Each binding can still own up to the existing runtime limit;
the connection's unchanged capacity remains authoritative. A binding associated
with the correct but closed connection is legal; the existing poll produces no
callback. Custom implementations of the open interface are trusted application
code and must report connection identity truthfully and not retain borrowed data.

## Focused validation

Use the existing qualified Task096 setup, without changing its pins:

```sh
python3 tests/ada-lacal-dispatch/run.py \
  --scratch /tmp/task107-16 --setup /tmp/task099-evidence/setup \
  --compiler 16 --generator "$PWD/target/debug/ams-gra-codegen-oms"
```

Repeat with a GNAT14 setup and `--compiler 14`. The runner copies runtime/provider
sources to isolated scratch directories, generates actual code, and compiles/runs
O0/O2 × Normal/Check/Ignore for each compiler. Each execution requires exactly
64 checks and the completion marker. It also requires two compiler failures
specifically diagnosing an input's nonexistent `Publisher` and an output's
nonexistent `Subscriber`; unrelated compiler failures do not count as success.
Stdout, stderr, commands, exits, generated sources and a compact result are kept
in scratch. No external broker or network is used by the consumer.

Checks cover one initialized core and three active subscriptions, independent
hand-authored JSON and generated publication, interleaved ordered single-frame
polls, codec and QName failures/recovery, handler exception isolation, nested
dispatcher and binding reentrancy rejection, inactive/unknown/unsubscribed SID
silence, unauthorized unsubscribe rejection, duplicate/foreign/null list rejection
without consuming queued frames, non-one-based and subset lists, empty lists,
checked receiver lifetimes, shorter-lived local bindings, explicit close and
binding/connection finalization, and unexpected exception propagation.

Compatibility validation uses the unchanged Task099 runner and Task106 verifier
unit tests. The new consumer is a step in the existing `ada-lacal-runtime-core`
main job's GNAT14/16 matrix; artifacts use that job's existing upload. No job,
workflow trigger, ten-lane assignment, PR smoke selector, Deep registration,
manifest, source-bound evidence verifier or frozen consumer changes. Ordinary
Task107 paths are already Ada-owned; `ci.yml` additionally selects syntax checks.
The generator, shared IR, other backends, Sleet and external pins are untouched.

Local results and exact final HEAD/tree, incorporated main, PR URL and exact-head
hosted PR smoke status are reported in the PR/delivery record. Full pre-merge Deep
qualification is deliberately not launched; it remains main-only. Readiness
requires both focused compiler runs, compatibility checks and exact-head PR smoke.