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
* `src/owp.cpp`, `src/runtime.cpp` — compile both (C++17, `-pthread`).

Dependency: Boost.Beast headers, Boost >= 1.81 (qualified on 1.83.0; Ubuntu
24.04 `libboost-dev`). No Boost library is linked. `wss://` is refused.

Design, safety rules and qualification evidence:
[docs/task-cpp-oms-003-lacal-runtime.md](../../docs/task-cpp-oms-003-lacal-runtime.md).
