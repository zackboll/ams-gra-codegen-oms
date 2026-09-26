# Task 048 — generated typed publish/subscribe façade

Task 048 implements the Phase 2.5 roadmap item **generated service
publish/subscribe façade**. For every OMS Message exchange of a READY Service
Contract, `service-generate` now emits exactly one typed, direction-safe
operation beside the Task 047 descriptors:

```text
output OMS exchange -> Publish,   taking exactly that endpoint's Payload
input  OMS exchange -> Subscribe, registering a handler for exactly that Payload
```

The application never supplies a topic, a UCI message name, a namespace, or a
subscription group: the generated operation supplies them from the endpoint.
Each operation forwards to an **injected runtime adapter** that the
application provides. No runtime implementation exists in this task.

> Task 048 performs no communication. There is no WebSocket, Sleet, OWP
> (`INIT`/`PUB`/`SUB`/`UNSUB`/`MSG`), subscription-ID generation,
> reconnection, TLS, JSON/XML codec, receive loop, queue, thread, task, or
> CAL server discovery, and no new dependency.

```text
Task 047:  What endpoints exist?
Task 048:  Which typed operation may the application perform at each OMS endpoint?
Task 049:  How are those operations executed through LA-CAL/Sleet?
```

## 1. Evidence gate: direction semantics

The mapping from contract direction to application operation was fixed from
authoritative material **before** any production change. The typed
`Direction` field is authoritative; exchange IDs such as `*-input` are never
consulted (the lowering test `operation_is_decided_by_direction_only` uses IDs
that deliberately contradict their directions).

### Pinned sources

| Source | Revision | Artifact | sha256 |
| --- | --- | --- | --- |
| OMS standard (`open-arsenal/oms`) | `726272bd0390982a759c91a9cf4e13b81c2b510b` | `docs_official/14_2_OMSC-INS-003_RevM_ServiceContractInstructions_DandD_v2_5.docx` | `a995eff6...9077` |
| same | same | unofficial Markdown conversion of the above | `5c57894b...f2baf` |
| same | same | `docs_official/20_OMSC-SPC-013_RevB_LanguageAgnostic_CAL_Specification_DandD_v2_5.docx` | `b1c3c078...b1a7` |
| same | same | unofficial Markdown conversion of the above | `5798da43...e98f` |
| portable contract (`zackboll/ams-gra-service-contract`) | `4ea5be8dd36e9695bd58f2c36c6b3dd8075de249` | `examples/service-status.yaml` | `4611b098...53d5` |
| same | same | `examples/service-initialization.yaml` | `867bf155...6140` |
| same | same | `docs/specification.md` | `78fc434c...8dfc` |

Quotations below were read from the unofficial Markdown conversions and
checked against the table rows they reproduce; the `.docx` artifacts remain
the official sources.

### Service Contract directions are service-relative

OMSC-INS-003 Rev M, Section 3 column definitions:

> Input / Output (I/O) – this column indicates whether the data exchange is an
> input (I) or an output (O).

and, for OMS Messages, the Data Exchange Information column is

> the specific topic name on which this message is published. [...] If
> applicable, the subscription group follows in brackets: Topic Name
> \[Subscription Group\]

The function descriptions state which side acts. Service Status (§3.1.2):

> The periodic **ServiceStatus** message is published at a configurable
> interval time [...] this Service will reply to a received
> **ServiceStatusDataRequest** message with a **ServiceStatusDataRequestStatus**
> message

Position Information Processing: "subscribe to periodic **PositionReport** or
**PositionReportDetailed** messages"; Capability Enable/Disable: "subscribes to
**\[CapabilityName\]SettingsCommand** messages and publishes
**\[CapabilityName\]SettingsCommandStatus** response messages".

Table 3.1-2, Service Status Inputs and Outputs (MP, I/O, DE, name):

| MP | I/O | DE | Message |
| --- | --- | --- | --- |
| D | **O** | M | ServiceStatus (periodic, 1 Hz / 0.5 Hz) |
| DR | **I** | M | ServiceStatusDataRequest (aperiodic) |
| DRS | **O** | M | ServiceStatusDataRequestStatus (on demand, 0.5 s / 3 s) |

