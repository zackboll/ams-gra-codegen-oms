# CPP-OMS-008 — Standalone C++ UCI Log receiver

A standalone C++17 application,
[`examples/cpp-uci-client`](../examples/cpp-uci-client/README.md), that
receives authentic UCI `Log` messages from unmodified Sleet as
production-generated `programs::oam::LogMT` values, for UCI 2.5 and UCI 2.6.
It is an application feature only:

* no generator, codec, runtime or transport source changed (`crates/`,
  `cpp/oms-lacal-runtime/src|include`, Sleet and the UCI pins are untouched);
* the Task 102 contracts are unchanged; the application has its own
  receive-only contracts;
* no CI workflow changed; PR smoke routing gains one offline unit test.

```text
cpp-uci-client-uci25|26 (src/main.cpp)        CLI, signals, receive loop, exit status
  LogClient (src/client.hpp)                  main thread: connect/subscribe/poll/unsubscribe/close
    service_api::function_log::exchange_receive::subscribe     (generated)
    ams_gra::lacal::Runtime                                    (existing, unchanged)
      payload_codec<LogMT> -> service_codec::decode_LogMT       (generated)
    typed callback (runtime dispatcher thread) -> LogSummary -> BoundedQueue(64)
Sleet e38f61d8 (unmodified)  <- ws:// + owp ->
```

## 1. Generation and release binding

| Release | Contract | `service-check --language cpp --world closed-schema --with-codec` |
| --- | --- | --- |
| 2.5 | `examples/cpp-uci-client/contracts/uci25-service.yaml` | 1 message, closure 42/42 READY, codec 41/41 READY |
| 2.6 | `examples/cpp-uci-client/contracts/uci26-service.yaml` | 1 message, closure 44/44 READY, codec 43/43 READY |

Each contract has one input exchange, `log.receive`: message `Log`, topic
`uci.log`, service `cpp-uci-client`. The closure counts equal those of the
Task 102 contracts. The topic and the absent publish exchange do not change
the type closure.

`build.sh` checks both pinned SHA-256s (MessageDefinitions and
SecurityMarkings), requires those exact readiness lines, generates into
external storage with `service-generate … --with-codec`, and compiles with the
strict flag set used by PR smoke.

`src/release.hpp` binds the release at compile time. The OWP INIT schema is
`release::schema_version` and nothing else. `static_assert`s require that:

* the generated `LogMDT::data` / `LogMDT::serviceuptime` carriers are the
  selected release's (2.5: `std::vector<std::uint8_t>` / `XmlSchemaDuration`;
  2.6: `HexBinaryType` / `DurationType`). These are the only `LogMT` closure
  differences between the two pinned releases;
* the build-provided root hash is the pinned one for that release;
* the generated endpoint is `Payload = LogMT`, `message_name == "Log"`, OAM
  namespace, `topic == "uci.log"`, `direction == "input"`, no group, and
  `service_name == "cpp-uci-client"`.

The following cross-builds were checked to fail at compile time: 2.6 types
with the 2.5 selection, 2.5 types with the 2.6 selection, the 2.6 selection
with the 2.5 hash, and Task 102-generated headers (wrong topic and service).

## 2. Typed reception and threads

The typed callback receives `const programs::oam::LogMT&` on the runtime's
dispatcher thread. It copies only the displayed fields into a `LogSummary`
(severity, classification, schema version, both timestamps, SystemID and the
optional ServiceID/SubsystemID UUIDs), and then calls the non-blocking
`BoundedQueue::push`. The full payload is never copied. The queue mutex is the
only lock the callback takes, and it is never held across a runtime call. The
main thread performs every runtime operation. A full queue sets a sticky
overflow flag: the main thread fails the session (`QUEUE-OVERFLOW`, exit 5)
instead of skipping a Log, and items behind the gap are never displayed.

Lifecycle: `READY` is printed only after SUB `+OK`. Normal, failure and
signal paths all run the same shutdown, on the main thread:

1. `Subscription::unsubscribe()`. The runtime removes local dispatch, waits
   for a running callback, sends UNSUB and waits for `+OK`.
2. Close the queue and count the discarded summaries.
3. Drain the pending events.
4. `Runtime::close()`. This performs the WebSocket close and joins
   `ams-lacal-io` and `ams-lacal-dsp`.
5. Observe `/proc`: the number of live `ams-lacal*` threads, and open socket
   fds compared with the count at start.

SIGINT/SIGTERM handlers store into a `volatile sig_atomic_t` only.

## 3. Error classification

The runtime's `ErrorKind`, OWP code and details are printed unchanged.
Mapping to exit statuses:

| Outcome | Runtime observation | Exit |
| --- | --- | --- |
| invalid CLI, native `wss://` | rejected by `cli.hpp` before any connection | 2 |
| TCP refused | `connect_failed` | 3 |
| missing / wrong `owp`, HTTP 400 | `handshake_rejected` | 3 |
| no upgrade within 5 s | `timeout` | 3 |
| wrong release (INIT schema) | `broker_rejected` `Unsupported-Schema` | 3 |
| unknown service | `broker_rejected` `Unsupported-Service` | 3 |
| unauthorized SUB | `broker_rejected` `Illegal-State` | 4 |
| SUB unacknowledged within 5 s | `timeout` (the runtime fails the connection) | 5 |
| transport loss | `connection_closed` event | 5 |
| callback failure | `handler_exception` event | 5 |
| application queue full | application `QUEUE-OVERFLOW` | 5 |
| decoder rejection of one MSG | `decode_error` event (`codec`); reception continues | — |

