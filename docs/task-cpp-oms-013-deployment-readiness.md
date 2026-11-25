# CPP-OMS-013 — Deployment and operational qualification

This runbook covers the standalone C++17 UCI receiver. It does not certify a
production site, change the client or broker, or add retry, reconnect,
resubscribe, replay, readiness notification, or a background health check.

## Select and stage an artifact

Choose an explicit pinned UCI release (`2.5` → INIT `002.5.0`, `2.6` →
`002.6.0`), one message profile, and the executable variant. There is no
runtime release switch or arbitrary message selection.

| Profile | Name suffix before optional `-tls` | Enabled families (topic) | Client flags for full profile |
| --- | --- | --- | --- |
| A | none | `Log` (`uci.log`) | none |
| B | `-log-rdma` | A + `RDMA_InitializeSetup` (`uci.rdma`) | `--with-rdma` |
| C | `-log-rdma-survivability` | B + `SurvivabilityRiskLevel` (`uci.survivability`) | `--with-rdma --with-survivability` |

The name starts with `cpp-uci-client-uci25` or `cpp-uci-client-uci26`.
Plain executables accept `ws://` only. A `-tls` executable accepts both
`ws://` and verified `wss://`; select WSS explicitly when encryption and
peer verification are required. B and C otherwise still default to Log.

Use the matching release checkout and an already-qualified `build.sh`
directory. See the [client README](../examples/cpp-uci-client/README.md)
for pinned input and build commands. Do not stage from a build being
written or from an untrusted publisher.

```sh
# From the release checkout. Set BUILD to its qualified external build directory.
scratch=$(mktemp -d)
bundle="$scratch/uci26-log-rdma-survivability-tls"
python3 examples/cpp-uci-client/deploy/stage.py \
  --release 2.6 --profile C --variant tls --build "$BUILD" --out "$bundle"
```

The parent must exist; staging creates a new destination exclusively and
does not rebuild, regenerate, or modify inputs. Transfer only after `STAGED`
is reported. If staging is interrupted by an uncatchable signal, discard any
partial destination. Repeat staging to another new path: file contents are
identical from the same build and deployment tools (not a claim about file
timestamps, tar bytes, or separate rebuilds).

Each bundle contains one executable in `bin/`, `check.py`, `README.txt`, `LICENSE`, and
`manifest/{build-manifest.txt,deployment-manifest.json,SHA256SUMS}`. It has no
source, schemas, generated headers, build caches, system libraries, test
payloads, credentials, certificates, or keys. The generated code and runtime
are already linked into the executable, which still uses system shared
libraries. The preserved build manifest has its original absolute binary
path and toolchain flags as historical metadata, not runtime inputs; review
them before distributing. The deployment manifest uses a relative binary
path and records the release, profile, TLS variant, schema and contract
fingerprints, generated header hashes, compiler/Boost/OpenSSL/flags, binary
hash, build manifest hash, and direct ELF dependencies.

Staging reads the build manifest as data with an exact field set, compares
both schema pins, checks the selected contract against the checkout, and
rehashes the three generated headers and binary. It also runs the same
offline checks as the deployment host. The original manifest is not signed:
hashes and CLI/ELF checks detect corruption and inconsistent selections, not
a party who can replace the binary and every manifest. There is no independent
proof of compiler provenance or header-to-binary binding beyond the trusted
build/qualification process. Verify authenticity through your release channel
before running the checker or the executable. Install to a directory not
writable by the service user; do not edit the manifests to silence a failure.

## Host preflight and dependencies

The qualified host is Debian 13.8 amd64 (Linux 6.12, glibc 2.41). Staging
and offline checking require Python 3.11+ and GNU `readelf` (`binutils`).
Running the client itself does not require Python, Boost, Cargo, a compiler,
schemas, or the source tree. This tool deliberately admits dynamic Linux
x86-64 glibc ELF with `/lib64/ld-linux-x86-64.so.2`, system library resolution,
and, for TLS, the OpenSSL 3 SONAMEs. Other architectures, static builds,
private library search paths and OpenSSL 1.1 builds are outside this deployment
contract even if the original build system can produce some of them.

