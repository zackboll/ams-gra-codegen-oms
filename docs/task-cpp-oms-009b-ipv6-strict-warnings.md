# CPP-OMS-009B — strict C++17 IPv6 carriers

On `fd239deaabf97d233f3c82b17f004adb8c059aa1`, a translation unit including
the generated Log + RDMA model, API, and codec failed with the standalone
Log client's full warnings at `-O1`. GCC 14.2 reported `conversion to
'unsigned char' from 'std::basic_string_view<char>::value_type' {aka 'char'}
may change the sign of the result`; Clang 19.1 reported `implicit conversion
changes signedness: 'const value_type' (aka 'const char') to 'const unsigned
char'`. Both point to `hex_group` and `ipv4_octet` in `oam.hpp` (UCI 2.5:
lines 554 and 566; UCI 2.6: lines 534 and 546). The probe includes neither
the runtime nor Boost.

Both loops now read `char` and explicitly convert the byte to `unsigned char`
before testing it, as `hex_colon` already did. The conversion produces the
same byte value as before; the bounds, separator checks, digit arithmetic,
and storage are unchanged. `create(std::string_view)` validates the exact
Task 062 lexical profile and `value()` returns the original spelling.

The address-corpus and authentic RDMA tests now use all of `STRICT` at `-O1`.
GCC 14 separately diagnosed `-Wnull-dereference` in libstdc++ `streambuf`
when inlining the RDMA test's `istreambuf_iterator` fixture read. This also
reproduces without generated headers. The test copies the fixture through
`std::ostringstream` instead; its input, golden comparison, and codec
assertions are unchanged. No warning flags are suppressed.

| Pinned UCI | Combined model/API | Combined codec | Generated diff vs. base |
| --- | ---: | ---: | --- |
| 2.5 | 55/55 READY | 54/54 READY | Only the two loop edits in `oam.hpp` |
| 2.6 | 57/57 READY | 56/56 READY | Only the two loop edits in `oam.hpp` |

Both combined service API and codec headers, and all three Log-only headers
for each release, are byte-identical to the baseline. The local evidence
directory (`$HOME/cpp-oms-009b-evidence`) retains SHA-256 hashes, focused
generated-model diffs, and compiler and test logs.

GCC 14.2 and Clang 19.1 passed the strict combined header probe and the
authentic RDMA consumer for both releases. Both also passed the address codec
corpora (6,158 IPv4 and 26,946 IPv6 vectors). The unchanged standalone C++
Task 062 oracle passed all 26,946 vectors and caught the deliberately wrong
expectation before it was restored.

To rerun the independent model corpus and the checked-codec tests, set
`TMPDIR` and `CARGO_TARGET_DIR` to external directories and point
`AMS_GRA_UCI_2_5_ROOT` and `AMS_GRA_UCI_2_6_ROOT` at the pinned message-definition
roots:

```sh
cargo test --locked -p ams-gra-oms-backend-cpp --lib \
  standalone_ipv6_compiler_corpus_and_lifecycle -- --nocapture
AMS_GRA_REQUIRE_CPP_OMS_009A_UCI=1 cargo test --locked --release \
  -p ams-gra-codegen-oms --test cpp_codec cpp_oms_009a -- --nocapture
```

This correction does not change codec admission or implement the CPP-OMS-009
application changes.