Table 3.1-1, Service Initialization: **FileMetadata** `I`, **FileLocation**
`I` (both optional, aperiodic). Table 3.3-1, Position Information Processing:
**PositionReport** `I`.

The portable contract mirrors these rows exactly
(`examples/service-status.yaml`: `ServiceStatus` output,
`ServiceStatusDataRequest` input, `ServiceStatusDataRequestStatus` output;
`examples/service-initialization.yaml`: `FileMetadata` input, `FileLocation`
input), and its specification (§10.1) says one exchange has exactly one
direction, so a message used both ways is two exchanges.

**Conclusion.** An `output` is a message the Service publishes; an `input` is
one it receives, i.e. subscribes to. Nothing contradicts the mapping:

```text
Direction::Output -> Publish
Direction::Input  -> Subscribe
```

### LA-CAL Rev B protocol evidence (for Task 049, not implemented here)

OMSC-SPC-013 Rev B, §5.1.1.1 (OMS WebSocket Protocol), "Publish and
Subscribe":

> To publish a message, a CAL Client provides a topic and CAL Message [...].
> To subscribe to a topic and begin receiving messages, a CAL Client provides a
> unique subscription identifier, the message name, the topic, and an optional
> subscription group name [...]. After subscribing to a topic, a CAL Client
> receives incoming messages with the **MSG** protocol operation [...]. To stop
> receiving messages from a subscription, a CAL Client uses the **UNSUB**
> protocol operation with the associated subscription identifier

Operation syntax (CERT LACAL-000008/9/10/12):

```text
PUB <topic> <message>
SUB <subscription id> <message name> <topic> [group]
UNSUB <subscription id>
MSG <subscription id> <message>
```

* **subscription id** — "generated by and local to the CAL Client", so it is
  runtime-owned; UNSUB and MSG refer to it.
* **message name** — "The {target namespace} and {name} of a *global*
  {element}, specified as "{name}" if {target namespace} is
  *https://www.vdl.afrl.af.mil/programs/oam*; otherwise
  "{{target namespace}}{name}"". SUB therefore needs the resolved global
  message identity, not a payload type name.
* **group** — "The optional subscription group name".
* **message** (PUB, MSG) — "A CAL Message that conforms to a format in Table
  5.1-2", currently only OMS JSON (§6.1).

Task 048 implements none of these operations.

## 2. The runtime boundary this implies

Publish and subscribe need different metadata, so they are different
operations with different adapter hooks:

| | Generated façade supplies | Application supplies | Future runtime owns |
| --- | --- | --- | --- |
| output (Publish) | topic | typed Payload | JSON encoding, PUB framing, connection, errors |
| input (Subscribe) | message namespace + local name, topic, optional group | typed handler | subscription ID, QName→OWP spelling, SUB/UNSUB/MSG framing, JSON decoding, dispatch, errors |

The resolved message identity stays **structured** end to end: the model
carries a `QualifiedName`, and every façade hands the adapter the namespace
URI and the local name separately. Task 048 never produces `PositionReport`
or `{namespace}PositionReport`; the LA-CAL spelling rule is Task 049's single
runtime concern.

## 3. `ServiceApiModel` extension (codegen-core)

```rust
pub enum ServiceApiOmsOperation { Publish, Subscribe }

impl ServiceApiOmsOperation {
    pub const fn for_direction(direction: Direction) -> Self {
        match direction {
            Direction::Output => Self::Publish,
            Direction::Input => Self::Subscribe,
        }
    }
}
```

`ServiceApiOmsBinding` gains `operation()` (decided once, at lowering) and
`subscription_group() -> Option<&str>` (the authored portable value, verbatim;
`None` when omitted). Topic, resolved message `QualifiedName`, and payload
`TypeRef`/name are preserved unchanged. Backends read `operation()` and never
look at `Direction`, mandate, timing, or IDs to decide what to emit.

* **Mandate does not change availability.** Mandatory and optional inputs are
  both Subscribe; mandatory and optional outputs are both Publish. No
  `Option` return, no suppressed façade.
