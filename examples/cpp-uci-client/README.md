# Standalone C++17 UCI receiver

A real command-line application, not a test facade. It connects to Sleet
through the **existing** C++ LA-CAL runtime
([`cpp/oms-lacal-runtime`](../../cpp/oms-lacal-runtime/README.md), Boost.Beast,
subprotocol `owp`). It subscribes to the authentic UCI global message **`Log`**
on the application topic **`uci.log`** and receives production-generated,
strongly typed `programs::oam::LogMT` values. It then prints a bounded,
schema-checked summary of each one. An optional build profile adds typed
`RDMA_InitializeSetup` reception on the **same connection**. A third build
profile adds optional `SurvivabilityRiskLevel` reception. All builds run in
Log-only mode by default.

This application adds no runtime, transport, codec or generator. It uses only
these:

| Piece | Source |
| --- | --- |
| Payload type | generated `programs::oam::LogMT` (`oam.hpp`) |
| Subscribe endpoint | generated `service_api::function_log::exchange_receive::subscribe` |
| Payload codec | generated `service_codec::decode_LogMT` / `encode_LogMT`, bound with `AMS_GRA_LACAL_PAYLOAD_CODEC` |
| RDMA endpoint | generated `service_api::function_rdma::exchange_receive::subscribe` (`RDMA_InitializeSetup` on `uci.rdma`) |
| RDMA payload and codec | `programs::oam::RDMAInitializeSetupMT`, `service_codec::decode_RDMAInitializeSetupMT` / `encode_RDMAInitializeSetupMT`, bound with the same macro |
| Survivability endpoint | `service_api::function_survivability::exchange_receive::subscribe` (`SurvivabilityRiskLevel` on `uci.survivability`) |
| Survivability payload and codec | `programs::oam::SurvivabilityRiskLevelMT`, `service_codec::decode_SurvivabilityRiskLevelMT` / `encode_SurvivabilityRiskLevelMT`, bound with the same macro |
| Connection, owp, INIT/INFO, SUB/UNSUB, dispatch | unchanged `ams_gra::lacal::Runtime` |

No UCI schema, generated source, object file or binary is committed. Each build
generates the selected release into external build storage.

## Layout

```text
contracts/uci25-service.yaml   receive-only contract, UCI 2.5 (Log on uci.log)
contracts/uci26-service.yaml   receive-only contract, UCI 2.6 (Log on uci.log)
src/main.cpp                   entry point: CLI, signals, receive loop, exit status
src/client.hpp                 receiver core: runtime + generated endpoint + bounded queue
src/release.hpp                build-time release binding and generated-type checks
src/cli.hpp                    argument validation (before any connection)
src/bounded_queue.hpp          bounded, ordered hand-off (dispatcher -> main thread)
src/log_summary.hpp            the displayed, schema-checked LogMT fields
src/rdma_summary.hpp           the displayed, schema-checked RDMA fields
src/survivability_summary.hpp  the displayed, schema-checked Survivability fields
src/session.hpp                reporting of runtime outcomes; exit-status mapping
src/process_resources.hpp      /proc thread and socket observation after close
build.sh                       pinned-schema check, readiness, generation, strict build
tests/                         qualification only (never linked into the application)
```

`src/` does not include anything from `tests/` or any test-only helper. The
runtime's `detail::TestAccess` seam is not used.

## Prerequisites

* Linux (thread/socket evidence reads `/proc`), Git, GNU coreutils, Python 3
  (for the UCI 2.6 archive extraction in the fetch script and for free-port
  selection in qualification).
* A C++17 compiler. Qualified with GCC 14.2 (`g++`); the unit test is also
  built with Clang 19.
* **Boost ≥ 1.81 headers** (Boost.Beast/Asio; header-only; no Boost library
  is linked; only `-pthread`). Qualified on Boost 1.83.0
  (`BOOST_BEAST_VERSION` 347). On Ubuntu 24.04:
  `sudo apt-get install --no-install-recommends -y libboost-dev`. Headers
  installed elsewhere can be provided with `CPLUS_INCLUDE_PATH`.
