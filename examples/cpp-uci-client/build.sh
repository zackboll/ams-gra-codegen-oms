#!/usr/bin/env bash
# Build the standalone C++17 UCI receiver for ONE pinned UCI
# release, from a clean checkout, into external build storage.
#
#   examples/cpp-uci-client/build.sh --release 2.5|2.6 --schema MESSAGE_DEFINITIONS_XSD --out BUILD_DIR [--with-rdma [--with-survivability]]
#
#   1. verifies the pinned MessageDefinitions root and its sibling
#      SecurityMarkings by SHA-256 (the same pins as scripts/task102/admission.py);
#   2. production `service-check --language cpp --world closed-schema --with-codec`
#      on the release/profile contract must report model, service API and codec READY;
#   3. production `service-generate ... --with-codec` writes oam.hpp,
#      service_api.hpp and service_codec.hpp to BUILD_DIR/generated/uci{25,26};
#   4. strict C++17 compile of src/main.cpp with the existing runtime sources
#      (cpp/oms-lacal-runtime/src) and the generated headers into
#      BUILD_DIR/bin/cpp-uci-client-uci{25,26}; the release, schema version and
#      root hash are compile-time constants checked against the generated types.
#
# Environment: CXX (default g++), AMS_GRA_CLI (prebuilt production CLI; built
# with cargo otherwise), CPP_UCI_CLIENT_EXTRA_CXXFLAGS (e.g. sanitizers).
# Requires Boost >= 1.81 headers (Boost.Beast; no Boost library is linked).
# All build artifacts are written to external storage.
set -euo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo="$(cd "$here/../.." && pwd)"

usage() { echo "usage: $0 --release 2.5|2.6 --schema MESSAGE_DEFINITIONS_XSD --out BUILD_DIR [--with-rdma [--with-survivability]]" >&2; exit 2; }
release="" schema="" out="" with_rdma=0 with_survivability=0
while (($#)); do
  case "$1" in
    --release) (($# >= 2)) || usage; release="$2"; shift 2 ;;
    --schema) (($# >= 2)) || usage; schema="$2"; shift 2 ;;
    --out) (($# >= 2)) || usage; out="$2"; shift 2 ;;
    --with-rdma) ((with_rdma == 0)) || usage; with_rdma=1; shift ;;
    --with-survivability) ((with_survivability == 0)) || usage; with_survivability=1; shift ;;
    -h|--help) usage ;;
    *) echo "unknown argument: $1" >&2; usage ;;
  esac
done
[[ -n "$release" && -n "$schema" && -n "$out" ]] || usage
if ((with_survivability && !with_rdma)); then
  echo "--with-survivability requires --with-rdma" >&2
  usage
fi

case "$release" in
  2.5) v=25; root_sha=ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27
       marks_sha=4a8056f1503234d423a0c2495844ab65387febced62bea58b87d4f343e9e7a0b
       marks=UCI_SecurityMarkings_v2_5_0.xsd; closure=42; emitted=41 ;;
  2.6) v=26; root_sha=af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b
       marks_sha=ee6e58d5db9fd80d526bc964b8244ec296d106301640d34c3cb2c5c00b710b88
       marks=UCI_SecurityMarkings_v2_6_0.xsd; closure=44; emitted=43 ;;
  *) echo "unsupported UCI release '$release' (expected 2.5 or 2.6)" >&2; exit 2 ;;
esac
schema_version="002.${release#2.}.0"
for pin in "$root_sha" "$marks_sha"; do
  grep -q "$pin" "$repo/scripts/task102/admission.py" \
    || { echo "pin $pin is not in scripts/task102/admission.py" >&2; exit 1; }
done

# ---- 1. pinned schema ---------------------------------------------------------
[[ -f "$schema" ]] \
  || { echo "--schema must name the MessageDefinitions file (fetch it with scripts/fetch-pinned-uci-$release.sh)" >&2; exit 1; }
