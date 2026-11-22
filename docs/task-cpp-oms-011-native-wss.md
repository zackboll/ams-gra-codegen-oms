# CPP-OMS-011 — native verified WSS for the C++17 LA-CAL runtime

## Build and scope

Use the same `cpp/oms-lacal-runtime/src/{runtime,owp}.cpp` and `include/`
directory as CPP-OMS-003. Add `-DAMS_GRA_LACAL_ENABLE_TLS=1` when compiling
the runtime and `-lssl -lcrypto` when linking it. Both builds use `-std=c++17`
and `-pthread`, and neither links a Boost library. Boost 1.83+ and a maintained
OpenSSL release with TLS 1.3 support are recommended. On Ubuntu 24.04 the
dependencies are `libboost-dev` and, for TLS builds, `libssl-dev`. The smoke
peer also needs Python 3 and the `openssl` command.

| Runtime build | `ws://` (port 80 by default) | `wss://` (port 443 by default) |
| --- | --- | --- |
| Default (macro absent or zero) | Plain WebSocket; no OpenSSL link | `unsupported_scheme`, no connection |
| TLS enabled | Same plain WebSocket; no TLS context created | Verified native TLS followed by WebSocket |

The standalone C++ UCI client remains **`ws://` only** pending separate CLI
and build integration. No generated model, service API, codec, other language
runtime, or broker code is changed.

## Trust and identity

The I/O thread initializes an Asio SSL client context, loads OpenSSL's default
verify paths, requires a peer certificate, and configures TLS 1.2–1.3 with
anonymous and NULL ciphers excluded. OpenSSL verifies the chain, validity
period, certificate purpose and issuer constraints. `Config::tls_ca_file`
optionally adds CA certificates to that same store. It never replaces the
default paths or disables verification. This file must be a readable, nonempty
regular file (max 16 MiB) consisting of `CERTIFICATE` PEM blocks separated by
whitespace; other objects, malformed blocks and non-CA certificates are
rejected before dialing. The default verify paths honor the usual OpenSSL
environment and system configuration; deployment must control those and the
contents of any explicitly supplied CA file.

URL authority, explicit port, path and query are carried into the WebSocket
request. For WSS DNS names the original ASCII DNS hostname is used for SNI
and OpenSSL DNS SAN verification (no CN fallback or partial wildcards). For
IPv4 and bracketed IPv6, OpenSSL verifies an IP SAN and no SNI is sent. The
resolved endpoint, reverse DNS, and HTTP response are not identity inputs.
Use A-labels for IDNs; scoped IPv6 and trailing-dot DNS names are not accepted
for WSS. There is no insecure mode, redirect following, plaintext fallback,
client cert/key, custom certificate callback, explicit CRL or OCSP checking,
or application authentication added by this feature.

## Ownership, deadlines, and errors

The private I/O state owns one typed Beast stream, either plain or over an
Asio SSL stream. The SSL context is configured before stream construction
and outlives it. Both variants use the same dispatcher, frame parser, command
slot, buffers, subscription IDs, acknowledgment pairing, queues and counters.
The write completion owns its frame; shutdown discards only unsent frames.

`DNS -> TCP -> [verified TLS] -> WebSocket owp -> INIT -> +OK -> INFO` uses
one absolute `connect_timeout` deadline. A failed WSS startup aborts TCP and
drains cancellation callbacks before releasing the SSL objects; it does not
start a second graceful-close budget. The API is synchronous, so callers
can't close an unpublished Runtime during startup. The timer stops connection
progress at expiry, but Asio cannot interrupt an already-running system
`getaddrinfo`; returning from cleanup may wait for the resolver worker to
exit. Literal addresses skip the resolver. Similarly `close()` joins the
dispatcher, which still must wait for an already running user handler.

| Error | Meaning |
| --- | --- |
| `invalid_argument` | Bad configuration, including a bad CA bundle or a CA file on `ws://` |
| `unsupported_scheme` | Unsupported URL scheme, including WSS in a plaintext build |
| `connect_failed` | DNS/TCP error or transport lost during WebSocket upgrade or OWP initialization |
| `tls_verification_failed` | Certificate chain, validity, server purpose or requested identity failed |
| `tls_handshake_failed` | TLS negotiation or transport error before a successful TLS handshake |
| `handshake_rejected` | HTTP upgrade refused/invalid (invalid upgrade on WSS), or other than exactly one `owp` selection |
| `broker_rejected` / `protocol_violation` | INIT `-ERR`, or invalid INIT/INFO sequence or payload |
| `timeout` | Deadline, including the shared startup or shutdown deadline |