* Rust/Cargo for the production generator CLI (`ams-gra-codegen-oms`). Sleet
  pins its own toolchain, Rust 1.95.0.

No new C++ dependency is introduced beyond what the runtime already requires.

## Prepare the pinned UCI schemas

From the repository root (network access is needed for the first fetch):

```bash
export CLIENT_SCRATCH="$(mktemp -d)"   # external storage; never inside the repository
export AMS_GRA_UCI_2_5_ROOT="$(scripts/fetch-pinned-uci-2.5.sh "$CLIENT_SCRATCH/uci25")"
export AMS_GRA_UCI_2_6_ROOT="$(scripts/fetch-pinned-uci-2.6.sh "$CLIENT_SCRATCH/uci26")"
```

| Release | MessageDefinitions SHA-256 | SecurityMarkings SHA-256 | OWP schema |
| --- | --- | --- | --- |
| 2.5 | `ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27` | `4a8056f1503234d423a0c2495844ab65387febced62bea58b87d4f343e9e7a0b` | `002.5.0` |
| 2.6 | `af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b` | `ee6e58d5db9fd80d526bc964b8244ec296d106301640d34c3cb2c5c00b710b88` | `002.6.0` |

`build.sh` verifies both hashes again; they are the pins in
`scripts/task102/admission.py`.

## Build

```bash
# UCI 2.5
examples/cpp-uci-client/build.sh --release 2.5 --schema "$AMS_GRA_UCI_2_5_ROOT" --out "$CLIENT_SCRATCH/build"
# UCI 2.6
examples/cpp-uci-client/build.sh --release 2.6 --schema "$AMS_GRA_UCI_2_6_ROOT" --out "$CLIENT_SCRATCH/build"
# Both message families, UCI 2.6 (use --release 2.5 and the matching root for 2.5)
examples/cpp-uci-client/build.sh --with-rdma --release 2.6 --schema "$AMS_GRA_UCI_2_6_ROOT" --out "$CLIENT_SCRATCH/build"
# All three families, UCI 2.6 (use --release 2.5 and its root for 2.5)
examples/cpp-uci-client/build.sh --with-rdma --with-survivability --release 2.6 --schema "$AMS_GRA_UCI_2_6_ROOT" --out "$CLIENT_SCRATCH/build"
```

Each build:

1. Verifies the MessageDefinitions file and its sibling SecurityMarkings
   against the pins. A file from the other release, or an unpinned file, is
   refused.
2. Runs the production readiness check and requires model, service API and
   codec READY:
   `service-check --language cpp --world closed-schema --with-codec`. The
   actual counts are below.
3. Generates `oam.hpp`, `service_api.hpp` and `service_codec.hpp` into
   `BUILD/generated/uci25|uci26`, using
   `service-generate --language cpp --world closed-schema --with-codec`.
4. Compiles `src/main.cpp`, the runtime's `owp.cpp` and `runtime.cpp`, and
   the generated headers with strict C++17 flags (`-Wall -Wextra -Wpedantic
   -Werror -Wconversion -Wsign-conversion -Wshadow -Wold-style-cast …`, `-O1`)
   into `BUILD/bin/cpp-uci-client-uci25` or `cpp-uci-client-uci26`.
5. Writes `BUILD/build-uci25|26.manifest`, which records the pins, the
   contract, the generated-header hashes, the compiler, Boost and the binary
   hash.

The combined profile uses the receive-only contracts in
`tests/fixtures/cpp-codec/addresses/uci{25,26}-log-rdma-service.yaml` and adds
`-log-rdma` to the generated/object directories, manifest and executable names
(for example `bin/cpp-uci-client-uci26-log-rdma`). It does not overwrite
Log-only artifacts.

| Build profile | Build switches | Artifact suffix and generation directory | Runtime modes |
| --- | --- | --- | --- |
| A | None | `uci25` / `uci26` | Log |
| B | `--with-rdma` | `uci25-log-rdma` / `uci26-log-rdma` | Log; Log + RDMA |
| C | `--with-rdma --with-survivability` | `uci25-log-rdma-survivability` / `uci26-log-rdma-survivability` | Log; Log + RDMA; all three |

