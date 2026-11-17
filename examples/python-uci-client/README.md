# Standalone Python UCI RDMA (and optional Log) receiver

A command-line application that connects to a Sleet OMS endpoint, subscribes
to the authentic UCI global message **`RDMA_InitializeSetup`** and hands each
received message, decoded into the generated payload type
**`RDMA_InitializeSetupMT`**, to an application callback. The default callback
prints selected typed fields.

With **`--with-log`** (PY-OMS-021) the same single authenticated connection
also subscribes to the UCI global message **`Log`** and delivers generated
**`LogMT`** values to the same callback. Without it, behavior, output and exit
statuses are exactly those of the RDMA-only receiver (PY-OMS-020).

It is built only from existing, tested pieces:

- the Python OMS LA-CAL runtime `oms_lacal` (`python/oms-lacal-runtime`), which
  owns WebSocket/TLS, `owp` negotiation, INIT/INFO, SUB/UNSUB, the strict
  global-element envelope and bounded dispatch;
- the actual `model.py`, `service_codec.py` and `service_api.py` produced by the
  production generator (`service-generate --language python --world
  closed-schema --with-codec`) from the pinned UCI 2.5 or 2.6 schema.

The application frames no OWP and decodes no UCI JSON itself. Neither UCI
schemas nor generated UCI source are committed.

| File | Purpose |
| --- | --- |
| `python_uci_client.py` | CLI and reusable `run_client()` API |
| `generate_bindings.py` | Generates one release into external build storage |
| `contracts/uci25-service.yaml`, `contracts/uci26-service.yaml` | Receive-only RDMA contracts derived from `tests/fixtures/task103` |
| `contracts/uci25-rdma-log-service.yaml`, `contracts/uci26-rdma-log-service.yaml` | Receive-only RDMA + Log contracts (`--with-log`); Log as in `tests/fixtures/task102` |
| `tests/` | CLI, session, multimessage, real-Sleet and WSS qualification |

## 1. Python environment and runtime

Python **3.11 or newer** (qualified on 3.11 and 3.13). From the repository root:

```bash
python3.11 -m venv "$HOME/.venvs/python-uci-client"
"$HOME/.venvs/python-uci-client/bin/python" -m pip install ./python/oms-lacal-runtime
export PY="$HOME/.venvs/python-uci-client/bin/python"
```

This installs `ams-gra-oms-lacal-runtime` with its single pinned dependency
`websockets==17.2`. The generated UCI modules need nothing else.

## 2. Generate UCI 2.5 or 2.6 bindings

Build the production generator and fetch the pinned public schema **outside**
the repository. The fetch scripts verify the tag commit and root SHA-256:

```bash
cargo build --release --locked -p ams-gra-codegen-oms
export CLIENT_SCRATCH="$(mktemp -d)"
UCI25="$(scripts/fetch-pinned-uci-2.5.sh "$CLIENT_SCRATCH/uci-2.5")"
UCI26="$(scripts/fetch-pinned-uci-2.6.sh "$CLIENT_SCRATCH/uci-2.6")"
```

Generate one release (or both) into a new directory outside the repository:

```bash
"$PY" examples/python-uci-client/generate_bindings.py \
  --schema-release 2.5 --schema "$UCI25" --output "$CLIENT_SCRATCH/rdma-2.5"
"$PY" examples/python-uci-client/generate_bindings.py \
  --schema-release 2.6 --schema "$UCI26" --output "$CLIENT_SCRATCH/rdma-2.6"
```

The helper refuses a schema root or `UCI_SecurityMarkings_v2_<n>_0.xsd`
include whose SHA-256 differs from the release pin, refuses to write inside the
repository (judged on the fully resolved path, so a symlinked ancestor cannot
alias it) or into a non-empty directory, and records
`python-uci-client-build.json` with the release and the SHA-256 of the three
generated files. `--generator PATH` (or `AMS_GRA_GENERATOR`) selects another
production binary; the default is `target/release/ams-gra-codegen-oms`.

| Release | Root SHA-256 | INIT schema version |
| --- | --- | --- |
| 2.5 (`v2.5`, `093610b7…`) | `ac943049…a7bf27` | `002.5.0` |
| 2.6 (`v2.6`, `78eb61b6…`) | `af54ce72…59955b` | `002.6.0` |

### RDMA + Log bindings (`--with-log`)

Add `--with-log` to select both message families from the same pinned
release. One `model.py`, one `service_codec.py` and one `service_api.py` then
contain both messages:

```bash
"$PY" examples/python-uci-client/generate_bindings.py --with-log \
  --schema-release 2.5 --schema "$UCI25" --output "$CLIENT_SCRATCH/rdma-log-2.5"
"$PY" examples/python-uci-client/generate_bindings.py --with-log \
  --schema-release 2.6 --schema "$UCI26" --output "$CLIENT_SCRATCH/rdma-log-2.6"
```

| Global message (OWP wire name) | Generated payload type | Topic | Generated endpoint |
| --- | --- | --- | --- |
| `RDMA_InitializeSetup` | `model.RDMA_InitializeSetupMT` | `uci.rdma` | `function_rdma.exchange_receive` |
| `Log` | `model.LogMT` | `uci.log` | `function_log.exchange_receive` |

The combined manifest has `"format": 2`, `"with_log": true` and a `messages`
list naming both global messages, payload types and topics, plus the same pin,
contract, generator and per-file digests as before. RDMA-only generation still
writes the unchanged PY-OMS-020 manifest (`"format": 1`) and byte-identical
generated files, and existing PY-OMS-020 binding directories keep loading.
The client refuses `--with-log` with RDMA-only bindings, combined bindings
without `--with-log`, a release mismatch, a modified file, and generated
sources lacking the Log endpoint or `LogMT` codec (exit `3`).

## 3. Run against a Sleet endpoint

The broker must load the same UCI release and authorize the service identity
for topic `uci.rdma` (the generated contract topic). A Sleet service file, for
example `services.d/python-uci-client.toml`:

```toml
service_id = "python-uci-client"
service_uuid = "550e8400-e29b-41d4-a716-446655440020"
allowed_topics = ["uci.rdma"]
[[topic_bindings]]
topic = "uci.rdma"
allowed_messages = ["RDMA_InitializeSetup"]
```

Then:

```bash
"$PY" examples/python-uci-client/python_uci_client.py \
  --url ws://127.0.0.1:9000 --service-id python-uci-client \
  --schema-release 2.5 --bindings "$CLIENT_SCRATCH/rdma-2.5" \
  --duration-seconds 30
```

| Option | Meaning |
| --- | --- |
| `--url` | `ws://` or `wss://` endpoint; no credentials or fragment |
| `--service-id` | OWP service identity (`[A-Za-z0-9_.-]+`) authorized by the broker |
| `--schema-release` | `2.5` or `2.6`; must match the generated bindings |
| `--bindings` | Output directory of `generate_bindings.py` |
| `--duration-seconds` | Receive window after SUB `+OK`, 1–86400 (default 10) |
| `--max-messages` | Optional early stop after N typed messages, 1–1000000 |
| `--ca-file` | Extra trusted CA PEM for `wss://` only |
| `--with-log` | Also receive `Log` on `uci.log` over the same connection; needs `--with-log` bindings |
| `--help` | Usage and exit statuses |

All arguments are validated before any connection is opened. Contradictions
are rejected: `--ca-file` with `ws://`, a release that does not match the
bindings manifest, modified generated files, duplicated or unknown options.
The selected release fixes the INIT schema version. There is no override.

### Expected output

This is the actual UCI 2.5 output when three independently published messages
arrive on real pinned Sleet. One of them has a schema-invalid date that the
broker accepts but the generated decoder rejects:

```text
CONNECTED ws://127.0.0.1:9000 owp=1.0 service-id=python-uci-client schema=002.5.0
SUBSCRIBED RDMA_InitializeSetup topic=uci.rdma sid=sub-1 payload=RDMA_InitializeSetupMT duration=30s
RECEIVED RDMA_InitializeSetup #1 type=RDMA_InitializeSetupMT setup-id=5b0f6d4e-1c2a-4b7e-9a3d-0123456789ab provider=TCP mode=LIVE schema=002.5.0 timestamp=2026-01-02T03:04:05Z classification=U endpoints=1 [IPv4 198.51.100.42/24]
RECEIVED RDMA_InitializeSetup #2 type=RDMA_InitializeSetupMT setup-id=00000000-0000-0000-0000-000000000000 provider=VERBS mode=LIVE schema=002.5.0 timestamp=2026-06-30T23:59:59.5Z classification=U endpoints=2 [IPv6 fE80::aBcD, IPv4 198.51.100.42/24]
RECEIVED RDMA_InitializeSetup #3 type=RDMA_InitializeSetupMT setup-id=aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee provider=UDP mode=LIVE schema=002.5.0 timestamp=2026-01-02T03:04:05Z classification=U endpoints=0 []
UNSUBSCRIBED sid=sub-1
CLOSED received=3 rejected=1 callback-failures=0
```