| Qualified variant | Direct `DT_NEEDED` | Also resolved by the loader on Debian 13 |
| --- | --- | --- |
| Plain | `libstdc++.so.6`, `libgcc_s.so.1`, `libc.so.6`; no OpenSSL | `libm.so.6` |
| TLS | Plain + `libssl.so.3`, `libcrypto.so.3` | Plain + `libz.so.1`, `libzstd.so.1` |

Use distro packages `libc6`, `libstdc++6`, `libgcc-s1` and (TLS) `libssl3t64`
and its dependencies, plus current `ca-certificates` if public/system roots
are needed. SONAME presence alone is insufficient: the qualified build needs
symbol versions up to `GLIBC_2.38`, `GLIBCXX_3.4.32` and `CXXABI_1.3.15`.
Do not copy libraries from the build host; use compatible maintained distro
packages and test on the actual target. Build-only Boost ≥1.81 remains
required: this build used externally extracted Debian Boost 1.83 headers
(Beast 347) because the host's default headers were not sufficient.

```sh
# Run as the intended non-root service user, on the transferred bundle.
python3 /opt/uci-client/check.py --release 2.6 --profile C --variant tls /opt/uci-client
```

The read-only checker compares the binary hash, deployment metadata and all
listed checksums, inspects ELF, uses the known glibc loader's `--list` to
resolve direct and transitive libraries, and executes `--help` and selected
unsupported-argument probes without a URL. It uses a clean environment and
reports the observed library resolution. It neither evaluates shell input nor
installs anything. It does not test broker reachability, certificates, DNS,
site OpenSSL policy, or a different service environment's loader settings.

## Secure TLS configuration and startup

Choose the broker URL, authorized Service ID, receive duration (`1..3600`
seconds) and typed message limit (`1..100000`, total across enabled families).
Duration is a receive bound, not a deadline for startup plus teardown. Use an
absolute CA file when needed and omit `--ca-file` entirely if default trust
is sufficient. For example, from any working directory:

```sh
/opt/uci-client/bin/cpp-uci-client-uci26-log-rdma-survivability-tls \
  --url wss://broker.example.net:9443/owp --service-id cpp-uci-client \
  --ca-file /etc/uci-client/broker-ca.pem --with-rdma --with-survivability \
  --duration-seconds 60 --max-messages 100
```

The runtime uses OpenSSL's default system paths and optionally adds PEM CA
certificates from `--ca-file`. This is **additive**, not CA pinning or a
replacement trust store. `SSL_CERT_FILE` and `SSL_CERT_DIR` can change default
trust selection; review them and other site OpenSSL settings in the service
environment. Neither staging nor the smoke runner overrides runtime trust.
Provision the CA separately through the operator trust process. Use a
trusted administrator owner, a non-service-writable parent directory, and
read permission for the service (e.g. `root:uci-client` `0640`, or `0644` for
public CA data where permitted). Never use a private key or server leaf as
a CA file, and never install the qualification peer's CA in production.

TLS 1.2/1.3 verifies chain, validity, server purpose and original URL identity.
DNS names require a matching DNS SAN and are sent as SNI. Literal IP URLs
(bracket IPv6) require an IP-address SAN and do not send SNI. There is no CN
fallback, insecure option, client certificate, or plaintext downgrade.
`--ca-file` on WS or on a plaintext-only executable is invalid. Unreadable,
missing, malformed or otherwise invalid additional trust fails locally
before dialing. Do not fix a certificate problem by switching to `ws://`.

## Readiness, observation and shutdown

Startup is CONNECT (TCP and, for WSS, verified TLS), WebSocket `owp`
negotiation, OWP `INIT` / `+OK` / `INFO`, then one acknowledged `SUB` per
enabled family. There is no separate application readiness protocol.