* **Timing schedules nothing.** `periodic`, `asynchronous`, and `on_demand`
  remain metadata. No timer, sleep, deadline, task, or request/response
  pairing is generated; a periodic output is an ordinary Publish the
  application calls when it chooses.
* **Subscription group** is carried for every OMS exchange (it is contract
  data) but handed to the adapter only by Subscribe. It is never inferred,
  defaulted, derived from an ID or mandate, or checked against the LA-CAL
  lexical grammar — the portable contract only requires it non-empty, and the
  future adapter owns OWP syntax.
* The four non-OMS kinds carry no binding, hence no operation. A Data
  Transfer `direction: input` is **not** a CAL subscription.
* Operational attributes and Appendix C mappings are still not exposed; the
  façade does not need them.

## 4. Prototypes before generators

Hand-written throwaway prototypes of the generated shape (not committed) were
compiled with fake adapters in all three languages **before** any renderer
changed, under `rustc -D warnings`; `c++ -std=c++17 -Wall -Wextra -Werror
-pedantic-errors` (GCC 14.2); and GNAT 14.2 `-gnatwa -gnatwe`. Each proved the
seven requirements: payload-exact publish; payload-exact handler; no
application-supplied routing strings; the wrong operation absent; the hook
receiving exact topic / QName components / optional group; a custom result
and subscription token; no forced allocation, threading, or lifetime claim.

### Alternatives considered

| Alternative | Verdict |
| --- | --- |
| Emit both Publish and Subscribe everywhere, caller checks `DIRECTION` | Rejected: direction must be a capability, not metadata (§10). |
| Rust: `async fn` / return `impl Future` | Rejected: forces an executor model on every service. The associated `Output` already lets a runtime return a future. |
| Rust: adapter trait with a fixed `Result<(), ServiceApiError>` | Rejected: freezes a cross-language runtime error type. `ServiceApiError` stays a generation/preflight error. |
| Rust: `SubscribeAdapter::subscribe<H: FnMut(&P)>` (method-generic) | Rejected: a runtime could not add `H: Send + 'static`. The handler type is a **trait parameter** instead, so an implementation may bound it (prototype `Strict` adapter required `Send + 'static` and rejected an `Rc` capture with E0277). |
| Rust: `self` by value, implement for `&mut R` | Viable, but `&mut self` with `A: ?Sized` is simpler and also accepts `&mut dyn PublishAdapter<P, Output = X>`. |
| Rust: generated per-endpoint traits | Rejected: a runtime would implement one trait per endpoint per service. Two shared generic traits let a runtime write `impl<P: Codec> PublishAdapter<P> for Runtime` once (prototype `Bridge` in a separate crate compiled — orphan rules allow it). |
| C++: virtual adapter base class | Rejected: introduces a hierarchy, vtables, and ownership policy solely for Task 048. |
| C++: `std::function` handler | Rejected: type erasure implies allocation and copyability. Handlers are perfectly forwarded; `std::is_invocable_v<Handler&, const Payload&>` gives a direct diagnostic. |
| C++: adapter `subscribe` without payload template argument | Rejected: the adapter could not know which payload to decode. The façade calls `adapter.template subscribe<Payload>(...)`. |
| Ada: `access procedure (Message : Payload)` handler | Rejected: anonymous access-to-subprogram handlers cannot be retained by a runtime past the call without unchecked tricks. |
| Ada: `Handler` interface passed `in out` | Workable for immediate delivery, but a runtime cannot keep it. `not null access Handler'Class` lets a runtime retain it under accessibility checks (prototype stored it in a library-level `access all` and delivered later). |
| Ada: generated package body with ordinary subprograms | Not needed: expression functions inside the generic packages keep the spec bodyless, so the artifact layout is unchanged (§18). |
| Ada: `System.Address` / unchecked conversion / `String` payload | Rejected outright. |

## 5. Final generated shapes

Shown for the bidirectional fixture (`facade-bidirectional.yaml`). Task 047
declarations are unchanged and elided (`...`).

### Rust — `service_api.rs`

The root gains two shared adapter contracts, only when the service has at
least one OMS exchange:

