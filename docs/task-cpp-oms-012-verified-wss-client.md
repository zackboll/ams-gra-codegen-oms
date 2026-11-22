# CPP-OMS-012 — verified WSS in the standalone C++17 UCI receiver

The application selects the transport with `--url` and passes the optional
`--ca-file` directly to `ams_gra::lacal::Config::tls_ca_file`. The single
unmodified [CPP-OMS-011 runtime](task-cpp-oms-011-native-wss.md) owns DNS, TCP,
verified TLS, WebSocket, exact `owp` negotiation, INIT/INFO, subscriptions,
callbacks and shutdown. No SSL context or verification API is used by the
application. There is no new thread, hand-written codec, retry, reconnect,
automatic resubscription or replay.

## Build and run

Follow the [client README](../examples/cpp-uci-client/README.md#verified-wss---with-tls)
for the six build commands and pinned schema prerequisites. All outputs are
external. The TLS variant adds `-tls` to the profile name for the binary,
generated directory, object directory, generation log and manifest. It
uses the same strict C++17 `-O1 -pthread` policy and generated headers as
the corresponding plaintext profile.

| Build switches (besides `--with-tls`) | UCI 2.5 binary | UCI 2.6 binary | Application switches to enable the full built profile |
| --- | --- | --- | --- |
| none | `cpp-uci-client-uci25-tls` | `cpp-uci-client-uci26-tls` | none (Log) |
| `--with-rdma` | `cpp-uci-client-uci25-log-rdma-tls` | `cpp-uci-client-uci26-log-rdma-tls` | `--with-rdma` |
| `--with-rdma --with-survivability` | `cpp-uci-client-uci25-log-rdma-survivability-tls` | `cpp-uci-client-uci26-log-rdma-survivability-tls` | `--with-rdma --with-survivability` |

With no application switches, any binary receives only Log. Profile C can
also run with `--with-rdma` alone. The TLS-enabled variants accept both
`ws://` and `wss://`. Remove `--with-tls` when building the original
plaintext-only variants; they have no direct OpenSSL dependency and reject
`wss://` and `--ca-file` before opening a connection.

```bash
"$CLIENT_SCRATCH/build/bin/cpp-uci-client-uci26-log-rdma-survivability-tls" \
  --url wss://localhost:9443/owp \
  --service-id cpp-uci-client \
  --ca-file /path/to/trusted-ca.pem \
  --with-rdma \
  --with-survivability \
  --duration-seconds 30
```

On a TLS build the client and runtime are compiled with
`-DAMS_GRA_LACAL_ENABLE_TLS=1` and linked with `-lssl -lcrypto` after the
objects. Requires OpenSSL development headers and libraries ≥ 1.1.1
(use a maintained release), plus the existing Boost.Beast/Asio headers.
Missing TLS build dependencies are an error, not a fallback.

## Trust, rollback and exit status

Default OpenSSL system trust paths are used even with an additional CA
file. A private CA bundle must contain readable PEM CA certificates, not
a leaf or a key. Omit the option for a broker trusted through the default
paths. TLS 1.2/1.3 requires a valid chain and server purpose, with a SAN
matching the original URL (DNS + SNI, or an IP SAN without SNI). No CN
fallback, client certificate, verification bypass or plaintext downgrade.

Invalid local arguments (including an invalid additional CA file) exit 2.
Both `tls_verification_failed` and `tls_handshake_failed` exit 3, as do TCP,
timeout, upgrade, wrong subprotocol and INIT failures. A failed TLS handshake
sends no WebSocket upgrade, INIT or SUB and cannot invoke a callback. No
failed startup can emit `CONNECTED` or `READY`.

Setup retains only acknowledged SIDs. A SUB denial exits 4 and rolls back
earlier SIDs in reverse order. Transport loss or other session failure exits
5; dead transport never produces a false UNSUB acknowledgment. Both signals
request the same teardown; after clean close, SIGINT exits 130 and SIGTERM
exits 143. A live WSS connection requires reciprocal WebSocket Close and TLS
`close_notify`. If either fails, an otherwise successful session exits 6
even after releasing every socket and thread. Earlier failures retain
precedence. Plaintext retains its existing best-effort close policy.

## Reproduce qualification

From the repository root, set `CLIENT_SCRATCH` to a fresh external directory,
set both pinned `AMS_GRA_UCI_2_{5,6}_ROOT` paths, and set `SLEET_CHECKOUT` to the
clean pinned checkout with `target/release/sleet` already built. First build
all six TLS and (for `--plain`) all six plaintext variants in the same
`$CLIENT_SCRATCH/build` using the README commands. Then compile the independent
publisher/observer and receiver-core test against the existing objects:

```bash
set -e
client=examples/cpp-uci-client
build="$CLIENT_SCRATCH/build"
cxx=${CXX:-g++-14}
strict=(-std=c++17 -Wall -Wextra -Wpedantic -Werror -Wconversion -Wsign-conversion
        -Wshadow -Wold-style-cast -Wcast-align -Wnon-virtual-dtor -Woverloaded-virtual
        -Wnull-dereference -Wdouble-promotion -Wformat=2 -O1 -pthread)
inc=(-I cpp/oms-lacal-runtime/include -I "$client/src" -I tests/cpp-lacal)
extra=()
if [[ -n ${CPP_UCI_CLIENT_EXTRA_CXXFLAGS:-} ]]; then
  read -r -a extra <<< "$CPP_UCI_CLIENT_EXTRA_CXXFLAGS"
fi
"$cxx" "${strict[@]}" "${inc[@]}" "$client/tests/wire_peer.cpp" -o "$build/wire_peer"
for v in 25 26; do
  schema=$AMS_GRA_UCI_2_5_ROOT
  if [[ $v == 26 ]]; then schema=$AMS_GRA_UCI_2_6_ROOT; fi
  hash=$(sha256sum "$schema" | cut -d' ' -f1)
  profile="uci$v-log-rdma-survivability-tls"
  "$cxx" "${strict[@]}" "${extra[@]}" "${inc[@]}" -I "$build/generated/$profile" \
    -DAMS_GRA_LACAL_ENABLE_TLS=1 -DCPP_UCI_CLIENT_WITH_RDMA=1 \
    -DCPP_UCI_CLIENT_WITH_SURVIVABILITY=1 -DCPP_UCI_CLIENT_UCI_RELEASE="$v" \
    "-DCPP_UCI_CLIENT_SCHEMA_SHA256=\"$hash\"" "$client/tests/survivability_test.cpp" \
    "$build/obj/$profile/owp.o" "$build/obj/$profile/runtime.o" \
    -lssl -lcrypto -o "$build/survivability-$v-tls"
done
python3 "$client/tests/check_verified_wss.py" --build "$build" --plain "$build" \
  --sleet "$SLEET_CHECKOUT" --wire "$build/wire_peer" \
  --schema25 "$AMS_GRA_UCI_2_5_ROOT" --schema26 "$AMS_GRA_UCI_2_6_ROOT" \
  --out "$CLIENT_SCRATCH/evidence"
```

To run just the independent peer matrix, pass `--build` and `--out` only
(`--plain` is optional); no Sleet or core-test executable is needed. Use a
fresh scratch directory for sanitizer campaigns. Pass
`-fsanitize=address,undefined -fno-omit-frame-pointer -fno-sanitize-recover=all`
or `-fsanitize=thread` through `CPP_UCI_CLIENT_EXTRA_CXXFLAGS` for both builds
and receiver-core, and enable `ASAN_OPTIONS=detect_leaks=1` for ASan.
OpenSSL and pinned Sleet are not instrumented by this procedure.

## What the qualification proves

The independent peer reuses CPP-OMS-011's CA/leaf controls, TLS context and
the existing raw WebSocket peer. It requires exact INIT/SUB/UNSUB, checks
SNI and host/target, and exchanges both Close and `close_notify`. Its UCI
inputs are authentic Task093, Task092/103 and Task104/105 fixtures decoded
by the production-generated types/codecs; this peer does not establish
broker interoperability. Its negative cases independently require a TLS
handshake alert without application bytes for bad certificates, and no INIT
or SUB for failed upgrades. Local arguments and malformed CA bundles are
checked with a listening socket that must never be dialed. Default and
additive trust controls use process-local default OpenSSL paths; they do not
modify host trust stores.

Actual broker interoperability is tested against unchanged Sleet
`e38f61d8ce0d75c8508434a52f2ed77c69cf6a3b` (tree
`7e705f4eb5f658957bd414b4abc8207311f3c1e4`). The independent raw publisher
and observer connect directly over plaintext and check admitted MSGs byte
for byte; the test terminator only decrypts and forwards transport bytes
for the C++ client. No OWP reply, MSG or WebSocket Close is manufactured by
that relay. Mode C expects six typed summaries and three decoder rejections
from nine observed broker MSGs, with two summaries in per-family order,
one INIT/INFO, three SUB acknowledgments and three reverse UNSUB
acknowledgments. A deterministic receiver-core test uses the same production
client with its existing extractor and capacity seams to check the shared
queue, three pending discards and a contained callback exception.

The broker sends EOF without reciprocal WebSocket Close. The WSS
application receives and cleans up but reports `close=failed` and **exit 6**.
Only the independent peer qualifies complete graceful WSS shutdown and
**exit 0**. These are separate outcomes. The WS comparison exercises both
TLS-enabled and plaintext-only binaries with the same inputs and expected
summaries, callbacks and decoder counts.

## Observed qualification

GCC 14.2.0 and Clang 19.1.7 passed the strict C++17 `-O1` builds and both
release matrices using Boost 1.83.0 and OpenSSL 3.5.9. `readelf -d` showed
direct `libssl.so.3` and `libcrypto.so.3` dependencies in the TLS executables
and neither in the plaintext executables. All three generated headers
matched the pre-edit baseline byte for byte in every profile; building TLS
into the same output roots left all 96 original header/object/binary/manifest
hashes intact. The rebuilt plaintext help also matched. Rebuilt executable
hashes changed because of the new CLI and CA configuration; TLS executables
add the runtime's TLS code and linkage.

The CPP-OMS-008/009/010 plaintext qualifications and CPP-OMS-011 runtime
WSS campaign passed. CPP-OMS-008's old subprotocol diagnostic expectation
was updated to match the runtime already present at the baseline (the
negative exit status and cleanup checks were unchanged). The 009/010 cases
also passed with TLS-enabled executables and receiver cores on plaintext.
Both the independent-peer and pinned-Sleet client matrices passed under
Clang ASan/UBSan with leak detection, and under Clang TSan. Instrumentation
covered the application and C++ runtime, not OpenSSL or Sleet.