Profile C takes its receive-only contracts from `contracts/`, and keeps its
binary, manifest, generated headers and objects separate from A and B.

| Release | Selected messages | Type closure (model + API) | Codec declarations |
| --- | --- | --- | --- |
| 2.5 | 1 (`Log`) | 42/42 READY | 41/41 READY |
| 2.6 | 1 (`Log`) | 44/44 READY | 43/43 READY |
| 2.5 | 2 (`Log` + `RDMA_InitializeSetup`) | 55/55 READY | 54/54 READY |
| 2.6 | 2 (`Log` + `RDMA_InitializeSetup`) | 57/57 READY | 56/56 READY |
| 2.5 | 3 (Log + RDMA + Survivability) | 64/64 READY | 63/63 READY |
| 2.6 | 3 (Log + RDMA + Survivability) | 65/65 READY | 64/64 READY |

`CXX` selects the compiler. `AMS_GRA_CLI` names a prebuilt production CLI;
without it, `build.sh` builds the CLI with `cargo build --locked --release`
into `BUILD/cargo`.
`CPP_UCI_CLIENT_EXTRA_CXXFLAGS` adds flags, for example sanitizers. The script
refuses an `--out` inside the repository. It builds at `-O1`, the level the
CPP-OMS-003 runtime qualification uses: with GCC 14, `-O2` triggers
`-Wnull-dereference` (kept as an error) inside Boost.Asio code inlined into the
unchanged `runtime.cpp`.

### The release is fixed at build time

`src/release.hpp` gets `CPP_UCI_CLIENT_UCI_RELEASE` and the schema root hash
from `build.sh`. The release, the OWP INIT `schema` (`002.5.0` or `002.6.0`)
and the `--help` banner are compile-time constants. No command-line option or
environment variable changes them at run time. `static_assert`s compare those
constants with the generated types that are actually compiled in:

* the release's generated carriers of `MessageData.Data` and
  `MessageData.ServiceUpTime` (2.5: `std::vector<std::uint8_t>` /
  `XmlSchemaDuration`; 2.6: `HexBinaryType` / `DurationType`);
* the pinned root hash of the selected release;
* the endpoint identity: payload `LogMT`, global message `"Log"` (never
  `"LogMT"`), OAM namespace, topic `uci.log`, input direction, no group,
  and the correct generated service name for the build profile;
* the combined profile's RDMA endpoint: payload `RDMAInitializeSetupMT`, global
  message `RDMA_InitializeSetup`, OAM namespace, topic `uci.rdma`, input, no group.
* the three-message profile's third endpoint: payload `SurvivabilityRiskLevelMT`,
  global message `SurvivabilityRiskLevel`, OAM namespace, topic
  `uci.survivability`, input, no group.

Compiling 2.5 types with the 2.6 selection (or vice versa), or with headers
generated from a different contract, fails the build.

## Run against Sleet

Use the unmodified pinned Sleet `e38f61d8ce0d75c8508434a52f2ed77c69cf6a3b`
(tree `7e705f4eb5f658957bd414b4abc8207311f3c1e4`), and load the **same**
release's full schema:

```bash
git clone https://github.com/open-arsenal/ams-gra-hello-world-sk-infra-sleet "$CLIENT_SCRATCH/sleet"
git -C "$CLIENT_SCRATCH/sleet" checkout --detach e38f61d8ce0d75c8508434a52f2ed77c69cf6a3b
test -z "$(git -C "$CLIENT_SCRATCH/sleet" status --porcelain)"
(cd "$CLIENT_SCRATCH/sleet" && cargo build --release --locked -p sleet)

mkdir -p "$CLIENT_SCRATCH/services"
cat > "$CLIENT_SCRATCH/services/cpp-uci-client.toml" <<'TOML'
service_id = "cpp-uci-client"
service_uuid = "550e8400-e29b-41d4-a716-446655440108"
allowed_topics = ["uci.log"]

[[topic_bindings]]
topic = "uci.log"
allowed_messages = ["Log"]
TOML
cat > "$CLIENT_SCRATCH/server.toml" <<TOML
bind_addr = "127.0.0.1:9000"
server_id = "cpp-uci-example"
system_label = "Standalone C++ UCI Log receiver"
schema_path = "$AMS_GRA_UCI_2_6_ROOT"
schema_version = "002.6.0"
services_dir = "$CLIENT_SCRATCH/services"
TOML
"$CLIENT_SCRATCH/sleet/target/release/sleet" "$CLIENT_SCRATCH/server.toml" &

"$CLIENT_SCRATCH/build/bin/cpp-uci-client-uci26" \
  --url ws://127.0.0.1:9000 --service-id cpp-uci-client --duration-seconds 30 --max-messages 10
```

