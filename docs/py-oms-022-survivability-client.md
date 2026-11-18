# PY-OMS-022 Optional SurvivabilityRiskLevel reception in the standalone Python UCI client

PY-OMS-022 extends the PY-OMS-021 receiver
(`examples/python-uci-client/`, [PY-OMS-021](py-oms-021-multimessage-client.md))
with one explicit three-message mode, `--with-log --with-survivability`. One
authenticated WebSocket connection, one OMS runtime, one OWP INIT/INFO and one
bounded application inbox carry three generated subscriptions:

| Global message | Generated payload | Topic | Generated endpoint |
| --- | --- | --- | --- |
| `RDMA_InitializeSetup` | `RDMA_InitializeSetupMT` | `uci.rdma` | `function_rdma.exchange_receive` |
| `Log` | `LogMT` | `uci.log` | `function_log.exchange_receive` |
| `SurvivabilityRiskLevel` | `SurvivabilityRiskLevelMT` | `uci.survivability` | `function_survivability.exchange_receive` |

There are exactly three modes. `--with-survivability` without `--with-log` is
a usage error (exit `2`), raised before any network activity.

| Mode | Flags | Manifest |
| --- | --- | --- |
| A | *(none)* | format 1 (PY-OMS-020, unchanged) |
| B | `--with-log` | format 2 (PY-OMS-021, unchanged) |
| C | `--with-log --with-survivability` | format 3 (new) |

Modes A and B keep their defaults, flags, output lines, exit statuses, WSS
trust handling and cleanup unchanged. The
[README](../examples/python-uci-client/README.md) covers usage, Sleet topic
permissions, the typed callback API and the safe displayed fields.

## Readiness preflight

Production `service-check --language python --world closed-schema
--with-codec` was run on the new contracts
`contracts/uci2{5,6}-rdma-log-survivability-service.yaml`. For both releases
it reports `model READY`, `service api READY` and `codec READY`, with 3
selected messages. UCI 2.5 has a 64-type closure and UCI 2.6 a 65-type
closure. No backend expansion was needed.

The new contracts copy the existing RDMA + Log contract text verbatim (checked
with `diff`) and add one receive-only `survivability` function using the same
global message as `tests/fixtures/task104` and `task105`. They add no
publication endpoints. The old contracts are unchanged.

## Composition

| Concern | Owner |
| --- | --- |
| Connection, INIT/INFO, SUB/UNSUB, per-SID bounded dispatch, envelope | Existing `oms_lacal` runtime (unchanged) |
| Three-message model/codec/API | Production `service-generate` on the new contracts |
| Format-3 manifest and mode checks | `generate_bindings.py --with-log --with-survivability`, `load_bindings(..., with_log=True, with_survivability=True)` |
| Sequential SUBs, READY, per-family accounting, owned-SID teardown, Survivability summary | `python_uci_client.py` |

The third `CodecBinding` is keyed by the wire message `SurvivabilityRiskLevel`
and uses the generated `model.SurvivabilityRiskLevelMT`,
`encode_SurvivabilityRiskLevelMT` and `decode_SurvivabilityRiskLevelMT`. It is
never keyed by the payload class name. The client subscribes in a fixed
order: RDMA, Log, then SurvivabilityRiskLevel. `READY` is printed only after
all three acknowledgements, and only while the runtime is still connected.
Teardown UNSUBs every owned active SID in reverse order, closes the runtime,
then joins the callback task. A SID left inactive by a failed transport is
reported as `UNSUBSCRIBE SKIPPED`, never as a successful UNSUB.

The format-3 manifest records `format: 3`, `with_log: true`,
`with_survivability: true` and the exact ordered `messages` inventory. The
loader rejects any of these with exit `3`:

- a format/flag mismatch in either direction;
- `with_survivability` on a format 1 or 2 manifest;
- a missing or false marker;
- a reordered, truncated, renamed or retopiced inventory;
- the wrong release;
- re-signed sources lacking the Survivability endpoint or codec;
- format-2 sources relabelled as format 3, and the reverse.

