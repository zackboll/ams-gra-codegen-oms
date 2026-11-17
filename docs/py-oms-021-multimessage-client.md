# PY-OMS-021 Optional Log reception in the standalone Python UCI client

PY-OMS-021 extends the PY-OMS-020 receiver
(`examples/python-uci-client/`, [PY-OMS-020](py-oms-020-standalone-rdma-client.md))
with an opt-in `--with-log` mode. One authenticated WebSocket connection, one
OMS runtime, and one OWP INIT carry two generated subscriptions:

| Global message | Generated payload | Topic | Generated endpoint |
| --- | --- | --- | --- |
| `RDMA_InitializeSetup` | `RDMA_InitializeSetupMT` | `uci.rdma` | `function_rdma.exchange_receive` |
| `Log` | `LogMT` | `uci.log` | `function_log.exchange_receive` |

Without `--with-log`, behavior is unchanged: RDMA-only generation, the format-1
manifest, CLI defaults, output lines, exit statuses, WSS trust handling, and
cleanup all stay the same. The [README](../examples/python-uci-client/README.md)
covers usage, Sleet topic permissions and the typed callback API.

## Readiness preflight

Production `service-check --language python --world closed-schema --with-codec`
on the combined contracts reports `model READY`, `service api READY` and
`codec READY` for both releases. UCI 2.5 has 2 selected messages and a
55-type closure. UCI 2.6 has 2 selected messages and a 57-type closure. No
backend expansion was needed.

## Composition

| Concern | Owner |
| --- | --- |
| Connection, INIT/INFO, SUB/UNSUB, per-SID bounded dispatch, envelope | Existing `oms_lacal` runtime (unchanged) |
| Combined model/codec/API | Production `service-generate` on `contracts/uci2{5,6}-rdma-log-service.yaml` |
| Combined manifest (format 2) and mode checks | `generate_bindings.py --with-log`, `load_bindings(..., with_log=True)` |
| Sequential SUBs, READY, per-family accounting, owned-SID teardown, Log summary | `python_uci_client.py` |

Both `CodecBinding`s are registered in one `CodecRegistry`. Each is keyed by
the global message (`Log`), never by the payload class name (`LogMT`). Each SUB
goes through its own generated endpoint, so each SID unwraps and decodes only
its own global element. Both subscriptions feed the existing bounded
application inbox. Each subscription keeps the runtime's own bounded queue, so
delivery stays ordered within a subscription. `--max-messages` counts both
families together.

SUBs are strictly sequential. `READY` is printed only after both
acknowledgements and only while the runtime is still connected. Teardown
UNSUBs every owned SID in reverse order, then closes the runtime and joins the
callback task. If a SID is no longer active because the session already
failed, it cannot be UNSUBed. That SID is reported, and the connection close
releases it at the broker.

## Qualification

The existing source-discovered pinned-Sleet lane
(`python/qualification/sleet/test_python_uci_client.py`) now generates four
binding sets per interpreter: 2.5, 2.6, 2.5 `--with-log` and 2.6 `--with-log`.
It requires exact counts for six suites, with no skips:

- `test_cli.py` (13). The 9 PY-OMS-020 tests are unchanged. 4 new tests cover:
  - `--with-log` parsing;
  - combined package identities (wire name vs payload class, registry keyed by
    global message);
  - refusal of mode, release, tamper and manifest mismatches;
  - re-signed sources missing the `LogMT` codec or the Log endpoint;
  - RDMA-only sources relabeled as combined.
- `test_session.py` (10): unchanged PY-OMS-020 tests.
- `test_multimessage.py` (12): local OWP peer with the actual generated
  combined package. Covers:
  - interleaved typed delivery and per-SID order;
  - exact Log output, with no free text printed;
  - cross-SID envelope rejection;
  - Log decoder rejection followed by recovery;
  - a per-family callback failure;
  - a rejected first or second SUB;
  - connection loss and cancellation between SUBs;
  - failure after both acknowledgements;
  - shutdown with pending messages of both families;
  - combined `--max-messages`;
  - mode/bindings mismatch;
  - leaked tasks.
- `test_real_sleet.py` (4): unchanged PY-OMS-020 controls.
- `test_real_sleet_with_log.py` (4): unchanged pinned Sleet `e38f61d8…`,
  separately for 2.5 and 2.6. The broker authorizes `uci.rdma` and `uci.log`.
  An independent raw-websocket publisher interleaves RDMA (Task092 goldens) and
  Log (PY-016 goldens), and an independent observer witnesses both MSG
  families. The test asserts:
  - broker `Invalid-Message` for a non-enumerated Log severity, with no MSG
    witnessed;
  - broker `Illegal-State` for RDMA on `uci.log`;
  - generated-decoder rejection of an impossible Log date, which the broker
    accepts and the observer witnesses;
  - continued RDMA reception after that rejection;
  - broker evidence of one CLI INIT, two registrations and two removals on one
    connection, and exactly two UNSUBs;
  - no output after teardown, and a clean broker exit.

  It also covers an unauthorized `uci.log`: the RDMA SID is explicitly UNSUBed
  and Log is never registered. A SIGTERM releases both SIDs.
- `test_wss.py` (5): the 4 PY-OMS-020 tests are unchanged. One new test covers
  verified-WSS `--with-log` reception for each release, with the same
  assertions.

For RDMA-only mode, the new `generate_bindings.py` output was compared
byte-for-byte (`diff -r`, manifest included) with the merged PY-OMS-020 helper
for both releases. The combined output was compared with a direct
`service-generate` run. Both comparisons were manual verification and are not
automated tests.

The runtime, generator, IR, frontend, Sleet, workflows and other language
implementations are unchanged. No runtime or generator defect was found. This
change adds no reconnect, retry, replay, open-extensions support, generic
subscription mechanism or new authentication.