On standard error: `REJECTED RDMA_InitializeSetup payload (CodecFailure:
payload decoder rejected message)`. That run exits `7`.

Only schema-constrained values are printed: UUIDs, enumerations, the header
timestamp and schema version, and the checked `IPv4_AddressType` /
`IPv6_AddressType` carriers. The IPv6 spelling is preserved exactly, as in
PY-017. Free-text members (descriptions, labels, endpoint names) and rejected
payload content are never printed. The URL path and query are not echoed.

### RDMA + Log on one connection (`--with-log`)

The broker must authorize **both** topics for the service identity, each bound
to its one global message:

```toml
service_id = "python-uci-client"
service_uuid = "550e8400-e29b-41d4-a716-446655440020"
allowed_topics = ["uci.rdma", "uci.log"]
[[topic_bindings]]
topic = "uci.rdma"
allowed_messages = ["RDMA_InitializeSetup"]
[[topic_bindings]]
topic = "uci.log"
allowed_messages = ["Log"]
```

```bash
"$PY" examples/python-uci-client/python_uci_client.py \
  --url ws://127.0.0.1:9000 --service-id python-uci-client \
  --schema-release 2.6 --bindings "$CLIENT_SCRATCH/rdma-log-2.6" \
  --with-log --duration-seconds 30
```

This is the actual UCI 2.6 output on real pinned Sleet. An independent
publisher sent RDMA, Log, RDMA, then a Log whose date the broker accepts but
the generated `LogMT` decoder rejects:

```text
CONNECTED ws://127.0.0.1:9000 owp=1.0 service-id=python-uci-client schema=002.6.0
SUBSCRIBED RDMA_InitializeSetup topic=uci.rdma sid=sub-1 payload=RDMA_InitializeSetupMT
SUBSCRIBED Log topic=uci.log sid=sub-2 payload=LogMT
READY subscriptions=2 messages=RDMA_InitializeSetup,Log duration=2s
RECEIVED RDMA_InitializeSetup #1 type=RDMA_InitializeSetupMT setup-id=5b0f6d4e-1c2a-4b7e-9a3d-0123456789ab provider=TCP mode=LIVE schema=002.6.0 timestamp=2026-01-02T03:04:05Z classification=U endpoints=1 [IPv4 198.51.100.42/24]
RECEIVED RDMA_InitializeSetup #2 type=RDMA_InitializeSetupMT setup-id=00000000-0000-0000-0000-000000000000 provider=VERBS mode=LIVE schema=002.6.0 timestamp=2026-06-30T23:59:59.5Z classification=U endpoints=2 [IPv6 fE80::aBcD, IPv4 198.51.100.42/24]
RECEIVED Log #3 type=LogMT severity=WARNING log-timestamp=2026-01-02T03:04:05Z mode=LIVE schema=002.6.0 timestamp=2026-01-02T03:04:05Z classification=U service-id=- subsystem-id=- components=0 capabilities=0
UNSUBSCRIBED sid=sub-2 message=Log
UNSUBSCRIBED sid=sub-1 message=RDMA_InitializeSetup
CLOSED received=3 rejected=1 callback-failures=0 RDMA_InitializeSetup=2/0 Log=1/1
```

On standard error: `REJECTED Log payload (CodecFailure: payload decoder
rejected message)`. That run exits `7`.

- `READY` is printed only after **both** SUB acknowledgements. The receive
  window (`--duration-seconds`) starts there.
- `#N` counts typed messages across both families. `--max-messages N` stops
  after N messages in total, across both families.
- Messages of one subscription are delivered in order. The two families are
  independent subscriptions and may interleave in any order (in this run, the
  second RDMA was delivered before the earlier-published Log).
- The Log summary prints only the `LogSeverityEnum` severity, the checked
  `MessageData`/header timestamps, the header mode, schema version and
  classification, the `ServiceID`/`SubsystemID` UUIDs (`-` when absent), and
  the number of component and capability IDs. It never prints `Label`,
  `Details`, `Data`, `ServiceUpTime`, descriptive labels or security caveats.
- `CLOSED … RDMA_InitializeSetup=R/J Log=R/J` gives received/rejected counts
  per message.

Partial setup is always cleaned up. If the Log SUB is rejected (for example,
the broker does not authorize `uci.log`), the client prints
`SUBSCRIBE REJECTED by server: Illegal-State message=Log` and explicitly UNSUBs
the RDMA subscription before closing. It exits `5` and never prints `READY`. If
the connection is lost during setup, an already-acknowledged SID cannot be
UNSUBed. The client reports `UNSUBSCRIBE SKIPPED … (session already failed;
released by connection close)`, closes, and exits `6`. Cancellation between
the two SUBs closes the one connection.