```rust
pub mod service_api {
    // ... SERVICE_NAME / SERVICE_VERSION / SERVICE_KIND ...

    pub trait PublishAdapter<P> {
        type Output;
        fn publish(&mut self, topic: &'static str, value: &P) -> Self::Output;
    }

    pub trait SubscribeAdapter<P, H> {
        type Output;
        fn subscribe(
            &mut self,
            message_namespace: &'static str,
            message_name: &'static str,
            topic: &'static str,
            subscription_group: Option<&'static str>,
            handler: H,
        ) -> Self::Output;
    }
```

An input exchange (after its unchanged Task 047 metadata and `Payload`):

```rust
            pub const MESSAGE_NAMESPACE: &str = "urn:test";
            pub const MESSAGE_NAME: &str = "MessageA";
            pub const SUBSCRIPTION_GROUP: Option<&str> = Some("pool 7: \"hot\"");

            pub fn subscribe<A, H>(adapter: &mut A, handler: H) -> A::Output
            where
                A: super::super::SubscribeAdapter<Payload, H> + ?Sized,
                H: FnMut(&Payload),
            {
                adapter.subscribe(MESSAGE_NAMESPACE, MESSAGE_NAME, TOPIC, SUBSCRIPTION_GROUP, handler)
            }
```

An output exchange:

```rust
            pub fn publish<A>(adapter: &mut A, value: &Payload) -> A::Output
            where
                A: super::super::PublishAdapter<Payload> + ?Sized,
            {
                adapter.publish(TOPIC, value)
            }
```

Application code:

```rust
output_endpoint::publish(&mut runtime, &payload);
input_endpoint::subscribe(&mut runtime, |message: &input_endpoint::Payload| { /* typed */ });
```

`H: FnMut(&Payload)` is the only bound the façade imposes — it is what makes
the handler payload-typed. No `Send`, `Sync`, `'static`, `Box`, or executor.

### C++17 — `service_api.hpp`

When the service has any OMS exchange the header additionally includes
`<optional>`, `<type_traits>`, and `<utility>`. No adapter type is declared:
adapters are duck-typed. An input exchange:

```cpp
inline constexpr std::string_view message_namespace = "urn:test";
inline constexpr std::string_view message_name = "MessageA";
inline constexpr std::optional<std::string_view> subscription_group = std::nullopt;

template <typename Adapter, typename Handler>
decltype(auto) subscribe(Adapter&& adapter, Handler&& handler) {
    static_assert(std::is_invocable_v<Handler&, const Payload&>,
                  "subscriber handler must accept const Payload&");
    return std::forward<Adapter>(adapter).template subscribe<Payload>(
        message_namespace, message_name, topic, subscription_group,
        std::forward<Handler>(handler));
}
```

An output exchange:

```cpp
template <typename Adapter>
decltype(auto) publish(Adapter&& adapter, const Payload& value) {
    return std::forward<Adapter>(adapter).publish(topic, value);
}
```

The adapter must provide `publish(std::string_view topic, const P&)` and
`template <typename P, typename H> subscribe(std::string_view namespace,
std::string_view name, std::string_view topic,
std::optional<std::string_view> group, H&& handler)`, returning anything.

```cpp
output_endpoint::publish(runtime, payload);
input_endpoint::subscribe(runtime, [](const input_endpoint::Payload& payload) { /* typed */ });
```

### Ada — `service_api.ads` (still bodyless)

An input exchange declares its routing constants, a `Handler` interface over
exactly its `Payload`, and a generic `Subscriber` bound to a runtime hook:

```ada
         Message_Namespace      : constant String := "urn:test";
         Message_Name           : constant String := "MessageA";
         Has_Subscription_Group : constant Boolean := True;
         Subscription_Group     : constant String := "pool 7: ""hot""";

         type Handler is limited interface;
         procedure Handle
           (Self : in out Handler; Message : Payload) is abstract;

         generic
            type Result (<>) is limited private;
            with function Subscribe_To
              (Message_Namespace      : String;
               Message_Name           : String;
               Topic                  : String;
               Has_Subscription_Group : Boolean;
               Subscription_Group     : String;
               Receiver               : not null access Handler'Class)
               return Result;
         package Subscriber is
            function Subscribe
              (Receiver : not null access Handler'Class) return Result is
              (Subscribe_To
                 (Message_Namespace, Message_Name, Topic,
                  Has_Subscription_Group, Subscription_Group, Receiver));
         end Subscriber;
```