| Marker | Meaning and operator action |
| --- | --- |
| `CPP-UCI-CLIENT built-for=…` | Check release, schema, enabled families and bounds. This is not connected. |
| `CONNECTED owp=1.0 schema=… server=…` | Upgrade/subprotocol and INIT/INFO succeeded (including TLS verification for WSS). |
| `READY sid=…` | Verify the full expected roster. Enabled SUBs were acknowledged and the connection checked, not guaranteed alive in the future. READY does not attest to delivery. |
| `RECEIVED <family> #N` | A generated decoder delivered a typed summary. Compare sequence/counts to independent publisher evidence if delivery matters. Treat summaries as operational data, not as a raw message archive. |
| `DECODE-ERROR` | Delivery was rejected by the decoder. Inspect schema, sender and category/path; rejected payloads do not count toward the typed bound. Can occur in a run that exits 0. |
| `CONNECT-FAILED`, `SUBSCRIBE-REJECTED`, `SUBSCRIBE-FAILED` | No full readiness; check URL, TLS, release, broker INIT and topic entitlement. A partial setup rolls back only acknowledged SIDs. |
| `CONNECTION-LOST`, `QUEUE-OVERFLOW` or other fatal diagnostics | Session failed; check broker logs, queue counters and workload. No automatic recovery or resubscription. |
| `STOP`, `UNSUBSCRIBED sid=… acknowledged` | Bound or signal observed; inspect reverse-order acknowledgments. `UNSUBSCRIBE-FAILED` is not an acknowledgment. |
| `CLOSED close=ok` / `close=failed` / `close=not-connected` | Inspect close result, per-family counters (`FAMILY` when enabled), queue discards and `resources-released`. For a clean connected run also check all UNSUBs and process status. |

Send SIGTERM or SIGINT to the actual executable, then wait for its status.
Allow time for the existing command and close timeouts; forced termination
does not prove cleanup. `runtime-threads=0`, sockets no higher than
`sockets-at-start`, and `resources-released=yes` describe local resource
cleanup, separately from successful UNSUB and graceful wire close. Counts
such as `pending-discarded`, `discarded-on-close`, `queue-rejected-full` and
`decode-errors` are useful for investigation, not end-to-end delivery proofs.

| Exit | Outcome | Operator action |
| --- | --- | --- |
| 0 | Completion with clean shutdown under the selected transport policy | Check STOP reason and typed counts; a duration bound can expire with zero messages. |
| 1 | Unexpected internal failure | Preserve diagnostic/version information and escalate; do not blindly retry. |
| 2 | Invalid local arguments/configuration | Check profile switches, URL, Service ID grammar, bounds and CA file; no broker contact is expected. |
| 3 | TCP/TLS/WebSocket/`owp`/INIT/INFO failure | Inspect `CONNECT-FAILED`: distinguish certificate verification, handshake, protocol and broker schema/identity errors. Check DNS, clock, chain/SAN and broker logs. |
| 4 | Broker rejected a SUB | Inspect entitlement and topic/message binding; check rollback of earlier SIDs. |
| 5 | Session failure (including setup timeout after connect, loss or overflow) | Inspect connection/events and load; compare publisher and receiver evidence before restarting. |
| 6 | Otherwise successful run, but UNSUB, graceful close or cleanup failed | Alert; separate wire close, subscription acknowledgments, local resources and delivery evidence. Not success. Does not by itself imply message loss. |
| 130 | SIGINT after clean shutdown | Expected only if intentionally signaled; confirm teardown. |
| 143 | SIGTERM after clean shutdown | Expected only if intentionally signaled; confirm teardown. |

Earlier primary failures retain precedence even if shutdown also fails.
For plaintext the existing WebSocket close policy is best effort; for live
WSS it requires reciprocal WebSocket Close and TLS `close_notify`.

### Pinned Sleet limitation

The unchanged Sleet pin `e38f61d8ce0d75c8508434a52f2ed77c69cf6a3b`
(tree `7e705f4eb5f658957bd414b4abc8207311f3c1e4`) serves plaintext WS. A
test-only byte-transparent TLS terminator was used to qualify WSS with it.
That Sleet pin sends EOF without reciprocal WebSocket Close. Record the
following separately for the tested WSS case:

1. **TRANSPORT VERIFIED** — trusted matching TLS peer and `owp` accepted.
2. **SUBSCRIPTIONS ACKNOWLEDGED** — INIT/INFO and full SUB roster passed.
3. **MESSAGES RECEIVED** — typed summaries and independent wire observation.
4. **RESOURCE CLEANUP COMPLETE** — reverse UNSUB `+OK`, zero runtime
   threads and baseline socket count.
5. **GRACEFUL SHUTDOWN FAILED** — missing server Close, `CLOSED close=failed`,
   actual application exit **6**. Never rewrite it to 0.

The independent peer completes both handshakes and exits cleanly. A
production broker/terminator must be qualified for the same behavior; don't
inject Close or change client shutdown policy to mask this limitation. A
broker fix is a separate upstream task.

### Debian/systemd example (guidance only)

Provision the `uci-client` user/group and trusted bundle and CA externally.
Verify as that user before starting. This example is a bounded receiver run,
not a daemon with an automatic restart policy.

```ini
[Unit]
Description=Standalone C++ UCI 2.6 profile C receiver
Wants=network-online.target
After=network-online.target

[Service]
Type=exec
User=uci-client
Group=uci-client
WorkingDirectory=/
ExecStartPre=/usr/bin/python3 /opt/uci-client/check.py --release 2.6 --profile C --variant tls /opt/uci-client
ExecStart=/opt/uci-client/bin/cpp-uci-client-uci26-log-rdma-survivability-tls --url wss://broker.example.net:9443/owp --service-id cpp-uci-client --ca-file /etc/uci-client/broker-ca.pem --with-rdma --with-survivability --duration-seconds 60 --max-messages 100
StandardOutput=journal
StandardError=journal
KillSignal=SIGTERM
TimeoutStopSec=60s
SuccessExitStatus=130 143
Restart=no
NoNewPrivileges=yes
```

Inspect the journal and `ExecMainStatus`; `Type=exec` means exec succeeded,
not READY. Only regard 130/143 as intentional when correlated with the
operator action. Adjust the stop allowance after testing your broker; a
forced kill is not clean. Do not use `Restart=always` for this bounded CLI.
Any site restart policy must distinguish expected stops, local configuration
errors, transient failures and persistent exit 6 to avoid restart loops.
This task installed/enabled no system service.

## Opt-in smoke and reproduction

Use only an explicitly authorized **test** broker/peer. Configure matching
UCI release and topic permissions; arrange a separate publisher to send at
least one valid message per enabled family *after* READY. Use enough duration
and message budget to see every family. The command runs the staged binary,
shows its logs, requires full READY and typed reception, and checks reverse
UNSUBs, clean close, resources and exit 0. A valid message-less run can exit
0 in the client but is not a passed smoke. Client exit 6 remains 6.

```sh
python3 /absolute/release/examples/cpp-uci-client/deploy/smoke.py \
  --release 2.6 --profile C --variant tls /absolute/bundle \
  --test-url wss://localhost:9443/owp --service-id cpp-uci-client \
  --ca-file /absolute/test-ca.pem --duration-seconds 20 --max-messages 6
```

