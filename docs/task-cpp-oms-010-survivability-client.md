# CPP-OMS-010: optional typed Survivability reception

Base: `1edb616ae348f789b3f2f1e4d6f9cb5843e36d39`, tree
`1ab14cfb2c82640da65919a88e3da18faac79538` (#217–#220). The change is limited to
the standalone C++ receiver, its qualification, and documentation. No changes
to the generator, codec, transport, LA-CAL runtime, schema pins, Sleet or CI.

## Readiness and generation gate

Before changing application code, ran the production CLI from the base with
`service-check --language cpp --world closed-schema --with-codec`. Both the
MessageDefinitions and SecurityMarkings XSD hashes were verified against the
canonical pins. Each receive-only contract selects exactly `Log`,
`RDMA_InitializeSetup`, and `SurvivabilityRiskLevel`.

| Pinned UCI | Selected/renderable messages | Renderable/selected types, model + API | Renderable/required codecs | Exit / first blocker |
| --- | --- | --- | --- | --- |
| 2.5 | 3/3 | 64/64 READY | 63/63 READY | 0 / none |
| 2.6 | 3/3 | 65/65 READY | 64/64 READY | 0 / none |

Both checks had empty stderr. Production `service-generate` emitted `oam.hpp`,
`service_api.hpp` and `service_codec.hpp` for each. A probe instantiated all
three generated subscribe functions with bound codecs and compiled without
diagnostics on GCC 14.2 and Clang 19.1.7 with the full CPP-OMS-009 warning policy,
`-std=c++17`, `-Werror` and `-O1`.

The generated third input is `service_api::function_survivability::exchange_receive::subscribe`.
`Payload` aliases `programs::oam::SurvivabilityRiskLevelMT`. The OWP global name
is `SurvivabilityRiskLevel`, not the C++ type name, and the topic is
`uci.survivability`. The client binds `service_codec::encode_SurvivabilityRiskLevelMT`
and `service_codec::decode_SurvivabilityRiskLevelMT` with
`AMS_GRA_LACAL_PAYLOAD_CODEC`; it only receives.

## Profiles, ownership and redaction

See the [client README](../examples/cpp-uci-client/README.md) for CLI modes,
topic authorization and full build instructions. Profile A retains the
`cpp-uci-client-uci25` / `cpp-uci-client-uci26` names. Profile B retains the
`-log-rdma` suffix. Profile C uses `-log-rdma-survivability`, with its own
generation/object directories and manifests. A and B do not depend on the
generated Survivability types. Every profile defaults to Log only. The
Survivability build and runtime switches both require `--with-rdma`.

The application thread owns the one Runtime and its three independent,
move-only handles. After the `owp` upgrade and a single INIT/INFO, startup
acknowledges Log, then RDMA, then Survivability. It checks runtime connection
health before announcing `READY`. A failed SUB prevents `READY`; the client
releases the earlier handles in reverse order. A pending synchronous operation
finishes or times out before a signal is acted on. A drop concurrently with or
after the final acknowledgment may be observed only after the health check;
`READY` is not a lease. A dead connection never produces a claimed UNSUB ack.

The existing runtime dispatcher hands the typed summaries off to the
unchanged bounded SPSC queue. The message limit counts consumed typed summaries
across the enabled families; rejected documents do not consume the limit.
There is no cross-SID publication-order guarantee. Shutdown quiesces each SID
in reverse order, closes the queue and counts pending summaries, then closes
the Runtime and observes threads and sockets. There is no TLS, reconnect,
retry, resubscribe or replay; native `wss://` is unsupported.

The third summary reads `messagedata.survivabilityrisklevelid.uuid`,
`messageheader.{mode,schemaversion,timestamp}`, and
`securityinformation.classification`. `SurvivabilityRiskSettingType` contains
`riskprioritization` (`RiskPrioritizationEnum`) and `risklevel`. Each
`RiskSettingType` is a record whose `allowedrisk` is a `VulnerabilityTypeEnum`.
The pinned arrays are bounded 1–4 and 1–5, respectively; output independently
caps them at three settings and four levels, reporting omissions. No labels,
caveats, system IDs from `AppliesToSystemID`, exposure durations or raw JSON
are queued or printed. Survivability decoder events show the known identity,
error category, a fixed allowlisted schema path or ancestor, and the payload
byte count. Handler exceptions do not carry payload length in the unchanged
runtime. No free-form decoder or exception text is printed. Reported family
counts require both a known global name and an owned SID. Missing events are
not evidence of zero rejections; aggregate runtime statistics are retained.

## Qualification evidence

The pinned, clean Sleet revision was `e38f61d8ce0d75c8508434a52f2ed77c69cf6a3b`,
tree `7e705f4eb5f658957bd414b4abc8207311f3c1e4`. For each UCI release the raw
OWP publisher and observer ran independently of the application and never
used a generated encoder. Test inputs came from Task093 Log, Task092/103
RDMA, and Task104/105 Survivability, with application topics/envelopes and
focused field variations to expose order, redaction and display limits.

Each main run established one application connection, sent one INIT, received
one INFO, registered three distinct SIDs (`sub-1`, `sub-2`, `sub-3`) and
processed six typed summaries, two per family, in per-SID order. The observer
checked nine broker-admitted MSGs byte for byte. On both releases Sleet accepted
the invalid Survivability timestamp (`+OK` and a byte-exact observed MSG) and
the generated decoder rejected it at `SurvivabilityRiskLevelMT.MessageHeader.Timestamp`
(475 envelope bytes). Valid reception resumed on the same SID. The established
Log hexBinary and RDMA timestamp cases each also reached and were rejected by
their decoder. Eleven other PUBs (global name/topic, enumerations, required
fields, both array bounds, repeated `OwnerProducer` Choice) were rejected by
Sleet and produced no observed MSG. The wrong release failed at INIT.

The main run and both signals with three active SIDs acknowledged UNSUB in
`sub-3`, `sub-2`, `sub-1` order. Broker denial at each of the three SUBs
unsubscribed exactly the earlier SIDs. The scripted peer covers TCP loss during
each SUB, signal before the third SUB and while it is pending, a drop just
after final acknowledgment, and a drop after `READY`. Dead connections report
failed acknowledgments for retained SIDs. Receiver-core tests independently
verify aggregate and per-family accounting: the capacity-three queue accepts
one summary from each family, then rejects one more from each, and discards
the three pending summaries on close. A Survivability callback exception is
contained and attributed without payload/exception text; RDMA and Log still
deliver in the core test. Per-SID unsubscribe and post-close publications
verify no further callbacks, and repeated shutdown performs no more actions.
The application reports zero remaining runtime threads and no leaked sockets.

With both compilers, A and B were built before and after C in the same output
roots. Binary, manifest, generated-header and object hashes (32 entries per
compiler) were unchanged across the build of C and subsequent A/B rebuilds.
Compared to the base, A/B generated header hashes and help output were
identical; rebuilding the changed application source does not give identical
executable bytes. Log-only and Log + RDMA modes of C retain the native A/B
output in matched sessions, including the typed summary fields.

The original CPP-OMS-008 Log-only and CPP-OMS-009 two-message campaigns passed
on both releases. The three-message client and receiver-core cases also passed
on both releases with Clang ASan/UBSan (including leak detection) and TSan.
The existing CPP-OMS-009A codec/address regressions, local C++ PR smoke and
43 PR smoke routing tests passed.

## Reproduce the focused campaign

From the repository root, prepare the same pinned roots and clean Sleet
checkout as in the client README. `SLEET_CHECKOUT` must already have
`target/release/sleet` built at the pinned revision. `CXX` chooses the compiler.

```bash
set -e
s=$(mktemp -d)
cxx=${CXX:-g++}
client=examples/cpp-uci-client
strict=(-std=c++17 -Wall -Wextra -Wpedantic -Werror -Wconversion -Wsign-conversion
        -Wshadow -Wold-style-cast -Wcast-align -Wnon-virtual-dtor -Woverloaded-virtual
        -Wnull-dereference -Wdouble-promotion -Wformat=2 -O1 -pthread)
inc=(-I cpp/oms-lacal-runtime/include -I "$client/src" -I tests/cpp-lacal)
extra=()
if [[ -n ${CPP_UCI_CLIENT_EXTRA_CXXFLAGS:-} ]]; then
  read -r -a extra <<< "$CPP_UCI_CLIENT_EXTRA_CXXFLAGS"
fi
for peer in wire_peer negative_peer; do
  "$cxx" "${strict[@]}" "${inc[@]}" "$client/tests/$peer.cpp" -o "$s/$peer"
done
for v in 25 26; do
  root=$AMS_GRA_UCI_2_5_ROOT
  if [[ $v == 26 ]]; then root=$AMS_GRA_UCI_2_6_ROOT; fi
  build=("$client/build.sh" --release "2.${v:1}" --schema "$root" --out "$s")
  CXX="$cxx" "${build[@]}"
  CXX="$cxx" "${build[@]}" --with-rdma
  CXX="$cxx" "${build[@]}" --with-rdma --with-survivability
  hash=$(sha256sum "$root" | cut -d' ' -f1)
  profile="uci$v-log-rdma-survivability"
  "$cxx" "${strict[@]}" "${extra[@]}" "${inc[@]}" -I "$s/generated/$profile" \
    -DCPP_UCI_CLIENT_WITH_RDMA=1 -DCPP_UCI_CLIENT_WITH_SURVIVABILITY=1 \
    -DCPP_UCI_CLIENT_UCI_RELEASE="$v" "-DCPP_UCI_CLIENT_SCHEMA_SHA256=\"$hash\"" \
    "$client/tests/survivability_test.cpp" "$s/obj/$profile/owp.o" \
    "$s/obj/$profile/runtime.o" -o "$s/survivability-$v"
done
PYTHONDONTWRITEBYTECODE=1 python3 "$client/tests/check_survivability.py" \
  --build "$s" --wire "$s/wire_peer" --peer "$s/negative_peer" \
  --out "$s/evidence" --sleet "$SLEET_CHECKOUT" \
  --schema25 "$AMS_GRA_UCI_2_5_ROOT" --schema26 "$AMS_GRA_UCI_2_6_ROOT"
```

Use a fresh scratch directory for each sanitizer, and pass sanitizer compile
and link options through `CPP_UCI_CLIENT_EXTRA_CXXFLAGS`. The uninstrumented
raw peers and Sleet do not need to be rebuilt for sanitizer coverage.