No reconnect, resubscribe or replay is attempted.

**Broker versus decoder rejection** is distinguished by observation.

* Missing `Severity`: Sleet `-ERR Invalid-Message … MissingRequiredMember`;
  the independent observer sees no MSG and the client sees no event.
* Global element `LogMT`: Sleet `-ERR Invalid-Message … UnknownGlobalElement`
  for PUB, and `-ERR Illegal-Argument unknown message name LogMT` for a SUB
  that names it.
* `Data` `"000"`: Sleet `+OK`, and the observer sees the MSG byte for byte.
  The client reports `DECODE-ERROR … codec path=LogMT.MessageData.Data`, and
  the next valid Log on the same subscription is received.

The pinned Sleet checks the anonymous `xs:hexBinary` of `Log/MessageData/Data`
only as a string; Task 102 and CPP-OMS-003 recorded the same behavior.

**Payload safety.** Raw MSG text (`Event::payload`) is never printed. A decoder
diagnostic is printed as its codec path. Its free text is printed only if it
contains no quote character, because the generated codec's `unknown member
"…"` diagnostic can echo document content.

## 4. Real-Sleet qualification

`scripts/check-cpp-oms-008-log-client.sh [scratch] [sleet-checkout]` runs the
following per release, against a fresh, unmodified, pinned Sleet loaded with
that release's full schema. Three services are configured: `cpp-uci-client`
(`uci.log`, bound to `Log`), the independent `cpp-uci-wire`, and
`cpp-uci-unauthorized` (`other.topic`).

| Cell | Evidence |
| --- | --- |
| A typed | The application binary prints `READY sid=sub-1` after SUB `+OK`. The independent publisher (`tests/wire_peer.cpp` on `tests/cpp-lacal/raw_owp_client.hpp`; hand-written documents based on the Task 093 golden, checked against `tests/fixtures/service-generate/task093-real-log2{5,6}.json`) sends v1, v2, decoder-bad, v3, broker-bad, LogMT, v4, v5. Checks: five `RECEIVED` lines in order with the expected fields; one `DECODE-ERROR`; two broker `-ERR`s; the observer's six MSG texts byte-identical to the admitted documents; Sleet's own log of the application's INIT, SUB (`sid=sub-1 topic=uci.log message_name=Log`), UNSUB and disconnect; `UNSUBSCRIBED … (+OK)`; `CLOSED … callbacks-after-unsubscribe=0 … runtime-threads=0 … resources-released=yes`; no raw Log text or rejected content in the output. |
| B duration | No traffic; `--duration-seconds 1` gives exit 0, UNSUB and a clean close. |
| C signals | SIGINT gives exit 130 and SIGTERM gives exit 143, each after two receptions, UNSUB `+OK` and a clean close. |
| D negative (Sleet) | The other release's binary gets `Unsupported-Schema` (exit 3). The unauthorized service gets `Illegal-State` on SUB (exit 4, no `READY`). An unknown service gets `Unsupported-Service` (exit 3). `wss://` and six invalid argument sets give exit 2, and Sleet accepts no connection for them. |
| E receiver core | `tests/receiver_test.cpp` (7 cases): order and fields; decoder rejection then a valid Log, versus broker rejection; the wrong global-element name; queue overflow at capacity 3; shutdown with 4 pending summaries and no callbacks after UNSUB/close while more Logs are published; a throwing extractor (`handler_exception`); an unauthorized SUB. |
| F transport | With Sleet stopped, the connection is refused (exit 3). The scripted `tests/negative_peer.cpp` (on `tests/cpp-lacal/mock_peer.hpp`) covers missing owp, `not-owp`, HTTP 400, never-upgrade (5 s), unacknowledged SUB (5 s) and a TCP drop after SUB. Each case is bounded to 15 s and ends with `runtime-threads=0 … resources-released=yes`. |

The negative peer is used only where Sleet cannot be made to misbehave, never
as positive interoperability evidence. Results depend on the run and are
recorded in the PR.

## 5. Regression and PR smoke

* PR smoke (`scripts/pr_smoke.py`, existing `cpp` route): `examples/cpp-uci-client/`
  and `scripts/check-cpp-oms-008-log-client.sh` are C++-owned. One step builds
  `tests/unit_test.cpp` with `CPP_STRICT` against the runtime sources and
  requires exactly `CPP_OMS_008_CLIENT_UNIT_COMPLETE checks=129`. No UCI,
  Sleet or generated UCI type is involved. `scripts/test_pr_smoke.py` covers
  routing, command shape and fail-closed summaries.
* `scripts/check-cpp-oms-003-real-log-sleet.sh` (the existing real-Log runtime
  regression) is rerun unchanged.

## 6. Limitations

* `ws://` only. Native `wss://` is refused, both in the CLI and by the
  runtime. WSS requires a future C++ runtime TLS feature. No TLS proxy is
  used or claimed.
* Only loopback `ws://` against pinned Sleet is qualified, on Linux.
  Thread/socket evidence uses `/proc`.
* `Runtime::close()` waits without bound for a running callback (runtime
  limitation; CPP-OMS-003). The application's callback never blocks.
* The build uses `-O1`. With GCC 14, `-O2` triggers `-Wnull-dereference`
  inside Boost.Asio code inlined into the unchanged `runtime.cpp`; not changed
  here.
* One subscription, one message type, one topic; nothing else is configurable.