## Generation compatibility

Each run used the same generator binary (SHA-256 `959318ba…`, the binary used
for the PY-OMS-021 qualification). For both releases:

- format-1 output (generated files and manifest) is `diff -r` identical to
  the merged PY-OMS-020 helper (`02393a0`) and the merged PY-OMS-021 helper
  (`5ba835e`);
- format-2 output is identical to the merged PY-OMS-021 helper;
- format-3 generated files are identical to a direct production
  `service-generate` run on the new contract.

These comparisons were run manually. They are not automated tests. All
output is written outside the repository, and the working tree stayed clean.

## Qualification

The existing source-discovered pinned-Sleet lane
(`python/qualification/sleet/test_python_uci_client.py`) now generates six
binding sets per interpreter: 2.5 and 2.6 in each of the three modes. It
requires exact counts for ten suites, with no skips. The six existing suites
and their counts (13/10/4/5/12/4) are unchanged. Four new suites:

- `test_survivability_cli.py` (6) covers:
  - parsing and the `--with-log` requirement (CLI and generator);
  - format-3 identities and the `function_survivability.exchange_receive`
    endpoint;
  - a registry keyed by the global message;
  - the bounded summary, which omits labels, exposures, system IDs and caveats;
  - mismatch, missing, altered, re-signed and relabelled bindings.
- `test_survivability.py` (11) uses a local OWP peer with the actual generated
  three-message package. It covers:
  - interleaved typed delivery and per-SID order;
  - exact output, including RDMA/Log lines byte-identical to PY-OMS-021;
  - cross-SID and payload-class-name envelope rejection;
  - Survivability decoder rejection with recovery of all families;
  - a per-family callback failure;
  - a rejected first, second or third SUB, with earlier SIDs UNSUBed in
    reverse order;
  - connection loss during the third SUB (`UNSUBSCRIBE SKIPPED`, no false
    UNSUB);
  - cancellation between each SUB pair;
  - failure right after the final acknowledgement;
  - shutdown with queued data of all three families (bounded, no stale
    callback);
  - combined `--max-messages`, mode mismatch and leaked tasks.
- `test_real_sleet_with_survivability.py` (4) runs unchanged pinned Sleet
  `e38f61d8…` separately for 2.5 and 2.6, authorizing `uci.rdma`, `uci.log`
  and `uci.survivability`. An independent raw-websocket publisher interleaves
  RDMA (Task092), Log and SurvivabilityRiskLevel (PY-016 goldens). Each
  deliverable publication waits for the CLI's previous typed callback line,
  so arrival is interleaved by construction. An independent observer
  witnesses all ten MSGs. The test asserts:
  - broker `Invalid-Message` for a non-enumerated `RiskPrioritization`, with
    no MSG witnessed;
  - broker `Illegal-State` for RDMA on `uci.survivability`;
  - generated-decoder rejection of an impossible date, which the broker
    accepts and the observer witnesses;
  - three distinct SIDs, registered on one broker connection after one CLI
    INIT;
  - no `message_name=SurvivabilityRiskLevelMT` in broker events;
  - exactly three UNSUBs and removals;
  - no delivery after teardown, and a clean broker exit.

  It also covers an unauthorized `uci.survivability`: RDMA and Log are
  explicitly UNSUBed, and Survivability is never registered. A SIGTERM
  releases all three SIDs.
- `test_wss_survivability.py` (2) covers verified-WSS three-message reception
  for both releases. In three-message mode it confirms that an untrusted CA
  and a wrong hostname are still rejected before INIT. The existing
  `test_wss.py` TLS negative controls are unchanged.

The runtime, generator, IR, frontend, Sleet, workflows, dependency pins and
other language implementations are unchanged. No runtime or generator defect
was found. This change adds no reconnect, retry, TCP-reset recovery, replay,
open-extensions support, generic or dynamic subscription mechanism, or new
authentication. OMS Issue #212 stays deferred.