schema="$(realpath -- "$schema")"
sha() { sha256sum "$1" | cut -d' ' -f1; }
actual="$(sha "$schema")"
[[ "$actual" == "$root_sha" ]] || {
  echo "UCI $release MessageDefinitions SHA-256 $actual != pinned $root_sha" >&2
  echo "(wrong release or file? refetch with scripts/fetch-pinned-uci-$release.sh)" >&2
  exit 1
}
[[ -f "$(dirname "$schema")/$marks" ]] || { echo "sibling $marks is missing next to $schema" >&2; exit 1; }
actual="$(sha "$(dirname "$schema")/$marks")"
[[ "$actual" == "$marks_sha" ]] || { echo "UCI $release SecurityMarkings SHA-256 $actual != pinned $marks_sha" >&2; exit 1; }
echo "UCI $release pinned schema verified: root $root_sha, SecurityMarkings $marks_sha"

# ---- external build storage -----------------------------------------------------
out="$(realpath -m -- "$out")"
case "$out/" in "$repo"/*) echo "refusing to build inside the repository ($out); use external build storage" >&2; exit 2 ;; esac
mkdir -p "$out"

# ---- toolchain --------------------------------------------------------------------
cxx="${CXX:-g++}"
command -v "$cxx" >/dev/null || { echo "C++ compiler '$cxx' not found" >&2; exit 1; }
strict=(-std=c++17 -Wall -Wextra -Wpedantic -Werror -Wconversion -Wsign-conversion -Wshadow -Wold-style-cast
        -Wcast-align -Wnon-virtual-dtor -Woverloaded-virtual -Wnull-dereference -Wdouble-promotion -Wformat=2)
extra=()
[[ -z "${CPP_UCI_CLIENT_EXTRA_CXXFLAGS:-}" ]] || read -r -a extra <<<"$CPP_UCI_CLIENT_EXTRA_CXXFLAGS"
# -O1: the optimization level the CPP-OMS-003 qualification uses for the
# runtime. With GCC 14, -O2 makes -Wnull-dereference (kept, as -Werror) fire
# inside Boost.Asio scheduler code inlined into the unchanged runtime.cpp.
flags=("${strict[@]}" -O1 -pthread "${extra[@]}")
probe="$(mktemp -d "$out/boost-probe.XXXXXX")"
trap 'rm -rf -- "$probe"' EXIT
cat > "$probe/boost.cpp" <<'EOF'
#include <boost/version.hpp>
#include <boost/beast/version.hpp>
#include <cstdio>
int main() { std::printf("Boost %d (%s), %s\n", BOOST_VERSION, BOOST_LIB_VERSION, BOOST_BEAST_VERSION_STRING); return BOOST_VERSION >= 108100 ? 0 : 1; }
EOF
"$cxx" -std=c++17 "$probe/boost.cpp" -o "$probe/boost" 2>"$probe/boost.log" || {
  cat "$probe/boost.log" >&2
  echo "Boost.Beast headers not found (install Boost >= 1.81 headers, e.g. Ubuntu 24.04 libboost-dev)" >&2
  exit 1
}
boost="$("$probe/boost")" || { echo "Boost >= 1.81 is required" >&2; exit 1; }
compiler="$("$cxx" --version | head -1)"
echo "compiler: $compiler"
echo "dependency: $boost"

cli="${AMS_GRA_CLI:-}"
if [[ -z "$cli" ]]; then
  (cd "$repo" && cargo build --target-dir "$out/cargo" --locked --release -p ams-gra-codegen-oms --bin ams-gra-codegen-oms)
  cli="$out/cargo/release/ams-gra-codegen-oms"
fi

# ---- 2. production readiness ---------------------------------------------------
contract_path="examples/cpp-uci-client/contracts/uci$v-service.yaml"
service=cpp-uci-client messages=1 artifact="uci$v"
if ((with_rdma)); then
  contract_path="tests/fixtures/cpp-codec/addresses/uci$v-log-rdma-service.yaml"
  service=cpp-uci-client-log-rdma messages=2 artifact="uci$v-log-rdma"
  case "$v" in
    25) closure=55; emitted=54 ;;
    26) closure=57; emitted=56 ;;
  esac
fi
if ((with_survivability)); then
  contract_path="examples/cpp-uci-client/contracts/uci$v-log-rdma-survivability-service.yaml"
  service=cpp-uci-client-log-rdma-survivability messages=3 artifact="uci$v-log-rdma-survivability"
  case "$v" in
    25) closure=64; emitted=63 ;;
    26) closure=65; emitted=64 ;;
  esac
fi
contract="$repo/$contract_path"
common=(--schema "$schema" --contract "$contract" --language cpp --world closed-schema)
report="$("$cli" service-check "${common[@]}" --with-codec)"
for expected in "service: $service" "selected oms messages: $messages" "renderable selected oms messages: $messages" \
                "selected type closure: $closure" "renderable selected types: $closure" "status: READY" \
                "codec renderable emitted declarations: $emitted/$emitted" "codec status: READY"; do
  grep -qxF "$expected" <<<"$report" || { echo "$report" >&2; echo "service-check: missing '$expected'" >&2; exit 1; }
done
echo "UCI $release readiness: closure $closure/$closure model+service API READY, codec $emitted/$emitted READY"

# ---- 3. generation into external storage ---------------------------------------------
gen="$out/generated/$artifact"
rm -rf "$gen"
"$cli" service-generate "${common[@]}" --with-codec --output "$gen" >"$out/generate-$artifact.log"
for header in oam.hpp service_api.hpp service_codec.hpp; do
  [[ -s "$gen/$header" ]] || { echo "service-generate did not produce $header" >&2; exit 1; }
done
grep -qF 'inline constexpr std::string_view message_name = "Log";' "$gen/service_api.hpp" \
  || { echo "generated endpoint is not the global message Log" >&2; exit 1; }
grep -qF 'inline constexpr std::string_view topic = "uci.log";' "$gen/service_api.hpp" \
  || { echo "generated endpoint topic is not uci.log" >&2; exit 1; }
if grep -q 'namespace exchange_publish' "$gen/service_api.hpp"; then
  echo "the receive-only contract produced a publish endpoint" >&2; exit 1
fi

# ---- 4. strict C++17 compile -------------------------------------------------------------
obj="$out/obj/$artifact"
bin="$out/bin/cpp-uci-client-$artifact"
mkdir -p "$obj" "$out/bin"
runtime_include="$repo/cpp/oms-lacal-runtime/include"
"$cxx" "${flags[@]}" -I "$runtime_include" -c "$repo/cpp/oms-lacal-runtime/src/owp.cpp" -o "$obj/owp.o" &
p1=$!
"$cxx" "${flags[@]}" -I "$runtime_include" -c "$repo/cpp/oms-lacal-runtime/src/runtime.cpp" -o "$obj/runtime.o" &
p2=$!
status=0
"$cxx" "${flags[@]}" "-DCPP_UCI_CLIENT_UCI_RELEASE=$v" "-DCPP_UCI_CLIENT_SCHEMA_SHA256=\"$root_sha\"" \
  "-DCPP_UCI_CLIENT_WITH_RDMA=$with_rdma" "-DCPP_UCI_CLIENT_WITH_SURVIVABILITY=$with_survivability" \
  -I "$runtime_include" -I "$here/src" -I "$gen" -c "$here/src/main.cpp" -o "$obj/main.o" || status=$?
wait "$p1" || status=$?
wait "$p2" || status=$?
((status == 0)) || { echo "compilation failed" >&2; exit "$status"; }
"$cxx" "${flags[@]}" "$obj/main.o" "$obj/owp.o" "$obj/runtime.o" -o "$bin"

{
  echo "release=$release"
  echo "schema_version=$schema_version"
  echo "schema_root_sha256=$root_sha"
  echo "security_markings_sha256=$marks_sha"
  echo "contract=$contract_path"
  echo "contract_sha256=$(sha "$contract")"
  for header in oam.hpp service_api.hpp service_codec.hpp; do
    echo "generated_${header%.hpp}_sha256=$(sha "$gen/$header")"
  done
  echo "compiler=$compiler"
  echo "boost=$boost"
  echo "flags=${flags[*]}"
  echo "binary=$bin"
  echo "binary_sha256=$(sha "$bin")"
} > "$out/build-$artifact.manifest"
sed 's/^/  /' "$out/build-$artifact.manifest"
echo "BUILT cpp-uci-client for UCI $release ($schema_version): $bin"