An output exchange:

```ada
         generic
            type Result (<>) is limited private;
            with function Publish_To
              (Topic : String; Value : Payload) return Result;
         package Publisher is
            function Publish (Value : Payload) return Result is
              (Publish_To (Topic, Value));
         end Publisher;
```

Application code, after binding to a runtime (or fake) hook once:

```ada
package Pub is new Output_Endpoint.Publisher (Runtime.Status, Runtime.Publish_To);
package Sub is new Input_Endpoint.Subscriber (Runtime.Token, Runtime.Subscribe_To);

Status := Pub.Publish (Value);
Token  := Sub.Subscribe (Handler_Object'Access);
```

Absence of a group is `Has_Subscription_Group = False` with an empty
`Subscription_Group` — an absence flag, not a fabricated group. `Result (<>)
is limited private` accepts any result the runtime chooses (a status
enumeration, a token record, a limited handle). An Ada runtime may also raise
exceptions. No task, protected object, allocator, `System.Address`, or
unchecked conversion appears. One tagged type may implement several endpoints'
`Handler` interfaces (the fake-hook consumer does).

## 6. Runtime-policy freedom (§9, §27, §28, §30)

| Policy | Where it is decided |
| --- | --- |
| result / error representation | adapter: Rust `Output`, C++ return type, Ada `Result` formal |
| subscription token (for a future UNSUB) | adapter; fake adapters return `Token(u32)`, `Token{int}`, and a `Token` record |
| blocking vs asynchronous | adapter; a Rust `Output` may be a future, a C++ return may be a `std::future`; generated code declares neither |
| handler lifetime / threading | adapter; Rust may bound `H`, C++ receives the forwarded handler, Ada receives `not null access` |
| allocation | none required by the façade in any language |
| JSON / codec, WebSocket, Sleet config | future runtime (Task 049) |

No `async fn`, `Future`, `tokio`, `std::future`, coroutine, or Ada task is
emitted (asserted by `task048_direction_alone_selects_the_operation`). No
Unsubscribe is generated, but no façade forces its result to `()`/`void`, so
a Task 049 runtime can return a subscription handle and implement UNSUB.

## 7. Naming preflight and artifact boundary

Every new identifier is a field of the shared `ServiceApiFixedNames::facade`
(`ServiceApiFacadeNames`, with parameters in `ServiceApiFacadeParameters`), and
the renderers read their spellings only from there.

| Language | Root | Subscribe exchange | Publish exchange | Nested operation region |
| --- | --- | --- | --- | --- |
| Rust | `PublishAdapter`, `SubscribeAdapter` | `MESSAGE_NAMESPACE`, `MESSAGE_NAME`, `SUBSCRIPTION_GROUP`, `subscribe` | `publish` | -- |
| C++ | -- | `message_namespace`, `message_name`, `subscription_group`, `subscribe` | `publish` | -- |
| Ada | -- | `Message_Namespace`, `Message_Name`, `Has_Subscription_Group`, `Subscription_Group`, `Handler`, `Handle`, `Subscriber` | `Publisher` | `Result`, `Subscribe_To`/`Publish_To`, `Subscribe`/`Publish` |

* **Shared name analysis** (`validate_names`, used by readiness via
  `service_api_blocker` and re-run by every backend): root names are claimed
  in the service region only when some exchange is OMS; each OMS exchange
  claims exactly its operation's names (so Subscribe names never appear in a
  Publish scope); the Ada generic package is its own
  `ServiceApiRegion::Operation`. Reserved-word checks and Ada
  case-insensitivity apply unchanged. Unit tests pin legality, reservation,
  non-reachability from prefixed contract IDs, no case-folding onto Task 047
  names, and that Task 047 function/exchange normalization collisions still
  fail closed. No suffixing was added.
