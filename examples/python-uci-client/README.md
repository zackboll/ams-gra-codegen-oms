# Standalone Python UCI RDMA (and optional Log and SurvivabilityRiskLevel) receiver

A command-line application that connects to a Sleet OMS endpoint, subscribes
to the authentic UCI global message **`RDMA_InitializeSetup`** and hands each
received message, decoded into the generated payload type
**`RDMA_InitializeSetupMT`**, to an application callback. The default callback
prints selected typed fields.

With **`--with-log`** (PY-OMS-021) the same single authenticated connection
also subscribes to the UCI global message **`Log`** and delivers generated
**`LogMT`** values to the same callback. Without it, behavior, output and exit
statuses are exactly those of the RDMA-only receiver (PY-OMS-020).

With **`--with-log --with-survivability`** (PY-OMS-022) the same connection
additionally subscribes to the UCI global message **`SurvivabilityRiskLevel`**
and delivers generated **`SurvivabilityRiskLevelMT`** values. There are exactly
three modes; no other message selection exists:

| Mode | Flags | Messages | Bindings manifest |
| --- | --- | --- | --- |
| A | *(none)* | `RDMA_InitializeSetup` | format 1 |
| B | `--with-log` | `RDMA_InitializeSetup`, `Log` | format 2 |
| C | `--with-log --with-survivability` | `RDMA_InitializeSetup`, `Log`, `SurvivabilityRiskLevel` | format 3 |

