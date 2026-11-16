# PY-OMS-020 Standalone Python UCI RDMA receiver

`examples/python-uci-client/` is a usable command-line application, not a
transport framework. It receives the authentic UCI global message
`RDMA_InitializeSetup` from a Sleet endpoint and delivers each message as the
generated payload type `RDMA_InitializeSetupMT` to an application callback.
See the [application README](../examples/python-uci-client/README.md) for
setup, generation, CLI usage, expected output and WSS configuration.

## Composition

| Concern | Owner |
| --- | --- |
| WebSocket/TLS, `owp` negotiation, INIT/INFO, SUB/UNSUB, envelope, bounded dispatch | Existing `oms_lacal` runtime (unchanged) |
| UCI model, strict OMS JSON codec, service endpoint | Production `service-generate --language python --world closed-schema --with-codec` output |
| Release pinning, external generation, digest manifest | `generate_bindings.py` |
| CLI, callback delivery, bounded application inbox, lifecycle, exit status | `python_uci_client.py` |

The contracts are receive-only derivatives of `tests/fixtures/task103`: the same
global message and generated payload type, topic `uci.rdma`. Both releases
produce one selected message and 50 selected types with the codec.

| Release | Pinned root SHA-256 | INIT schema |
| --- | --- | --- |
| UCI 2.5 (`093610b7753944059360d3236770ab446d039556`) | `ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27` | `002.5.0` |
| UCI 2.6 (`78eb61b6112c8bffa40820c33124b57787fc5bd9`) | `af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b` | `002.6.0` |

No generated UCI source or XSD is committed. The generated files are byte-for-byte
identical to a direct `service-generate` run (tested). The client refuses
bindings whose manifest release differs from `--schema-release`, or whose
files differ from the recorded digests.

## Qualification

`python/qualification/sleet/test_python_uci_client.py` joins the existing
source-discovered pinned-Sleet lane. That lane already provides the shared
broker build, both UCI roots, the generator and the installed runtime on Python
3.11/3.13, normal and `-O`. It runs the four application suites in fresh
interpreters and requires the exact test counts with no skips.

- `test_cli.py`: argument/exit-code matrix with a socket guard proving
  rejection before any network access; release selection; actual generated
  package loading and identity; tamper and mismatch refusal; generation helper
  refusals.
- `test_session.py`: local OWP peer plus the real runtime and generated code.
  Covers typed ordered delivery, decoder/envelope rejection with recovery,
  callback failure isolation, INIT/SUB rejection, missing `owp`, duration and
  limit expiry, connection loss, cancellation, bounded back-pressure, no stale
  callback, and no leaked tasks.
- `test_real_sleet.py`: unchanged Sleet `e38f61d8…` (tree `7e705f4e…`),
  separately for 2.5 and 2.6, with the actual CLI process. An independent
  raw-websocket publisher and observer performs authorized PUBs and witnesses
  each MSG. The test asserts exact typed output lines (IPv4 with prefix, mixed
  IPv6/IPv4 order, empty endpoints, fractional timestamp, nil UUID), broker
  `Invalid-Message` and `Illegal-State` rejections, and one generated-decoder
  rejection of an impossible date the broker accepts. It checks broker
  INIT/registration/removal logs, a single UNSUB, and no CLI output for
  post-teardown publication. It also covers wrong release, unknown service,
  unauthorized message and SIGINT/SIGTERM.
- `test_wss.py`: reuses the Rust client's TLS terminator. Verified WSS
  reception for both releases with `--ca-file`. Rejects an untrusted leaf, a
  rogue-only CA, isolated system trust, a wrong DNS name, an IP mismatch and
  missing `owp` over TLS. All rejections happen before broker INIT.

The runtime, generator, IR, frontend, Sleet and other language implementations
are unchanged. No runtime defect was found. No CI workflow, lane, required
check, retry, reconnect or TCP-reset classification is added. OMS Issue #212
remains deferred.