For automated reproduction, rebuild the 12 combinations (both releases,
A/B/C, plain/TLS) as in the client README, adding `--with-tls` for TLS only.
Use [CPP-OMS-012](task-cpp-oms-012-verified-wss-client.md#reproduce-qualification)
to prepare the raw publisher/observer and TLS receiver-core harness in
`$BUILD`. Set absolute paths for `BUILD`, `CLIENT` (the example directory),
both `AMS_GRA_UCI_2_{5,6}_ROOT` variables, and an unchanged pinned
`SLEET_CHECKOUT` with its release binary built. Stage into the expected
test layout under a new external scratch directory (Bash below):

```bash
s=$(mktemp -d)
mkdir "$s/bundles" "$s/empty-cwd"
find "$BUILD/bin" "$BUILD/generated" -type f -print0 | sort -z | xargs -0 sha256sum >"$s/before"
for r in 2.5 2.6; do
  for p in A B C; do
    for t in plain tls; do
      a=uci${r/./}
      case $p in B) a+=-log-rdma;; C) a+=-log-rdma-survivability;; esac
      [[ $t == plain ]] || a+=-tls
      b="$s/bundles/$a"
      python3 "$CLIENT/deploy/stage.py" --build "$BUILD" --out "$b" --release "$r" --profile "$p" --variant "$t"
      cmp "$b/bin/cpp-uci-client-$a" "$BUILD/bin/cpp-uci-client-$a"
      (cd "$s/empty-cwd" && env -i PATH=/usr/bin:/bin LC_ALL=C python3 "$b/check.py" --release "$r" --profile "$p" --variant "$t" "$b")
    done
  done
done
sha256sum -c "$s/before"
python3 -B "$CLIENT/tests/check_deployment.py" --build "$BUILD" --scratch "$s" --missing-libs
(cd "$s/empty-cwd" && env -i PATH=/usr/bin:/bin LC_ALL=C python3 -B "$CLIENT/tests/check_verified_wss.py" \
  --bundles "$s/bundles" --build "$BUILD" --out "$s/qualification" \
  --sleet "$SLEET_CHECKOUT" --wire "$BUILD/wire_peer" \
  --schema25 "$AMS_GRA_UCI_2_5_ROOT" --schema26 "$AMS_GRA_UCI_2_6_ROOT")
```

The shared harness starts loopback-only test endpoints. The client sees a
separate scratch CWD, `LC_ALL=C`, explicit empty OpenSSL default paths and
an additional test CA; default/additive trust cases explicitly set the
default CA file. The operator smoke is also exercised with a literal-IP SAN
and a clean close, and with missing reciprocal Close. To skip Sleet, pass
only `--bundles` and `--out` to the harness. The `--missing-libs` control
requires Debian amd64, bubblewrap and non-root user/mount namespaces; it
hides each SONAME independently without modifying the installed libraries.
Omitting the switch explicitly skips that control.

## Qualification and remaining risk

- Regressions: [008](task-cpp-oms-008-standalone-log-client.md#4-real-sleet-qualification),
  [009](task-cpp-oms-009-multimessage-client.md#reproduce-the-focused-campaign),
  [010](task-cpp-oms-010-survivability-client.md#reproduce-the-focused-campaign)
  ran against pinned Sleet for both releases; [011](task-cpp-oms-011-native-wss.md#qualification)
  ran the offline runtime peer and plaintext regressions with GCC 14.2 and
  Clang 19.1.7. The staged 012 matrix supplies the real-broker WSS evidence
  rather than repeating the full native runtime campaign.
- Both releases and all A/B/C profiles: independent verified WSS received
  2/4/6 typed summaries with clean exit 0 (TLS 1.2 and 1.3). Pinned Sleet
  plaintext on both executable variants gave the same typed summaries;
  broker WSS matched them, including C's three decoder rejections from
  nine admitted messages, but always exited 6 on incomplete close. The
  independent observer verified messages byte-for-byte. Reverse UNSUBs and
  local cleanup were observed on both clean and incomplete-close paths.
- Local controls rejected changed/missing binaries, manifest/checksum drift,
  unsupported release, wrong profile including rehashed swaps, wrong TLS
  linkage, missing libraries, generated header drift, and existing output.
  CLI/CA failures exited 2 with no connection observed; TLS chain, SAN,
  validity, purpose and failed upgrade checks exited 3. SUB rollback (4),
  lost session (5), clean signals (130/143), and earlier failure precedence
  over incomplete close were exercised. Status 1 is documented but no
  artificial internal failure was injected into the staged executable.
- Local qualification used a non-root UID, GCC 14.2.0, Boost 1.83,
  OpenSSL 3.5.9, binutils 2.44, Python 3.13.5, Rust/Cargo 1.98.1 for codegen,
  and Rust 1.95.0 for pinned Sleet. The bundle checks found no build or
  generated-file changes. No CI workflow or production source was changed.
- Operational risk remains: unsigned checksums, platform/ABI and site trust
  differences, certificate rotation, entitlement or schema mismatches,
  bounded queue pressure, no recovery/replay, and the Sleet close limitation.
  Requalify target hosts and the actual endpoint; local results are not a
  production certification or delivery guarantee.