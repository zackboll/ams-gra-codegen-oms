# CPP-OMS-009A — checked address codecs for C++

## Admission

The C++ codec analyzer and renderer both call `string_profile` and
`cpp_codec_admits_string_profile`. The new alternating-ASCII arm accepts only
references to the canonical 16 Task 060 rows (15 per pinned release), not
independently assembled profiles, even if their bodies look identical. The
IPv6 arm accepts the Task 062 variant, which the classifier returns only for
the exact pinned constraint tuple. Neither arm uses declaration names.

The codec still encodes with `put_string(value.value())` and decodes through
`get_checked_text<T>` → `T::create`. The generated model is the only lexical
validator. No new parser or address normalization was added. In particular,
standalone IPv4 rejects padded decimal octets; IPv6 retains case, accepts
schema-valid forms such as `1::2::3` and `:001.009.099.199`, and emits them
unchanged. C++ Unicode 3.1 profiles, unconstrained named Strings, unknown
constraints, and unsupported direct field-local constraints still fail closed.
Float32, named `Time`, length-constrained hexBinary, optional/repeated Choice
alternatives, and polymorphic `$type` mappings are unchanged and unsupported.

## Staged production evidence

Base: `885588a660ac48532fa4407f6a388abfb667c6b9`, tree
`cce410f9e7d53e737f0d0aead37d075b57b61ba7`. Both committed receive-only
contracts in `tests/fixtures/cpp-codec/addresses/` are byte-identical to the
CPP-OMS-009 blocker evidence. UCI pin commits were `093610b7753944059360d3236770ab446d039556`
and `78eb61b6112c8bffa40820c33124b57787fc5bd9`; the message-definition roots and
SecurityMarkings files matched the existing CPP-OMS pin digests.

| Release | Messages | Model/API throughout | Base codec | Task 060 only | Final codec |
| --- | ---: | ---: | --- | --- | --- |
| 2.5 | 2/2 | 55/55 READY | 52/54 NOT READY | 53/54 NOT READY | 54/54 READY |
| 2.6 | 2/2 | 57/57 READY | 54/56 NOT READY | 55/56 NOT READY | 56/56 READY |

Both base runs exited 1 with the first codec boundary on stdout:
`IPv4_AddressType: named String profile alternating ASCII has no C++ codec mapping`.
With **only** the alternating-ASCII admission applied, both reruns exited 1
and reported `IPv6_AddressType: named String profile IPv6 address has no C++ codec mapping`.
Stderr for those four runs was `error: service selection is not renderable for cpp under closed-schema`.
Only after those reruns was IPv6 admitted. Both final checks and both
generations exited 0. The generated `oam.hpp` and `service_api.hpp` matched
plain generation on the base byte for byte.

Both API headers expose `service_api::function_log::exchange_receive`
(`Log`, `uci.log`, `programs::oam::LogMT`) and
`service_api::function_rdma::exchange_receive`
(`RDMA_InitializeSetup`, `uci.rdma`, `programs::oam::RDMAInitializeSetupMT`) in
the `https://www.vdl.afrl.af.mil/programs/oam` message namespace. The actual
codec entry points are `service_codec::decode_RDMAInitializeSetupMT` and
`service_codec::encode_RDMAInitializeSetupMT`. The `endpoint` field of
`RDMAInitializeSetupMDT` is `UnboundedVector<NetworkEndpointType, 0>`; its
Choice alternatives are `NetworkEndpointType::IPv4Endpoint` and
`NetworkEndpointType::IPv6Endpoint`, with checked `IPv4AddressType` and
`IPv6AddressType` members in the corresponding endpoint records.

## Executed checks

* The address consumer ran 6,158 Task 060 IPv4 vectors (including the added
  `198.51.100.42` witness) and all 26,946 independent Task 062 IPv6 vectors,
  each in required, optional, repeated and Choice positions. Successful
  encode/decode preserves lexical text; negative vectors check schema paths
  including repeated indices and Choice keys. Wrong JSON kinds, malformed
  JSON, and an escaped positive are checked separately.
* The authentic RDMA consumer used the existing `task092-real-rdma25.json`
  and `task092-real-rdma26.json`. It checked typed alternatives, address
  values, cardinalities 0/1/2/4, exact spelling on wire, round trips, and
  carrier and codec rejection. Encoded output was compared as an independent
  JSON value with the original fixture. The existing Log consumer also ran
  against the combined headers (307 checks, 189/192 rejections).
* Address-bearing headers were compiled with the existing Task 062 gate
  (`-std=c++17 -Wall -Wextra -Werror -pedantic`) on GCC 14.2.0 and Clang
  19.1.7. The unchanged IPv6 carrier emits sign-conversion diagnostics under
  the broader CPP codec flags with `-Wsign-conversion`; this task does not
  change the model or production warning policy. Rust/Cargo 1.98.1 and Python
  3.13.5 were used locally. Local C++ PR smoke passed with Boost 1.83 headers,
  and all 43 PR smoke routing/execution tests passed.
* The CPP-OMS-002 standalone codec script and CPP-OMS-006 structured-ASCII
  script passed. FileMetadata remains NOT READY at `SHA_2_256_HashType`
  (49/50 and 51/52). The CPP-OMS-005 bounded-ASCII regression consumer suite
  and the authentic AMTI settings codec consumers ran without changing Sleet
  or running a new transport campaign.
* The existing batch sweep evaluated 722/725 global elements, with 676/679
  model-READY selections unchanged. Both releases rose from 400 to 408
  codec-READY single-message selections, with zero losses or decreases in
  renderable counts. The separately measured single-message gains were
  `COMINT_Capability`, `FacilityRecord`, `IO_PortCommand`, `IO_PortStatus`,
  `LaunchObservation`, `RDMA_Initialize`, `RDMA_InitializeSetup`, and
  `UnitRecord` in both releases. This sweep measures readiness, not runtime
  or transport qualification for those additional selections.

## Reproduce

Build the base and final release CLIs separately with
`cargo build --locked --release -p ams-gra-codegen-oms`. To reproduce the
intermediate stage, change only the alternating-ASCII match arm in the base
predicate to the canonical-row match in the final predicate. For each CLI and
pin, run the same command:

```sh
"$CLI" service-check --schema "$PINNED_UCI_ROOT" \
  --contract "tests/fixtures/cpp-codec/addresses/uci${VERSION}-log-rdma-service.yaml" \
  --language cpp --world closed-schema --with-codec
```

`VERSION` is `25` or `26`. For final generation, replace `service-check` with
`service-generate` and add `--output "$OUT"`. With both
`AMS_GRA_UCI_2_5_ROOT` and `AMS_GRA_UCI_2_6_ROOT` set to the pinned roots, run:

```sh
AMS_GRA_REQUIRE_CPP_OMS_009A_UCI=1 cargo test --locked --release \
  -p ams-gra-codegen-oms --test cpp_codec cpp_oms_009a -- --nocapture
python3 -I scripts/check-cpp-oms-006-codec-sweep.py \
  --cli "$FINAL_CLI" --baseline "$BASE_CLI" \
  --uci-2-5 "$AMS_GRA_UCI_2_5_ROOT" --uci-2-6 "$AMS_GRA_UCI_2_6_ROOT" \
  --scratch "$SWEEP_OUT" --require-no-loss
```

Local raw command, source identity, stdout, stderr, exit and generated-header
evidence is under `$HOME/cpp-oms-009a-evidence`. No pinned schemas or generated
headers are committed. The standalone app, runtime and transport are not
changed or qualified by this task.