For UCI 2.5, use `$AMS_GRA_UCI_2_5_ROOT`, `schema_version = "002.5.0"` and
`cpp-uci-client-uci25`. A binary whose release differs from the broker's is
refused by Sleet at INIT (`-ERR Unsupported-Schema`, exit 3).

### Required topic authorization

The service ID passed with `--service-id` must be a Sleet service whose
`allowed_topics` includes **`uci.log`**. Binding the topic to the global
message **`Log`** with `[[topic_bindings]]`, as above, is recommended. Without
authorization, Sleet answers the SUB with
`-ERR Illegal-State … not authorized to subscribe to topic uci.log`, and the
application exits with status 4 without printing `READY`.

For combined mode, change `allowed_topics` to `["uci.log", "uci.rdma"]`,
keep the Log binding, and add this binding for the same service (also authorize
each publisher for the topics it publishes):

```toml
[[topic_bindings]]
topic = "uci.rdma"
allowed_messages = ["RDMA_InitializeSetup"]
```

```bash
"$CLIENT_SCRATCH/build/bin/cpp-uci-client-uci26-log-rdma" --with-rdma \
  --url ws://127.0.0.1:9000 --service-id cpp-uci-client --max-messages 10
```

For all three, set `allowed_topics = ["uci.log", "uci.rdma", "uci.survivability"]`
and add this binding without removing the Log or RDMA bindings:

```toml
[[topic_bindings]]
topic = "uci.survivability"
allowed_messages = ["SurvivabilityRiskLevel"]
```

```bash
"$CLIENT_SCRATCH/build/bin/cpp-uci-client-uci26-log-rdma-survivability" \
  --with-rdma --with-survivability --url ws://127.0.0.1:9000 \
  --service-id cpp-uci-client --max-messages 10
```

## CLI

```text
cpp-uci-client-uci26 --url ws://HOST[:PORT][/PATH] --service-id ID [OPTIONS]
  --url URL               Sleet endpoint; ws:// only (native wss:// TLS is not supported)
  --service-id ID         OWP service identity authorized for 'Log' on 'uci.log'
  --duration-seconds N    Receive for at most N seconds, 1..3600 (default 30)
  --max-messages N        Stop after N typed Log messages, 1..100000 (default 10)
  --with-rdma             Combined build only: enable the additional RDMA subscription
  --with-survivability    Profile C only, requires --with-rdma: enable SurvivabilityRiskLevel
  --help                  Print help (including the built-for release) without connecting
```

The binary validates every argument before it opens a connection. Each option
may appear once; the two subscription flags take no value, and value options take one
separate value. `--max-messages` counts typed summaries across enabled
subscriptions (decoder rejections do not count). `--opt=value`, unknown
options and extra arguments are rejected. Topic, message, schema, group,
reconnect and replay are **not** configurable. Unsupported switches and
`--with-survivability` without `--with-rdma` exit with status 2 before connecting.

Fixed runtime settings (`src/client.hpp`, all inside the runtime's supported
`Config`):

| Setting | Value |
| --- | --- |
| Connect (TCP + upgrade + INIT/+OK/INFO) | 5 s |
| Each acknowledged command (SUB, UNSUB) | 5 s |
| WebSocket close handshake | 2 s |
| Largest incoming frame | 1 MiB |
| Runtime MSG backlog | 1024 messages / 16 MiB |
| Runtime event backlog | 256 events / 256 KiB |
| Application hand-off queue | 64 summaries |

