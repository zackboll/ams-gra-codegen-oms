# CPP-OMS-009: one C++ UCI connection, two typed subscriptions

Base: `a14888c631d0522e07a5e0297747fe8dfa375dee`, tree
`d4ac27952c1aa938fb69f7cee32824b7aa9b3bd8` (#217 standalone Log,
#218 checked IPv4/IPv6 codecs, #219 strict IPv6 compilation). No changes to
the generator, canonical pins, LA-CAL runtime, transport, Sleet or CI.

See the [client README](../examples/cpp-uci-client/README.md) for build and
invocation examples, release selection, Sleet topic bindings and exit codes.
The original Log-only build and default runtime mode use `Log` on `uci.log`.
`build.sh --with-rdma` creates separate `cpp-uci-client-uci25-log-rdma` and
`cpp-uci-client-uci26-log-rdma` artifacts. Either is still Log-only unless
invoked with `--with-rdma`. The generated RDMA receive exchange is
`service_api::function_rdma::exchange_receive`, with global message
`RDMA_InitializeSetup` on `uci.rdma` and payload `RDMAInitializeSetupMT`.
The C++ type name is not an OWP identity.

## Ownership and ordering

One application thread owns one `Runtime` and performs every runtime operation.
After the WebSocket `owp` handshake and successful INIT/INFO, it subscribes to
Log, then RDMA. The two move-only handles retain their own SIDs. Combined
`READY` means both SUBs were acknowledged; it is not a lease or a guarantee
against subsequent transport failure.

The single runtime dispatcher runs both typed callbacks serially. Each extracts
only selected fields into a summary, then writes it to a preallocated bounded
SPSC ring; no application lock is acquired on this producer path. The consumer
polls with bounded waits in C++17. A full ring is a sticky, reported session
failure, not a drop-and-resume policy. Only the application thread assigns
display sequence numbers; per-subscription order is preserved, with no promise
of cross-subscription publication order. The message limit counts both families.
RDMA copies only the first four checked addresses as originally spelled, while
reporting the total array length. No arbitrary payload text is copied for display.

On a rejected second SUB the client retains Log's SID, does not print `READY`,
and explicitly unsubscribes Log. If transport has already failed, the runtime
can remove local registrations but cannot acknowledge UNSUB. Teardown attempts
established SIDs in reverse order, then closes the queue and counts pending
summaries discarded. `Runtime::close()` runs on the application thread and
joins the runtime threads. The `/proc` report distinguishes sockets already
open at startup from runtime leaks. Successful UNSUB quiesces only its SID;
`callbacks-after-unsubscribe` checks both families independently. The shutdown
report is cached, so repeating shutdown issues no more commands.

SIGINT and SIGTERM record the signal without doing I/O; setup observes it at
the next operation boundary. `--duration-seconds` starts after setup. No
reconnect, resubscribe, replay, TLS or native WSS are implemented. An event
diagnostic can be printed after a subsequent typed summary: the ring and the
runtime event queue are distinct. Per-family diagnostic counters count the
events observed; the runtime also exposes aggregate decoder/handler counts.

## Qualification

Preflight used the production `service-check` and `service-generate` with
`--language cpp --world closed-schema --with-codec` on the two address fixture
contracts before modifying the application. `oam.hpp`, `service_api.hpp` and
`service_codec.hpp` compiled with all CPP-OMS-008 warning flags, `-Werror` and
`-O1`, using GCC 14.2 and Clang 19.1.7.

| UCI | Selected/renderable messages | Model + API | Codec |
| --- | --- | --- | --- |
| 2.5 | 2/2 | 55/55 READY | 54/54 READY |
| 2.6 | 2/2 | 57/57 READY | 56/56 READY |

Both releases were qualified separately against unmodified Sleet
`e38f61d8ce0d75c8508434a52f2ed77c69cf6a3b`, tree
`7e705f4eb5f658957bd414b4abc8207311f3c1e4`. The independent raw OWP publisher
and observer use Task093 Log and Task092/103 RDMA fixtures, not the generated
encoder. Each main run used one application connection and INIT/INFO, distinct
`sub-1` and `sub-2`, and two successful SUBs. Four typed summaries (two per
family) retained their expected fields and per-SID order. Two more MSGs were
admitted and observed byte-for-byte, but rejected by the generated decoders:
Log odd-length hexBinary at `LogMT.MessageData.Data` and the Task103 timestamp
at `RDMA_InitializeSetupMT.MessageHeader.Timestamp`. Valid reception resumed
on both SIDs. Wrong global name, wrong-topic publication and malformed RDMA
were broker rejections with **no MSG**; the wrong release failed at INIT.
Neither application nor qualification status output prints rejected payloads.

Both UNSUBs were acknowledged in reverse order on successful completion. The
same campaign checks duration, first- and second-topic denial and SIGINT/
SIGTERM with both SIDs active. The scripted peer covers TCP loss during the
second SUB, a signal while its acknowledgment is pending, a protocol failure
after the final acknowledgment, and TCP loss after `READY`. On both releases
the core tests exercise a shared capacity of two (two accepted summaries, two
explicit full rejections, two pending discards), an RDMA callback exception,
per-SID callback quiescence with the other SID active, idempotent cleanup, and
endpoint summary truncation. The original CPP-OMS-008 suite and Sleet regression
remain separate.

Both build profiles compiled with the strict warning policy on GCC 14.2 and
Clang 19.1.7. Hashes of the Log-only binaries, manifests and generated headers
were unchanged after building the combined profile into the same output root.
The focused campaign passed for both releases on both compilers, and passed
again with Clang ASan/UBSan (including leak detection) and TSan. The existing
address/readiness tests, local C++ PR smoke and 43 smoke-routing regression
tests also passed.

### Reproduce the focused campaign

Run from the repository root. Set `AMS_GRA_UCI_2_5_ROOT` and
`AMS_GRA_UCI_2_6_ROOT` to the pinned MessageDefinitions XSDs as in the README.
`SLEET_CHECKOUT` is an external, clean checkout built at the pinned revision
(`cargo build --release --locked -p sleet` in that checkout). Use one scratch
directory per compiler or sanitizer configuration. This example assumes Boost
headers are on the compiler's include path.

```bash
s=$(mktemp -d)
cxx=${CXX:-g++}
client=examples/cpp-uci-client
strict=(-std=c++17 -Wall -Wextra -Wpedantic -Werror -Wconversion -Wsign-conversion
        -Wshadow -Wold-style-cast -Wcast-align -Wnon-virtual-dtor -Woverloaded-virtual
        -Wnull-dereference -Wdouble-promotion -Wformat=2 -O1 -pthread)
inc=(-I cpp/oms-lacal-runtime/include -I "$client/src" -I tests/cpp-lacal)
for peer in wire_peer negative_peer; do
  "$cxx" "${strict[@]}" "${inc[@]}" "$client/tests/$peer.cpp" -o "$s/$peer"
done
for v in 25 26; do
  root=$AMS_GRA_UCI_2_5_ROOT
  if [[ $v == 26 ]]; then root=$AMS_GRA_UCI_2_6_ROOT; fi
  CXX="$cxx" "$client/build.sh" --release "2.${v:1}" --schema "$root" --out "$s"
  CXX="$cxx" "$client/build.sh" --release "2.${v:1}" --schema "$root" --out "$s" --with-rdma
  hash=$(sha256sum "$root" | cut -d' ' -f1)
  profile="uci$v-log-rdma"
  "$cxx" "${strict[@]}" "${inc[@]}" -I "$s/generated/$profile" \
    -DCPP_UCI_CLIENT_WITH_RDMA=1 -DCPP_UCI_CLIENT_UCI_RELEASE="$v" \
    "-DCPP_UCI_CLIENT_SCHEMA_SHA256=\"$hash\"" "$client/tests/multimessage_test.cpp" \
    "$s/obj/$profile/owp.o" "$s/obj/$profile/runtime.o" -o "$s/receiver-$v"
done
PYTHONDONTWRITEBYTECODE=1 python3 "$client/tests/check_log_rdma.py" \
  --build "$s" --log-build "$s" --wire "$s/wire_peer" --peer "$s/negative_peer" \
  --out "$s/evidence" --sleet "$SLEET_CHECKOUT" \
  --schema25 "$AMS_GRA_UCI_2_5_ROOT" --schema26 "$AMS_GRA_UCI_2_6_ROOT"
```

For ASan/UBSan or TSan, use Clang and pass the same `-fsanitize=...` and
`-fno-omit-frame-pointer` flags to the application build (via
`CPP_UCI_CLIENT_EXTRA_CXXFLAGS`) and receiver-core test compile/link. The
independent peers and pinned broker need not be instrumented.