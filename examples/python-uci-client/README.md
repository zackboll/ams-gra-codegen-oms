# Standalone Python UCI RDMA receiver

A command-line application that connects to a Sleet OMS endpoint, subscribes
to the authentic UCI global message **`RDMA_InitializeSetup`** and hands each
received message, decoded into the generated payload type
**`RDMA_InitializeSetupMT`**, to an application callback. The default callback
prints selected typed fields.

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
| `tests/` | CLI, session, real-Sleet and WSS qualification |

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
| `test_cli.py` (9) | Arguments, exit codes, release selection, generated package loading, tamper/mismatch, generation helper |
| `test_session.py` (10) | Typed delivery and order, invalid payloads, callback errors, INIT/SUB rejection, missing `owp`, duration/limit, connection loss, cancellation, bounded buffer, no stale callback, task cleanup |
| `test_real_sleet.py` (4) | Real Sleet with UCI 2.5 and 2.6 independently, wrong-release/service/message rejection, SIGINT/SIGTERM |
| `test_wss.py` (4) | Verified WSS positive (both releases), untrusted CA, wrong hostname, missing `owp` over TLS |

The real-Sleet and WSS tests use an independent raw-`websockets` OWP
publisher/observer, not this application or `oms_lacal`. They publish edits of
the independent Task092 golden documents and assert broker-side debug evidence
(INIT, subscription registered/removed). In CI these suites run through
`python/qualification/sleet/test_python_uci_client.py` in the existing
pinned-Sleet lane.

## Known limitations

- Receives one message family (`RDMA_InitializeSetup`) on one topic
  (`uci.rdma`) per process. It does not publish.
- No automatic reconnect, retry, resubscription or replay. A lost connection
  ends the session with exit `6`.
- Closed-schema generation only. Unknown subtypes or extensions are rejected
  by the generated decoder.
- Pinned Sleet validates before routing, so most malformed documents are
  rejected at the broker and never reach the client. The decoder-rejection
  path is exercised with a payload the broker accepts.
- Deployment authentication (beyond TLS server verification) and client
  certificates are not supported by the runtime and are not added here.