## Example output

From the UCI 2.6 qualification run against pinned Sleet. The independent
publisher sent five valid Logs and one Log whose `Data` is `"000"`. Sleet
admits that document (it checks the anonymous `xs:hexBinary` only as a
string), and the generated decoder rejects it:

```text
CPP-UCI-CLIENT built-for=UCI-2.6 schema=002.6.0 message=Log topic=uci.log max-messages=5 duration-seconds=30
CONNECTED owp=1.0 schema=002.6.0 server=cpp-oms-008
READY sid=sub-1 message=Log topic=uci.log (SUB acknowledged by the broker)
RECEIVED Log #1 severity=WARNING classification=U schema=002.6.0 header-timestamp=2026-01-02T03:04:05Z log-timestamp=2026-01-02T03:05:01Z system=12345678-1234-1234-8234-123456789abc service=550e8400-e29b-41d4-a716-446655440108 subsystem=550e8400-e29b-41d4-a716-446655440208
RECEIVED Log #2 severity=INFO classification=U schema=002.6.0 header-timestamp=2026-01-02T03:04:05Z log-timestamp=2026-01-02T03:05:02Z system=12345678-1234-1234-8234-123456789abc
DECODE-ERROR Log rejected by the client: codec path=LogMT.MessageData.Data reason="not hexBinary: odd number of hexadecimal digits" payload-bytes=480 (not displayed)
RECEIVED Log #3 severity=WARNING classification=U schema=002.6.0 header-timestamp=2026-01-02T03:04:05Z log-timestamp=2026-01-02T03:05:03Z system=12345678-1234-1234-8234-123456789abc service=550e8400-e29b-41d4-a716-446655440108 subsystem=550e8400-e29b-41d4-a716-446655440208
RECEIVED Log #4 severity=INFO classification=U schema=002.6.0 header-timestamp=2026-01-02T03:04:05Z log-timestamp=2026-01-02T03:05:04Z system=12345678-1234-1234-8234-123456789abc
RECEIVED Log #5 severity=WARNING classification=U schema=002.6.0 header-timestamp=2026-01-02T03:04:05Z log-timestamp=2026-01-02T03:05:05Z system=12345678-1234-1234-8234-123456789abc service=550e8400-e29b-41d4-a716-446655440108 subsystem=550e8400-e29b-41d4-a716-446655440208
STOP max-messages=5 reached
UNSUBSCRIBED sid=sub-1 acknowledged by the broker (+OK)
CLOSED close=ok callbacks=5 callbacks-after-unsubscribe=0 pending-discarded=0 queue-accepted=5 queue-rejected-full=0 queue-rejected-closed=0 queue-high-water=1 delivered=5 decode-errors=1 handler-exceptions=0 unknown-subscription=0 discarded-on-close=0 runtime-threads=0 sockets=1 sockets-at-start=1 resources-released=yes
```

### What is displayed

Only these generated, decoder-validated fields are displayed:

| Output | Generated field | Type |
| --- | --- | --- |
| `severity` | `messagedata.severity` | `LogSeverityEnum` (exhaustive switch) |
| `classification` | `securityinformation.classification` | `ClassificationEnum` (exhaustive switch) |
| `schema` | `messageheader.schemaversion` | `UCISchemaVersionStringType` |
| `header-timestamp` | `messageheader.timestamp` | `DateTimeType` |
| `log-timestamp` | `messagedata.timestamp` | `DateTimeType` |
| `system` | `messageheader.systemid.uuid` | `UniversallyUniqueIdentifierType` |
| `header-service` (if present) | `messageheader.serviceid->uuid` | ″ |
| `service` / `subsystem` (if present) | `messagedata.serviceid->uuid` / `subsystemid->uuid` | ″ |

The typed callback copies only these fields. The full `LogMT` is never copied
or queued. Raw Log text (`Label`, `Details`), binary `Data`, security caveats
other than the classification, raw MSG text and rejected payload content are
never printed. A decoder diagnostic is reported by its schema-derived codec
path. Its free text is withheld when it could quote the rejected document.