`--with-survivability` without `--with-log` is a usage error (exit `2`,
reported before any network activity). Modes A and B are unchanged.

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
| `pyproject.toml` | Installable wheel `ams-gra-oms-python-uci-client` with the `python-uci-client` command (PY-OMS-023, [section 7](#7-installed-deployment-python-uci-client-command)) |
| `generate_bindings.py` | Generates one release into external build storage |
| `contracts/uci25-service.yaml`, `contracts/uci26-service.yaml` | Receive-only RDMA contracts derived from `tests/fixtures/task103` |
| `contracts/uci25-rdma-log-service.yaml`, `contracts/uci26-rdma-log-service.yaml` | Receive-only RDMA + Log contracts (`--with-log`); Log as in `tests/fixtures/task102` |
| `contracts/uci25-rdma-log-survivability-service.yaml`, `contracts/uci26-rdma-log-survivability-service.yaml` | Receive-only RDMA + Log + SurvivabilityRiskLevel contracts (`--with-log --with-survivability`); RDMA and Log functions verbatim from the RDMA + Log contracts, SurvivabilityRiskLevel as in `tests/fixtures/task104`/`task105` |
| `tests/` | CLI, session, multimessage, three-message, real-Sleet and WSS qualification |

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

### RDMA + Log + SurvivabilityRiskLevel bindings (`--with-log --with-survivability`)

```bash
"$PY" examples/python-uci-client/generate_bindings.py --with-log --with-survivability \
  --schema-release 2.5 --schema "$UCI25" --output "$CLIENT_SCRATCH/rdma-log-surv-2.5"
"$PY" examples/python-uci-client/generate_bindings.py --with-log --with-survivability \
  --schema-release 2.6 --schema "$UCI26" --output "$CLIENT_SCRATCH/rdma-log-surv-2.6"
```

One `model.py`, one `service_codec.py` and one `service_api.py` then contain all
three messages (production `service-check` reports model, service API and
codec `READY` for both releases):

| Global message (OWP wire name) | Generated payload type | Topic | Generated endpoint |
| --- | --- | --- | --- |
| `RDMA_InitializeSetup` | `model.RDMA_InitializeSetupMT` | `uci.rdma` | `function_rdma.exchange_receive` |
| `Log` | `model.LogMT` | `uci.log` | `function_log.exchange_receive` |
| `SurvivabilityRiskLevel` | `model.SurvivabilityRiskLevelMT` | `uci.survivability` | `function_survivability.exchange_receive` |

`SurvivabilityRiskLevelMT` is only the generated payload class. The OWP
`SUB`/`MSG` global element, and the codec-registry key, is always
`SurvivabilityRiskLevel`.

The three-message manifest has `"format": 3`, `"with_log": true`,
`"with_survivability": true` and a `messages` list naming the three global
messages, payload types and topics in subscription order. It also carries the
same pin, contract, generator and per-file digests. Format 1 and format 2
generation is unchanged and byte-identical to PY-OMS-020/021 output. The
client fails closed with exit `3` on:

- a manifest format that does not match the selected flags (including three-message
  bindings used without both flags, or older bindings used with
  `--with-survivability`);
- a release mismatch, a modified or missing file, or a reordered, renamed or
  truncated message inventory;
- generated sources lacking the Survivability endpoint or the
  `SurvivabilityRiskLevelMT` codec.

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
| `--with-survivability` | With `--with-log` only: also receive `SurvivabilityRiskLevel` on `uci.survivability`; needs `--with-log --with-survivability` bindings |
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

### RDMA + Log + SurvivabilityRiskLevel on one connection (`--with-log --with-survivability`)

The broker must authorize all three topics for the service identity:

```toml
service_id = "python-uci-client"
service_uuid = "550e8400-e29b-41d4-a716-446655440020"
allowed_topics = ["uci.rdma", "uci.log", "uci.survivability"]
[[topic_bindings]]
topic = "uci.rdma"
allowed_messages = ["RDMA_InitializeSetup"]
[[topic_bindings]]
topic = "uci.log"
allowed_messages = ["Log"]
[[topic_bindings]]
topic = "uci.survivability"
allowed_messages = ["SurvivabilityRiskLevel"]
```

```bash
"$PY" examples/python-uci-client/python_uci_client.py \
  --url ws://127.0.0.1:9000 --service-id python-uci-client \
  --schema-release 2.5 --bindings "$CLIENT_SCRATCH/rdma-log-surv-2.5" \
  --with-log --with-survivability --duration-seconds 30
"$PY" examples/python-uci-client/python_uci_client.py \
  --url ws://127.0.0.1:9000 --service-id python-uci-client \
  --schema-release 2.6 --bindings "$CLIENT_SCRATCH/rdma-log-surv-2.6" \
  --with-log --with-survivability --duration-seconds 30
```

This is the actual UCI 2.5 output on real pinned Sleet. An independent
publisher sent RDMA, SurvivabilityRiskLevel, Log, then a
SurvivabilityRiskLevel whose date the broker accepts but the generated
`SurvivabilityRiskLevelMT` decoder rejects. UCI 2.6 output is identical
except for `schema=002.6.0`:

```text
CONNECTED ws://127.0.0.1:9000 owp=1.0 service-id=python-uci-client schema=002.5.0
SUBSCRIBED RDMA_InitializeSetup topic=uci.rdma sid=sub-1 payload=RDMA_InitializeSetupMT
SUBSCRIBED Log topic=uci.log sid=sub-2 payload=LogMT
SUBSCRIBED SurvivabilityRiskLevel topic=uci.survivability sid=sub-3 payload=SurvivabilityRiskLevelMT
READY subscriptions=3 messages=RDMA_InitializeSetup,Log,SurvivabilityRiskLevel duration=2s
RECEIVED RDMA_InitializeSetup #1 type=RDMA_InitializeSetupMT setup-id=5b0f6d4e-1c2a-4b7e-9a3d-0123456789ab provider=TCP mode=LIVE schema=002.5.0 timestamp=2026-01-02T03:04:05Z classification=U endpoints=2 [IPv4 192.0.2.10, IPv6 2001:db8::1]
RECEIVED SurvivabilityRiskLevel #2 type=SurvivabilityRiskLevelMT risk-level-id=12345678-1234-1234-8234-123456789abc mode=LIVE schema=002.5.0 timestamp=2026-01-02T03:04:05Z classification=U risk-settings=2 [HIGH:NONE, LOW:TRACK+KILL]
RECEIVED Log #3 type=LogMT severity=WARNING log-timestamp=2026-01-02T03:04:05Z mode=LIVE schema=002.5.0 timestamp=2026-01-02T03:04:05Z classification=U service-id=- subsystem-id=- components=0 capabilities=0
UNSUBSCRIBED sid=sub-3 message=SurvivabilityRiskLevel
UNSUBSCRIBED sid=sub-2 message=Log
UNSUBSCRIBED sid=sub-1 message=RDMA_InitializeSetup
CLOSED received=3 rejected=1 callback-failures=0 RDMA_InitializeSetup=1/0 Log=1/0 SurvivabilityRiskLevel=1/1
```

On standard error: `REJECTED SurvivabilityRiskLevel payload (CodecFailure:
payload decoder rejected message)`. That run exits `7`.

- SUBs are sent strictly in order: RDMA, Log, then SurvivabilityRiskLevel.
  `READY` is printed only after **all three** acknowledgements, and only while
  the session is still healthy.
- `#N` and `--max-messages N` count typed messages across all three families.
- Order is preserved within each subscription. Different subscriptions are
  independent, so the client makes no cross-family ordering guarantee.
- The SurvivabilityRiskLevel summary prints only:
  - the `SurvivabilityRiskLevelID` UUID;
  - the header mode, schema version and checked timestamp;
  - the classification;
  - the number of `RiskSetting` entries;
  - for each entry, the `RiskPrioritizationEnum` value and its
    `VulnerabilityTypeEnum` `AllowedRisk` values.

  The schema bounds `RiskSetting` to 1–4 and `RiskLevel` to 1–5. The display
  is bounded independently (at most 4 settings and 5 risks each, then
  `+N more`). The summary never prints descriptive labels,
  `MaximumContinuousExposure` durations, `AppliesToSystemID` identities or
  security caveats.
- RDMA and Log summary lines are byte-identical to those in modes A and B.
- `CLOSED … SurvivabilityRiskLevel=R/J` gives received/rejected counts for
  each of the three messages.

Partial setup is cleaned up at every position:

| Failure | Result |
| --- | --- |
| A SUB is rejected (e.g. `uci.survivability` not authorized) | The client prints `SUBSCRIBE REJECTED by server: Illegal-State message=SurvivabilityRiskLevel`. Earlier SIDs are explicitly UNSUBed in reverse order (Log, then RDMA) before close. Exit `5`, never `READY` |
| Connection loss during the third SUB | Neither earlier SID can be UNSUBed. Each is reported as `UNSUBSCRIBE SKIPPED` and released by connection close. The client never claims an UNSUB the dead transport could not perform. Exit `6` |
| Cancellation between any two SUBs | The one connection is closed |
| Failure right after the final acknowledgement | Never `READY`. Exit `6` |

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

### Handling `SurvivabilityRiskLevelMT` (three-message mode)

With `with_log=True, with_survivability=True` the callback also receives
generated `SurvivabilityRiskLevelMT` values:

```python
import asyncio
from python_uci_client import Options, load_bindings, message_name, run_client

bindings = load_bindings('/path/to/rdma-log-surv-2.5', '2.5', with_log=True,
                         with_survivability=True)
model = bindings.model

def on_message(value):
    if isinstance(value, model.SurvivabilityRiskLevelMT):   # "SurvivabilityRiskLevel"
        data = value.messagedata
        for setting in data.risksetting:                    # SurvivabilityRiskSettingType, 1..4
            if setting.riskprioritization is model.RiskPrioritizationEnum.HIGH:
                print('high', [level.allowedrisk.value for level in setting.risklevel])
    elif isinstance(value, model.LogMT):                    # "Log"
        print('log', value.messagedata.severity.value)
    elif isinstance(value, model.RDMA_InitializeSetupMT):   # "RDMA_InitializeSetup"
        print('rdma', value.messagedata.rdma_initializesetupid.uuid.value)
    assert message_name(value) in ('RDMA_InitializeSetup', 'Log', 'SurvivabilityRiskLevel')

result = asyncio.run(run_client(
    Options(url='ws://127.0.0.1:9000', service_id='python-uci-client', schema_release='2.5',
            bindings='/path/to/rdma-log-surv-2.5', duration=30,
            with_log=True, with_survivability=True),
    bindings=bindings, callback=on_message))
raise SystemExit(result.status)
```

The third `CodecBinding` is registered under the global message
`SurvivabilityRiskLevel`. It uses the generated
`encode_SurvivabilityRiskLevelMT`/`decode_SurvivabilityRiskLevelMT` and
subscribes through the generated
`function_survivability.exchange_receive.subscribe` endpoint. The three
subscriptions share the one connection, runtime, INIT/INFO exchange and
bounded application inbox. Each SID is owned separately: a document of
another family arriving on the Survivability SID (or the reverse) is an
envelope rejection that never reaches the wrong generated decoder. A callback
failure for one family is counted against that family only.

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

RDMA + Log + SurvivabilityRiskLevel over the same verified WSS connection (all
three topics authorized as in section 3), for UCI 2.5 or 2.6:

```bash
"$PY" examples/python-uci-client/python_uci_client.py \
  --url wss://sleet.example.test:9443 --service-id python-uci-client \
  --schema-release 2.5 --bindings "$CLIENT_SCRATCH/rdma-log-surv-2.5" \
  --with-log --with-survivability --ca-file /etc/pki/deployment-ca.pem
"$PY" examples/python-uci-client/python_uci_client.py \
  --url wss://sleet.example.test:9443 --service-id python-uci-client \
  --schema-release 2.6 --bindings "$CLIENT_SCRATCH/rdma-log-surv-2.6" \
  --with-log --with-survivability --ca-file /etc/pki/deployment-ca.pem
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
| `test_survivability_cli.py` (6) | `--with-survivability` parsing and the `--with-log` requirement (CLI and generator, before any network), format-3 package identities (wire name vs payload class, `function_survivability.exchange_receive`, registry keyed by global message), bounded and safe Survivability summary, mode/release mismatch, missing/altered/re-signed/relabelled bindings |
| `test_survivability.py` (11) | Three subscriptions on one connection: interleaved typed delivery, per-subscription order, exact output, cross-SID and payload-name envelope rejection, Survivability decoder failure with recovery, per-family callback failure, rejected first/second/third SUB, connection loss during the third SUB, cancellation between each SUB pair, failure after the final ack, shutdown with queued data of all three families, combined `--max-messages`, mode/bindings mismatch |
| `test_real_sleet_with_survivability.py` (4) | Real Sleet three-message mode, UCI 2.5 and 2.6: three SUB acks and distinct SIDs, interleaved independent publication of all three families witnessed on the wire and at the typed callback, broker vs decoder rejection, three UNSUBs, one broker connection, no delivery after teardown; unauthorized `uci.survivability` releases RDMA and Log; SIGTERM releases all three |
| `test_wss_survivability.py` (2) | Verified WSS three-message reception for both releases; untrusted CA and wrong hostname still rejected |
| `test_installed_cli.py` (8) | PY-OMS-023: wheel contents and metadata, offline install into a fresh environment, import origin, import audit (nothing from the checkout), installed `--help` and usage errors, all six deployed binding sets (formats 1/2/3, UCI 2.5/2.6), modified/wrong-release/wrong-mode refusal, `PYTHONOPTIMIZE=1` |
| `test_installed_real_sleet.py` (6) | PY-OMS-023: the **installed** `python-uci-client` against real Sleet: three-message mode for UCI 2.5 and 2.6, modes A and B, unauthorized third SUB (exit `5`), SIGTERM (exit `143`), verified WSS with a hostname-mismatch control |

The real-Sleet and WSS tests use an independent raw-`websockets` OWP
publisher/observer, not this application or `oms_lacal`. They publish edits of
the independent Task092 RDMA and PY-016 Log and SurvivabilityRiskLevel golden
documents and assert
broker-side debug evidence (INIT, subscription registered/removed). In CI these
suites run through `python/qualification/sleet/test_python_uci_client.py` in
the existing pinned-Sleet lane.

## 7. Installed deployment (`python-uci-client` command)

PY-OMS-023 packages the same `python_uci_client.py`, unmodified, as the wheel
**`ams-gra-oms-python-uci-client`**. The wheel provides the console command
**`python-uci-client`**, which calls `python_uci_client:main`. The installed
command has the same three modes, flags, output, exit statuses, typed
callback API and TLS verification as the source file. Generated code stays
outside the wheel: a deployed client always takes previously generated
bindings through `--bindings`.

| Needed on the build machine | Needed on the deployment machine |
| --- | --- |
| This checkout, Rust/Cargo, the generator, the pinned UCI XSDs, Python ≥ 3.11 with `pip` | Python ≥ 3.11, the three wheels, the generated binding directories, network access to Sleet (and a CA file for private `wss://`) |

The deployment machine needs **no** checkout, Rust, Cargo, generator or
UCI XSD.

### A. Developer/source-checkout usage

Unchanged. The source file still runs directly with an interpreter that has
the runtime installed (section 1):

```bash
"$PY" examples/python-uci-client/python_uci_client.py --help
```

### B. Building a client wheel

From the repository root, with any Python ≥ 3.11 that has `pip`. `pip wheel`
uses the setuptools backend declared in each `pyproject.toml`; it fetches
setuptools into an isolated build environment unless one is already
available. Write all output outside the repository:

```bash
export WHEELHOUSE="$CLIENT_SCRATCH/wheelhouse"
cp -r python/oms-lacal-runtime examples/python-uci-client "$CLIENT_SCRATCH/"
"$PY" -m pip wheel --no-deps --wheel-dir "$WHEELHOUSE" \
  "$CLIENT_SCRATCH/oms-lacal-runtime" "$CLIENT_SCRATCH/python-uci-client"
"$PY" -m pip download --no-deps --only-binary=:all: --dest "$WHEELHOUSE" websockets==17.2
ls "$WHEELHOUSE"
# ams_gra_oms_lacal_runtime-0.1.0-py3-none-any.whl
# ams_gra_oms_python_uci_client-0.1.0-py3-none-any.whl
# websockets-17.2-<platform tags>.whl
```

`websockets==17.2` is the runtime's own pin. `websockets` publishes
platform-specific wheels, so download it for the deployment platform; for
example add `--platform`/`--python-version` when building on another machine.

The application wheel contains exactly `python_uci_client.py` and its
`.dist-info` metadata (`METADATA`, `WHEEL`, `entry_points.txt`,
`top_level.txt`, `RECORD`). It contains no generated `model.py`,
`service_api.py` or `service_codec.py`, no contracts, no schemas, no
`generate_bindings.py`, no tests and no certificates. Check it with:

```bash
"$PY" -m zipfile -l "$WHEELHOUSE"/ams_gra_oms_python_uci_client-0.1.0-py3-none-any.whl
```

The package declares `Private :: Do Not Upload`, so PyPI refuses it. It is
distributed only as a locally built wheel. Like the runtime, it is not
published on any index.

The copies keep setuptools' `build/` and `*.egg-info` output out of the
checkout. An in-place build works too, and those directories are
git-ignored.

### C. Installing the runtime and client wheels

On the deployment machine, into a fresh virtual environment, strictly from
the wheelhouse (no index, no network):

```bash
python3.11 -m venv /opt/python-uci-client
/opt/python-uci-client/bin/python -m pip install --no-index --find-links /srv/wheelhouse \
  ams-gra-oms-python-uci-client==0.1.0
/opt/python-uci-client/bin/python-uci-client --help
```

pip resolves `ams-gra-oms-lacal-runtime==0.1.0` and `websockets==17.2` from
the same wheelhouse. Nothing else is installed.

### D. Preparing generated bindings on the build machine

Generation remains a build-time step on the machine with the checkout,
generator and pinned schemas (section 2). For example, UCI 2.6 three-message
bindings:

```bash
"$PY" examples/python-uci-client/generate_bindings.py --with-log --with-survivability \
  --schema-release 2.6 --schema "$UCI26" --output "$CLIENT_SCRATCH/rdma-log-surv-2.6"
```

Each output directory holds `model.py`, `service_api.py`, `service_codec.py`
and `python-uci-client-build.json`. Generate one directory per release and
mode you deploy (format 1, 2 or 3).

### E. Deploying those bindings to another machine

Copy each output directory **as is**, for example with `rsync -a` or a tar
archive, to any path on the deployment machine. The directory does not need
to be importable or on `PYTHONPATH`. Only these four files are needed, and no
schemas. The client verifies the manifest's release, mode, message inventory
and the SHA-256 of all three generated files before executing them. It
refuses, with exit `3`:

- a modified or missing file;
- a release that differs from `--schema-release`;
- bindings of another mode.

Regenerate on the build machine instead of editing a deployed copy.

### F. Running the installed console command

`python-uci-client` takes exactly the arguments of section 3. Mode A:

```bash
python-uci-client --url ws://sleet.example.test:9000 --service-id python-uci-client \
  --schema-release 2.5 --bindings /srv/uci-bindings/rdma-2.5
```

Mode B:

```bash
python-uci-client --url ws://sleet.example.test:9000 --service-id python-uci-client \
  --schema-release 2.6 --bindings /srv/uci-bindings/rdma-log-2.6 --with-log
```

Mode C:

```bash
python-uci-client --url ws://sleet.example.test:9000 --service-id python-uci-client \
  --schema-release 2.6 --bindings /srv/uci-bindings/rdma-log-surv-2.6 \
  --with-log --with-survivability --duration-seconds 60
```

Use the absolute path (`/opt/python-uci-client/bin/python-uci-client`) or
activate the environment. Output and exit statuses (`0`, `2`–`7`, `130`,
`143`) are identical to the source-file usage. Setting `PYTHONOPTIMIZE=1` is
supported and qualified.

The typed API remains importable from the installed environment:
`import python_uci_client` and then `python_uci_client.run_client(...)`, as in
section 4.

### G. Verified WSS configuration

Identical to section 5. Verification cannot be disabled; `--ca-file` only
adds trust anchors:

```bash
python-uci-client --url wss://sleet.example.test:9443 --service-id python-uci-client \
  --schema-release 2.6 --bindings /srv/uci-bindings/rdma-log-surv-2.6 \
  --with-log --with-survivability --ca-file /etc/pki/deployment-ca.pem
```

Distribute the CA certificate through your normal configuration management.
Never ship private keys or credentials with the wheel or the bindings.

### Complete example: checkout → wheel → installed client outside the checkout

```bash
# --- Build machine, inside the checkout ---------------------------------
cd /path/to/ams-gra-codegen-oms
export CLIENT_SCRATCH="$(mktemp -d)"            # all output outside the repository
python3.11 -m venv "$CLIENT_SCRATCH/build-venv"
export PY="$CLIENT_SCRATCH/build-venv/bin/python"
cargo build --release --locked -p ams-gra-codegen-oms
UCI26="$(scripts/fetch-pinned-uci-2.6.sh "$CLIENT_SCRATCH/uci-2.6")"
"$PY" examples/python-uci-client/generate_bindings.py --with-log --with-survivability \
  --schema-release 2.6 --schema "$UCI26" --output "$CLIENT_SCRATCH/deploy/rdma-log-surv-2.6"
# Build from a copy so setuptools' build/ and *.egg-info stay out of the checkout.
cp -r python/oms-lacal-runtime examples/python-uci-client "$CLIENT_SCRATCH/"
"$PY" -m pip wheel --no-deps --wheel-dir "$CLIENT_SCRATCH/deploy/wheelhouse" \
  "$CLIENT_SCRATCH/oms-lacal-runtime" "$CLIENT_SCRATCH/python-uci-client"
"$PY" -m pip download --no-deps --only-binary=:all: \
  --dest "$CLIENT_SCRATCH/deploy/wheelhouse" websockets==17.2
tar -C "$CLIENT_SCRATCH" -czf "$CLIENT_SCRATCH/python-uci-client-deploy.tgz" deploy

# --- Deployment machine (or any directory outside the checkout) ---------
mkdir -p /srv/python-uci-client && cd /srv/python-uci-client
tar -xzf python-uci-client-deploy.tgz           # deploy/wheelhouse, deploy/rdma-log-surv-2.6
python3.11 -m venv venv                         # same Python version as the websockets wheel
venv/bin/python -m pip install --no-index --find-links deploy/wheelhouse \
  ams-gra-oms-python-uci-client==0.1.0
venv/bin/python-uci-client --url ws://sleet.example.test:9000 --service-id python-uci-client \
  --schema-release 2.6 --bindings deploy/rdma-log-surv-2.6 \
  --with-log --with-survivability --duration-seconds 30
```

### H. Known limitations of the installed deployment

- The wheel holds no generated code, so the deployment must ship bindings
  that match its release and mode. There is no schema discovery and no
  on-target generation.
- `websockets` wheels are platform- and Python-version-specific
  (`cp311`, `cp313`, …). Download the one matching the deployment
  interpreter, or add `--python-version`/`--platform` to `pip download`.
- Not published to PyPI or any index; the version is fixed at `0.1.0` like
  the runtime's.
- The command name and behavior are the source CLI's. There is no new
  configuration file, service mode, reconnect, retry or replay.
- `python-uci-client` only installs into an environment with Python ≥ 3.11.
  Qualified on 3.11 and 3.13, normal and `PYTHONOPTIMIZE=1`.


- Receives `RDMA_InitializeSetup` on `uci.rdma`, with `--with-log` also `Log`
  on `uci.log`, and with `--with-log --with-survivability` also
  `SurvivabilityRiskLevel` on `uci.survivability`. These three fixed modes are
  the only selections: no arbitrary or dynamic message selection, other
  topics, or subscription groups. It does not publish.
- The families are independent subscriptions: their relative delivery order is
  not defined (no cross-subscription broker ordering is assumed), only the
  order within each family.
- The SurvivabilityRiskLevel summary deliberately omits `AppliesToSystemID`,
  `MaximumContinuousExposure` and descriptive labels; callbacks receive the
  full typed value.
- No automatic reconnect, retry, resubscription or replay. A lost connection
  ends the session with exit `6`.
- Closed-schema generation only. Unknown subtypes or extensions are rejected
  by the generated decoder.
- Pinned Sleet validates before routing, so most malformed documents are
  rejected at the broker and never reach the client. The decoder-rejection
  path is exercised with a payload the broker accepts (an impossible
  Gregorian date, for RDMA, Log and SurvivabilityRiskLevel).
- Deployment authentication (beyond TLS server verification) and client
  certificates are not supported by the runtime and are not added here.
- The installed wheel is not published to any package index. See section 7H
  for the deployment-specific limitations.
