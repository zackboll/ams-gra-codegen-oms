# C++17 LA-CAL runtime (`ams_gra::lacal`)

Handwritten, separately maintained runtime for generated C++ services
(CPP-OMS-003). The generated `service_api.hpp` forwards publish/subscribe to
`ams_gra::lacal::Runtime`; the generated `service_codec.hpp` encodes and
decodes payload bodies, bound once per payload type:

```cpp
#include "ams_gra/lacal/runtime.hpp"
#include "service_api.hpp"
#include "service_codec.hpp"

AMS_GRA_LACAL_PAYLOAD_CODEC(::programs::oam::LogMT,
                            ::service_codec::encode_LogMT, ::service_codec::decode_LogMT)
```

* `include/ams_gra/lacal/runtime.hpp` — public API (`Runtime`, `Subscription`,
  `Config`, `Status`/`Result`, `Error`/`ErrorKind`, `Event`, `Stats`).
* `include/ams_gra/lacal/owp.hpp` — pure OWP 1.0 framing and envelope helpers.
* `detail::TestAccess` / `detail::IoSnapshot` in `runtime.hpp` are a test-only
  observation seam for `tests/cpp-lacal`; they are not part of the supported API.
* `src/owp.cpp`, `src/runtime.cpp` — compile both (C++17, `-pthread`).

## Building and connecting

The default build uses Boost.Beast headers (Boost >= 1.81) and has **no
OpenSSL dependency**. `ws://` works as before; `wss://` is refused before
connecting. For native verified WSS, compile the runtime with
`-DAMS_GRA_LACAL_ENABLE_TLS=1` and link `-lssl -lcrypto` after the runtime
objects. Use Boost 1.83+ and OpenSSL 1.1.1+ headers/libraries (qualification
uses OpenSSL 3.x; use a maintained OpenSSL release). No Boost library is
linked. On Ubuntu 24.04 install `libboost-dev libssl-dev` (`openssl` too for
the test harness). `ws://` still uses plaintext in the TLS-enabled build.

```cpp
ams_gra::lacal::Config config;
config.url = "wss://broker.example.com:443/owp";
config.service_id = "example-service";
config.schema_version = "002.6.0";
// Optional: adds CA anchors to OpenSSL's default system trust paths.
config.tls_ca_file = "/etc/example/private-ca.pem";
auto runtime = ams_gra::lacal::Runtime::connect(config);
```

WSS verifies the certificate chain, validity, server purpose, and the original
URL host before sending the WebSocket upgrade. DNS hosts use SNI and a DNS SAN
(not the subject CN); IPv4 and bracketed IPv6 use an IP SAN and send no SNI.
Use ASCII DNS names (IDNs in A-label form). There is no verification toggle,
plaintext fallback, client certificate, or reconnect. `tls_ca_file` is only
valid for `wss://`; omit it for default trust alone. It accepts PEM CA
certificate blocks separated by whitespace, up to 16 MiB, in a regular file.

One `connect_timeout` covers DNS, TCP, TLS, WebSocket upgrade (`owp`), and
OWP INIT / `+OK` / `INFO`. One `close_timeout` covers write drain, WebSocket
Close, and TLS shutdown. A failed live WSS close is reported even though the
runtime releases its resources. See the task notes for error classes and
system DNS and shutdown limitations.

**The standalone C++ UCI client remains `ws://` only** until a separate CLI
and build integration task enables the runtime feature.

Design, safety rules and qualification evidence:
[CPP-OMS-003](../../docs/task-cpp-oms-003-lacal-runtime.md) (original runtime)
and [CPP-OMS-011](../../docs/task-cpp-oms-011-native-wss.md) (WSS).