Example combined-mode lines (Log lines retain the format above):

```text
READY sid=sub-1 message=Log topic=uci.log sid=sub-2 message=RDMA_InitializeSetup topic=uci.rdma (both SUBs acknowledged by the broker)
RECEIVED RDMA_InitializeSetup #2 setup=12345678-1234-1234-8234-123456789abc provider=TCP classification=U schema=002.6.0 header-timestamp=2026-01-02T03:04:05Z endpoints=2 ipv4=192.0.2.10 ipv6=2001:db8::1
UNSUBSCRIBED sid=sub-2 acknowledged by the broker (+OK)
UNSUBSCRIBED sid=sub-1 acknowledged by the broker (+OK)
```

RDMA shows the setup UUID, provider enum, classification, schema version,
header timestamp, total endpoint count and the first four addresses in their
original spelling and endpoint order. It counts any omitted endpoints, and
never shows endpoint names, descriptions, security caveats or rejected JSON.
The client only receives UCI messages; it does not initiate RDMA traffic.

Survivability prints the risk-level UUID, checked header mode, classification,
schema version, header timestamp and total risk-setting count. It displays the
first three settings and first four `AllowedRisk` values per displayed setting
in list order, with counts for omitted values. The pinned schema permits 1–4
settings and 1–5 levels per setting. `RiskSettingType` is a record; the generated
enums are `RiskPrioritizationEnum` and `VulnerabilityTypeEnum`. Descriptive
labels, security caveats, `AppliesToSystemID`, exposure durations and raw
payloads never appear. Decoder events show only the matched global name, error
category, a known schema path (or ancestor) and payload byte count. Handler
exceptions have no payload length in the runtime event; free text is withheld.

## Exit statuses

| Status | Meaning |
| --- | --- |
| 0 | `--max-messages` or `--duration-seconds` reached; UNSUB acknowledged; runtime closed |
| 1 | unexpected internal failure |
| 2 | invalid arguments, including native `wss://` (nothing was connected) |
| 3 | connect failed: TCP refused, upgrade declined, wrong/missing `owp`, INIT `-ERR` (for example `Unsupported-Schema` or `Unsupported-Service`), INFO invalid, or connect timeout |
| 4 | the broker rejected an enabled SUB (`-ERR`, for example `Illegal-State` for an unauthorized topic) |
| 5 | session failure after connecting: SUB/operation timeout, transport loss, application queue overflow, or a failure inside the typed callback |
| 6 | the run was otherwise clean, but UNSUB or close did not succeed, or a runtime thread or socket remained |
| 130 / 143 | stopped by SIGINT / SIGTERM, after the normal UNSUB and close |

Every failure line keeps the runtime's own classification (`kind=<ErrorKind>`,
`owp=<code>`, `details=…`). The application adds no recovery semantics: it
never reconnects, resubscribes or replays. The first failure determines the
exit status, and the normal shutdown still runs.

## Resource and lifecycle behavior

* **Threads.** The runtime owns two threads (`ams-lacal-io`,
  `ams-lacal-dsp`). The typed callback runs on the dispatcher. It extracts the
  summary and calls a non-blocking `push` into a bounded queue of 64, then
  returns. Every runtime operation (`connect`, `subscribe`, `unsubscribe`,
  `poll_event`, `close`) runs on the application's main thread. No
  application lock is held while a runtime operation runs. The runtime is
  never closed or destroyed from its handler.
* **READY** follows all enabled SUB acknowledgments. After a single INIT/INFO,
  the client subscribes to Log first, RDMA second if enabled, and Survivability
  third if enabled. A failed SUB prevents `READY`; the client explicitly releases
  all previously acknowledged SIDs in reverse order. It checks connection
  health after the final combined-mode acknowledgment. A subsequent transport
  failure can still occur after that check.
* **Order.** The shared FIFO has one display sequence (`#1, #2, …`). It preserves
  callback order within each SID. There is no publication-order guarantee across
  independent subscriptions.