* **Parameters** occupy only their own profile. They are checked for
  legality and never to shadow a name the same profile/body uses (the local
  `Payload`, `TOPIC`, the subscription constants, `Handler`, `Result`). The
  Ada hook profile deliberately reuses the constants' spellings; GNAT accepts
  that shape (proved by the consumer tests).
* **Artifact preflight** (`validate_service_api_artifacts`). Root-level
  façade names join the C++ `::service_api` walk and the Ada visibility walk
  (empty for both today; Rust root names cannot meet URI-derived model names).
  Every other façade name is declared inside an exchange scope **after** that
  exchange's `Payload`, and the façade names model types only via `Payload`,
  so no new model-reference site exists. A GNAT sweep generated the Ada
  wrapper against model packages named `Handler`, `Handle`, `Subscriber`,
  `Publisher`, `Result`, `Subscribe`, `Publish`, `Receiver`, `Value`, `Self`,
  `Message`, and more: all READY and all compile. `Topic` stays correctly NOT
  READY (the Task 047 constant hides it, as before). Pinned by
  `ada_facade_names_after_payload_never_hide_the_model`.
* **Ada body.** None is needed; `service_api.adb` is never written (asserted).
  The shared artifact layout and path-collision checks are unchanged.

## 8. Behaviour preserved

* Every Task 047 constant, `Payload` alias, and scope name is unchanged
  (`task048_preserves_every_task047_metadata_spelling`); OMS wrappers change
  only by the added façade declarations and a reworded header comment.
* **Zero-OMS wrappers are byte-identical** to `main` (sha256 Rust
  `c119e1c4...c41`, C++ `388c741b...0d4`, Ada `12a0ac0f...60a`): 0 model files,
  1 wrapper, no adapter infrastructure.
* Model files are byte-identical: `task047_model_files_are_exactly_the_ordinary_backend_output`
  still passes, and every model file of the real UCI run matches `main`.
* Ordinary `generate` emits no façade; `service-plan` and `service-check` are
  read-only; safe `service-check` reports are byte-identical.
* No dependency was added; no generated UCI type changed; type-generation
  coverage is unaffected, so no UCI coverage rerun was needed.

## 9. Evidence

All tests live in `crates/cli/tests/service_facade.rs` unless noted; each uses
a real toolchain and a fake adapter.

### Synthetic bidirectional fixture (`facade-bidirectional.yaml`, `root.xsd`)

Four exchanges, all `MessageA -> PayloadA`, distinct topics; the model
declares `PayloadA` once and all four `Payload` aliases name it.

| Exchange | Operation | Fake adapter observed |
| --- | --- | --- |
| input mandatory | Subscribe only | `urn:test`, `MessageA`, `track.in.mandatory`, no group; handler got `PayloadA` count 41 |
| input optional + group | Subscribe only | same identity, `track.in.optional`, group `pool 7: "hot"` verbatim |
| output mandatory (periodic) | Publish only | `track.out.mandatory` + the typed `PayloadA` |
| output optional (on demand) | Publish only | `track.out.optional` + the typed `PayloadA` |

Adapters returned custom types in every language: Rust `Result<Sent, String>`
/ `Token(u32)`; C++ `int` / `Token{int}`; Ada `Status` enumeration / `Token`
record.

### Negative compile controls (each paired with the positive build)