### Exit statuses

| Code | Meaning |
| --- | --- |
| 0 | Duration or `--max-messages` reached; UNSUB and close succeeded |
| 2 | Malformed or contradictory arguments; nothing was opened |
| 3 | Missing, modified or release-mismatched generated bindings |
| 4 | Connect failed: transport, TLS verification, missing `owp` negotiation |
| 5 | Server rejected INIT (`Unsupported-Schema`/`-Service`) or SUB (`Illegal-State`) |
| 6 | Session failed after subscribing (connection lost, protocol error, limits) |
| 7 | Completed, but payloads were rejected or the callback raised |
| 130 / 143 | SIGINT / SIGTERM; UNSUB and close still performed |

## 4. Using the typed API from Python

```python
import asyncio
from python_uci_client import Options, run_client

def on_rdma(value):  # an actual generated model.RDMA_InitializeSetupMT
    for endpoint in value.messagedata.endpoint:      # NetworkEndpointType Choice
        if endpoint.alternative == 'IPv4_Endpoint':
            print(endpoint.value.ipv4_address.value)   # IPv4_AddressType
        else:
            print(endpoint.value.ipv6_address.value)   # IPv6_AddressType

result = asyncio.run(run_client(
    Options(url='ws://127.0.0.1:9000', service_id='python-uci-client',
            schema_release='2.6', bindings='/path/to/rdma-2.6', duration=30),
    callback=on_rdma))
raise SystemExit(result.status)
```

`load_bindings()` verifies the manifest digests. It executes the verified
generated bytes as one private package and registers the generated codec
through `CodecRegistry`/`CodecBinding` for the OAM message QName
`RDMA_InitializeSetup`. It subscribes through the generated
`function_rdma.exchange_receive.subscribe` endpoint. Callbacks may be sync or
`async` and run in order outside the runtime receive path. A callback exception
is reported and counted, and reception continues.

### Distinguishing `LogMT` from `RDMA_InitializeSetupMT`

With `with_log=True` the same single-value callback receives both generated
types. Distinguish them by the **actual generated class**, from the loaded
bindings, never by parsing JSON or comparing class-name strings:

```python
import asyncio
from python_uci_client import Options, load_bindings, message_name, run_client

bindings = load_bindings('/path/to/rdma-log-2.6', '2.6', with_log=True)
model = bindings.model

def on_message(value):
    if isinstance(value, model.LogMT):                      # global message "Log"
        data = value.messagedata
        if data.severity in (model.LogSeverityEnum.ERROR, model.LogSeverityEnum.FATAL):
            print('log', data.severity.value, data.timestamp.value)
    elif isinstance(value, model.RDMA_InitializeSetupMT):   # "RDMA_InitializeSetup"
        print('rdma', value.messagedata.rdma_initializesetupid.uuid.value)
    assert message_name(value) in ('Log', 'RDMA_InitializeSetup')

result = asyncio.run(run_client(
    Options(url='ws://127.0.0.1:9000', service_id='python-uci-client', schema_release='2.6',
            bindings='/path/to/rdma-log-2.6', duration=30, with_log=True),
    bindings=bindings, callback=on_message))
raise SystemExit(result.status)
```

`message_name(value)` maps the generated class's `__schema_name__` back to the
wire global element (`LogMT` → `Log`). The payload class name is never used as
the OWP message name. `bindings.families` lists each selected generated
endpoint (`message`, `payload_name`, `topic`, `receive`, `payload_type`) in
subscription order. Both codecs are registered in one `CodecRegistry`, keyed by
global message, and each subscription decodes only its own message: a `Log`
document arriving on the RDMA SID is an envelope rejection and never reaches
the other decoder. `SessionResult` adds `received_by_message`,
`rejected_by_message`, `subscription_ids` and `unsubscribed_ids`.

Lifecycle and buffering:

- The application inbox is bounded (16 values by default, `buffer_size`
  1–4096). A full inbox back-pressures the runtime's own bounded
  per-subscription queue. Nothing is buffered without bound.
- Decoder or envelope rejections arrive as runtime events and are counted. The
  subscription stays active.
- On duration, limit, signal, cancellation or failure the client unsubscribes,
  closes the runtime (joining its receive and dispatch tasks), then ends the
  callback task. Buffered but undelivered values are discarded. No callback
  runs after teardown.