* **Bounded.** The receive period (`--duration-seconds`) and the message count
  (`--max-messages`) are both bounded. If the application queue fills, the
  session fails explicitly (`QUEUE-OVERFLOW`, exit 5) instead of silently
  skipping a message. The single dispatcher writes to a preallocated SPSC
  ring without acquiring an application lock or waiting for space.
* **Shutdown** (normal, failure or signal): explicit UNSUB for each established
  SID in reverse order (Survivability, RDMA, Log when all are enabled),
  then the queue is closed. Pending summaries are discarded and counted in
  `pending-discarded`. Then `Runtime::close()` performs the WebSocket close
  and joins both runtime threads. After UNSUB the runtime guarantees the
  callback is neither running nor invoked again, and the application counts
  `callbacks-after-unsubscribe` to confirm it. `CLOSED … runtime-threads=0
  sockets=N sockets-at-start=N resources-released=yes` reports `/proc`
  evidence: no live `ams-lacal*` thread and no socket beyond those open at
  start. A dead transport removes registrations locally; it is never reported
  as an acknowledged UNSUB. Combined mode reports per-family callback counts and
  decoder/handler events observed in the runtime event queue, attributed by both
  SID and global message name. If events are dropped, per-family diagnostic
  counts may be incomplete; use the aggregate runtime counts as well.
* **Signals.** SIGINT/SIGTERM handlers only store the signal number in a
  lock-free atomic. During reception the main loop observes it within about
  25 ms. Setup operations are synchronous: a signal is acted on after the
  outstanding operation returns or times out. A successful pending SUB is
  tracked and unsubscribed; an operation failure keeps its own exit status.

## `ws://` versus `wss://`

The C++ LA-CAL runtime implements **`ws://` only** and refuses native
`wss://` (`ErrorKind::unsupported_scheme`). This application keeps that
behavior. It rejects `wss://` URLs before any connection, with exit status 2
and this message:

```text
ERROR native wss:// (TLS) is not supported: the C++ LA-CAL runtime implements ws:// only and refuses wss:// rather than weakening TLS; use a ws:// endpoint (see examples/cpp-uci-client/README.md)
```

It does not downgrade to `ws://`, emulate TLS, or rely on a TLS proxy. A
deployment that requires WSS needs native TLS (certificate validation,
hostname verification, a TLS stream under Beast) in the C++ runtime. That is a
separate, future C++ runtime feature and is out of scope here. Only loopback
`ws://` is qualified.

## Qualification

```bash
# offline unit test (also run by PR smoke)
g++ -std=c++17 -Wall -Wextra -Wpedantic -Werror -pthread -I cpp/oms-lacal-runtime/include \
  -I examples/cpp-uci-client/src examples/cpp-uci-client/tests/unit_test.cpp \
  cpp/oms-lacal-runtime/src/owp.cpp cpp/oms-lacal-runtime/src/runtime.cpp -o "$CLIENT_SCRATCH/unit"
"$CLIENT_SCRATCH/unit"

# both releases against unmodified pinned Sleet (network for the first fetches)
scripts/check-cpp-oms-008-log-client.sh "$CLIENT_SCRATCH/qualification" [existing-sleet-checkout]
```

The test programs in `tests/` are qualification only. `wire_peer.cpp` is the
independent observer and publisher, built on
`tests/cpp-lacal/raw_owp_client.hpp`; it never uses the generated codec.
`receiver_test.cpp` exercises the receiver core against real Sleet.
`negative_peer.cpp` covers transport failures that Sleet cannot be made to
exhibit, using `tests/cpp-lacal/mock_peer.hpp`. See
[docs/task-cpp-oms-008-standalone-log-client.md](../../docs/task-cpp-oms-008-standalone-log-client.md)
for the original design and evidence, and
[CPP-OMS-009 qualification](../../docs/task-cpp-oms-009-multimessage-client.md)
for the two-message reproduction and evidence, and
[CPP-OMS-010 qualification](../../docs/task-cpp-oms-010-survivability-client.md)
for the three-message profile.