| Wrong use | Rust (`rustc -D warnings`) | C++ (GCC 14, strict) | Ada (GNAT 14, `-gnatwa -gnatwe`) |
| --- | --- | --- | --- |
| Publish on an input | `cannot find function `publish`` | `'publish' is not a member of` | `"Publisher" not declared in "Exchange_In_Mandatory"` |
| Subscribe on an output | `cannot find function `subscribe`` | `'subscribe' is not a member of` | `"Subscriber" not declared in "Exchange_Out_Mandatory"` |
| publish another type | `mismatched types` | `no matching function for call to` | `expected type "Payload"` |
| handler over another type | `type mismatch in closure arguments` | `subscriber handler must accept const Payload&` (the façade's own `static_assert`) | `not overriding` |

C++ runs with `LC_ALL=C` so GCC's quotes are ASCII. Ada substrings are
chosen to be stable across GNAT 13 and 14; builds use `gnatmake -f` because
the negatives rewrite the same file within one second.

### Other regressions

* **All five kinds** (`api-all-kinds.yaml`): Data Transfer, Special Signal,
  Security Exchange, and Non-OMS Message scopes contain no publish,
  subscribe, handler, message identity, or payload; the three OMS exchanges
  get exactly 1 Subscribe + 2 Publish.
* **Repeated message, opposite directions** (`facade-repeated.yaml`): one
  `PayloadA`, one Subscribe on `position.a`, one Publish on `position.b`, not
  deduplicated.
* **Contract order** (`facade-repeated-reversed.yaml`): model byte-identical;
  wrapper follows the new order; each operation moves with its exchange.
* **Service Status** (`crates/service-contract/tests/fixtures/upstream-service-status.yaml`,
  copied verbatim from the portable project at `4ea5be8d`, against the
  synthetic `service-status.xsd` carrying the same three global message names
  in the UCI namespace): READY in all three languages; operations in contract
  order are **Publish, Subscribe, Publish**; all compile.
* **Upstream PositionReport** (`upstream-minimal.yaml` + `position-report.xsd`):
  Subscribe only; fake adapters saw
  `https://www.vdl.afrl.af.mil/programs/oam` / `PositionReport` / topic
  `PositionReport` / no group, and the handler received a typed
  `PositionReportMT`.

### Real UCI 2.5 `PositionReport`

Pinned `UCI_MessageDefinitions_v2_5_0.xsd` (sha256
`ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27`, local copy
outside the repository) with the verbatim `upstream-minimal.yaml`, closed
world; release builds of `origin/main` (`9ab0e13`) and this branch.

| | Ada | Rust | C++ |
| --- | --- | --- | --- |
| readiness | 60/60 READY | 60/60 READY | 60/60 READY |
| `service-check` report | byte-identical | byte-identical | byte-identical |
| model files | byte-identical (`programs.ads`, `programs-oam.ads`, `programs-oam.adb`) | byte-identical (`oam.rs`) | byte-identical (`oam.hpp`) |
| wrapper | changed: Subscribe added | changed: Subscribe added | changed: Subscribe added |
| Task 047 constants and `Payload = PositionReportMT` | present | present | present |
| Subscribe / Publish | 1 / 0 | 1 / 0 | 1 / 0 |
| fake adapter saw | UCI namespace, `PositionReport`, topic `PositionReport`, no group | same | same |
| Publish-on-input negative | `"Publisher" not declared` | `cannot find function `publish`` | `'publish' is not a member of` |
| `service_api.adb` | not emitted | -- | -- |

Model hashes (prefix): `oam.rs` `0d207cd8`, `oam.hpp` `988348a4`,
`programs.ads` `a5d92d26`, `programs-oam.ads` `1e02efdd`, `programs-oam.adb`
`4ab4ecfd` — identical to `main`. This is compile and fake-adapter evidence
only, not Sleet evidence.

### Workspace validation

`cargo fmt --all -- --check`, `cargo check --workspace --all-targets`,
`cargo clippy --workspace --all-targets -- -D warnings`, and
`git diff --check` are clean. `AMS_GRA_REQUIRE_GNAT=1 cargo test --workspace`:
**936** passed on `origin/main` `9ab0e13` (baseline, measured on this branch
before any change) and **961** passed, 0 failed, 0 ignored, with this change
(+12 CLI façade tests, +10 codegen-core unit tests, +2 codegen-core lowering
tests, +1 contract test; the one adjusted Task 047 CLI assertion keeps its
test count).
CI gains a `require_one_test` hard gate for
`task048_ada_facade_round_trips_through_fake_hooks`; every previous GNAT gate
is kept.

Local toolchains: `rustc 1.98.1`, GCC `c++ 14.2.0` (Debian), GNAT
`GNATMAKE 14.2.0`.

## 10. Still open

- [ ] typed LA-CAL integration (Task 049: implement these adapters over
  WebSocket/OWP, subscription IDs, UNSUB, QName→OWP message-name spelling);
- [ ] codec and runtime integration (OMS JSON);
- kind-specific metadata for the four non-OMS exchange kinds;
- full-UCI generation (Phase 5).