`close_timeout` bounds write drain + WebSocket Close + TLS teardown. The
runtime never starts a WebSocket Close while an application write owns the
write side. Beast handles TLS shutdown as part of its SSL teardown; the
runtime does not call `async_shutdown` concurrently with it or with a read.
A reciprocal WebSocket Close followed by TLS `close_notify` is the normal
exchange. A missing `close_notify`, a transport abort or another close error
is not reported as a successful live WSS close; `close()` returns
`connection_closed` or `timeout` after local cleanup. Earlier connection loss
is reported via the existing event/operation paths; repeat `close()` remains
idempotent and `ws://` keeps its best-effort close contract. No reconnect,
resubscribe, replay or session recovery is attempted.

## Qualification

```sh
scripts/check-cpp-oms-011-verified-wss.sh [existing-external-scratch-parent]
CPP_OMS_011_SANITIZERS=1 scripts/check-cpp-oms-011-verified-wss.sh
CPP_OMS_011_OFFLINE=1 scripts/check-cpp-oms-011-verified-wss.sh
# Plaintext-only baseline and shutdown regression:
scripts/check-cpp-oms-003-runtime.sh
scripts/check-cpp-oms-003-real-log-sleet.sh
```

The CPP-OMS-011 script requires GCC 14 and Clang 19 by default
(`CPP_OMS_011_COMPILERS` can name other compilers), builds strict C++17 at
`-O1`, generates fresh certificates and binaries in a private external
directory, and never changes an existing Sleet checkout or Cargo target.
Set `AMS_GRA_UCI_2_5_ROOT`, `AMS_GRA_UCI_2_6_ROOT`, or
`CPP_OMS_011_SLEET_SOURCE` to reuse read-only inputs; the schema hashes and
Sleet commit/tree are still checked. `AMS_GRA_CLI` can select a previously
built production generator. `CPP_OMS_011_SANITIZERS=1` requires ASan/UBSan
with leak detection and attempts TSan (default instrumenting compiler
`clang++-19`). An unusable TSan startup is reported separately. The C++ PR
smoke route still runs its plaintext suites and adds two loopback cases: a
verified TLS 1.2 connection with complete close, and rejection of a trusted
certificate with the wrong DNS SAN. No UCI or Sleet download in smoke.

The offline peer uses a temporary CA and independently records SNI, TLS
version, Host and request target checks, and certificate alerts before any
WebSocket/OWP bytes. It tests default paths without an added bundle and then
with a second, unrelated added CA (using a process-local `SSL_CERT_FILE` for
the default anchor). The host trust store is not edited. The matrix includes
an untrusted chain, untrusted self-signed leaf, wrong DNS/IP SAN, expired and
future certs, CN-only identity, wrong server purpose, interrupted and timed out
TLS, post-TLS drop, HTTP/WS/subprotocol rejection, INIT and INFO failure,
pending SUB/PUB, receive loss, an observed in-flight 32 MiB PUB, and separate
WS Close and TLS shutdown timeouts and truncation. The probe checks no
runtime threads or client socket descriptors remain after each case.

For UCI 2.5 and 2.6 the script checks generator readiness and runs the same
typed `Log` test over plain WS and native WSS. Pinned Sleet
`e38f61d8ce0d75c8508434a52f2ed77c69cf6a3b` listens using TCP plus
WebSocket, not native TLS. The test-only relay (built on Tasks 108/109's
MemoryBIO peer) terminates the actual TLS connection from the C++ runtime
and forwards bytes unchanged. Its passive wire receipt and independent raw
publisher/observer check genuine broker INIT/INFO, SUB/UNSUB, PUB and MSG;
the runtime callback decodes the genuine generated `Log` type. This pin
drops the socket on a received WebSocket Close rather than replying with
one. The test records that behavior and asserts the runtime's close
diagnostic and bounded cleanup, not a completed broker Close handshake. The
independent local peer separately qualifies a complete WS and TLS close.