- With `--with-log` both subscriptions share that one bounded inbox (and each
  keeps the runtime's own bounded per-subscription queue). Every owned SID is
  UNSUBed, in reverse SUB order, before the runtime is closed.

## 5. WSS certificate configuration

`wss://` always uses Python's default verified context (`CERT_REQUIRED`,
hostname checking) through the runtime's existing TLS path. `--ca-file`
**adds** trust anchors, for example a private deployment CA, and never disables
verification. There is no "insecure" option.

```bash
"$PY" examples/python-uci-client/python_uci_client.py \
  --url wss://sleet.example.test:9443 --service-id python-uci-client \
  --schema-release 2.6 --bindings "$CLIENT_SCRATCH/rdma-2.6" \
  --ca-file /etc/pki/deployment-ca.pem
```

RDMA + Log over the same verified WSS connection (both topics authorized as in
section 3):

```bash
"$PY" examples/python-uci-client/python_uci_client.py \
  --url wss://sleet.example.test:9443 --service-id python-uci-client \
  --schema-release 2.5 --bindings "$CLIENT_SCRATCH/rdma-log-2.5" \
  --with-log --ca-file /etc/pki/deployment-ca.pem
```

Connect to the name in the certificate's subjectAltName. An untrusted chain, a
wrong hostname or IP, or a server that does not negotiate the `owp`
subprotocol fails with exit `4` before INIT is sent.

## 6. Tests

The suite needs both pinned roots, the release generator and the unchanged
pinned Sleet build (`e38f61d8ce0d75c8508434a52f2ed77c69cf6a3b`):

```bash
export AMS_GRA_UCI_2_5_ROOT="$UCI25" AMS_GRA_UCI_2_6_ROOT="$UCI26"
export AMS_GRA_SLEET_BIN=/path/to/clean/sleet/checkout/target/release/sleet
"$PY" -m unittest discover -s examples/python-uci-client/tests -v
```

| Suite | Covers |
| --- | --- |
| `test_cli.py` (13) | Arguments, exit codes, release selection, generated package loading, tamper/mismatch, generation helper; `--with-log` parsing, combined package identities, mode/release/tamper/missing-Log-codec/API refusal |
| `test_session.py` (10) | Typed delivery and order, invalid payloads, callback errors, INIT/SUB rejection, missing `owp`, duration/limit, connection loss, cancellation, bounded buffer, no stale callback, task cleanup |
| `test_multimessage.py` (12) | `--with-log` on one connection: interleaved typed delivery and per-subscription order, Log output, no cross-subscription decoding, Log decoder failure with recovery, per-family callback failure, rejected first/second SUB, connection loss and cancellation between SUBs, failure after both acks, shutdown with pending messages, combined `--max-messages` |
| `test_real_sleet.py` (4) | Real Sleet with UCI 2.5 and 2.6 independently, wrong-release/service/message rejection, SIGINT/SIGTERM |
| `test_real_sleet_with_log.py` (4) | Real Sleet `--with-log`, UCI 2.5 and 2.6: two SUB acks, interleaved independent RDMA and Log publication, broker vs decoder rejection, both UNSUBs, one broker connection; unauthorized `uci.log` releases RDMA; SIGTERM releases both |
| `test_wss.py` (5) | Verified WSS positive (both releases, RDMA-only and `--with-log`), untrusted CA, wrong hostname, missing `owp` over TLS |

The real-Sleet and WSS tests use an independent raw-`websockets` OWP
publisher/observer, not this application or `oms_lacal`. They publish edits of
the independent Task092 RDMA and PY-016 Log golden documents and assert
broker-side debug evidence (INIT, subscription registered/removed). In CI these
suites run through `python/qualification/sleet/test_python_uci_client.py` in
the existing pinned-Sleet lane.

## Known limitations

- Receives `RDMA_InitializeSetup` on `uci.rdma` and, with `--with-log`, `Log`
  on `uci.log`. No other message families, topics or subscription groups are
  selectable. It does not publish.
- The two `--with-log` families are independent subscriptions: their relative
  delivery order is not defined, only the order within each family.
- No automatic reconnect, retry, resubscription or replay. A lost connection
  ends the session with exit `6`.
- Closed-schema generation only. Unknown subtypes or extensions are rejected
  by the generated decoder.
- Pinned Sleet validates before routing, so most malformed documents are
  rejected at the broker and never reach the client. The decoder-rejection
  path is exercised with a payload the broker accepts (an impossible
  Gregorian date, for both RDMA and Log).
- Deployment authentication (beyond TLS server verification) and client
  certificates are not supported by the runtime and are not added here.
