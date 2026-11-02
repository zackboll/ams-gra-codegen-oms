#!/usr/bin/env bash
# CPP-OMS-003 (stages A, B, D): qualify the handwritten C++17 LA-CAL runtime
# (cpp/oms-lacal-runtime) offline, against the test-only controlled OWP peer,
# through GENERATED C++ service APIs and codecs.
#
#   1. checks the Boost.Beast dependency (headers only, Boost >= 1.81; the
#      exact version is printed and recorded);
#   2. generates the synthetic runtime-test (urn:test) and runtime-oam (OAM
#      namespace) services with the production CLI, --with-codec;
#   3. for every usable build configuration -- strict g++, strict clang++,
#      ASan+UBSan and TSan with each compiler -- compiles the runtime and the
#      four test programs and requires their EXACT summary lines:
#        CPP_OMS_003_OWP_UNIT_COMPLETE tests=6 checks=101
#        CPP_OMS_003_MOCK_COMPLETE tests=30 checks=364
#        CPP_OMS_003_SHUTDOWN_COMPLETE tests=12 checks=217
#        CPP_OMS_003_OAM_COMPLETE tests=1 checks=13
#
# Every configuration that cannot run is printed as SKIPPED and is NOT
# claimed. Exit 3 if clang++ is missing (unless CPP_OMS_ALLOW_CLANG_SKIP=1) or
# no sanitizer configuration ran (unless CPP_OMS_ALLOW_SANITIZER_SKIP=1).
# CPP_OMS_003_REPEAT=N reruns the mock and shutdown suites N times in every
# configuration.
#
# Usage: scripts/check-cpp-oms-003-runtime.sh [scratch-directory]
set -euo pipefail
repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
scratch="$(realpath -m -- "${1:-$(mktemp -d)}")"
mkdir -p "$scratch"
cd "$repo"
repeat="${CPP_OMS_003_REPEAT:-1}"

strict=(-std=c++17 -Wall -Wextra -Wpedantic -Werror -Wconversion -Wsign-conversion
        -Wshadow -Wold-style-cast -Wcast-align -Wnon-virtual-dtor -Woverloaded-virtual
        -Wnull-dereference -Wdouble-promotion -Wformat=2)
asan=(-std=c++17 -g -fsanitize=address,undefined -fno-sanitize-recover=all -fno-omit-frame-pointer)
tsan=(-std=c++17 -g -fsanitize=thread)

# ---- 1. dependency ---------------------------------------------------------
probe="$scratch/boost-probe"
mkdir -p "$probe"
cat > "$probe/probe.cpp" <<'EOF'
#include <boost/version.hpp>
#include <boost/beast/version.hpp>
#include <cstdio>
int main() { std::printf("Boost %d (%s), %s\n", BOOST_VERSION, BOOST_LIB_VERSION, BOOST_BEAST_VERSION_STRING); return BOOST_VERSION >= 108100 ? 0 : 1; }
EOF
if ! g++ -std=c++17 "$probe/probe.cpp" -o "$probe/probe" 2>"$probe/err"; then
  echo "Boost.Beast headers not found (install Boost >= 1.81 headers, e.g. Ubuntu 24.04 libboost-dev 1.83)" >&2
  cat "$probe/err" >&2
  exit 1
fi
"$probe/probe" || { echo "Boost >= 1.81 is required" >&2; exit 1; }
if command -v dpkg-query >/dev/null 2>&1; then
  dpkg-query -W -f='${db:Status-Abbrev} ${Package} ${Version}\n' 'libboost*-dev' 2>/dev/null \
    | awk '$1 == "ii" { print "  package " $2 " " $3 }' || true
fi

# ---- 2. generated synthetic services ---------------------------------------
cli="${AMS_GRA_CLI:-}"
if [[ -z "$cli" ]]; then
  cargo build --locked --release -p ams-gra-codegen-oms --bin ams-gra-codegen-oms
  cli="$repo/target/release/ams-gra-codegen-oms"
fi
gen() { # fixture output
  rm -rf "$2"
  "$cli" service-generate --schema "tests/fixtures/service-generate/$1.xsd" \
    --contract "tests/fixtures/service-generate/$1.yaml" --language cpp \
    --world closed-schema --with-codec --output "$2" >/dev/null
  test -s "$2/service_api.hpp" && test -s "$2/service_codec.hpp"
}
gen runtime-test "$scratch/gen-runtime-test"
gen runtime-oam "$scratch/gen-runtime-oam"
echo "generated runtime-test and runtime-oam C++ services (with codec)"

# ---- 3. build and run every configuration ------------------------------------
runtime_include=cpp/oms-lacal-runtime/include
skipped=()
ran=()
sanitized=()

usable() { # compiler flags...
  local compiler="$1"; shift
  printf 'int main() { return 0; }\n' > "$probe/usable.cpp"
  "$compiler" "$@" -pthread "$probe/usable.cpp" -o "$probe/usable" >/dev/null 2>&1 && "$probe/usable"
}

expect_line() { # binary expected-line [env...]
  local binary="$1" expected="$2"; shift 2
  local out status=0
  out="$(env "$@" timeout 600 "$binary" 2>"$binary.stderr")" || status=$?
  if [[ $status -ne 0 || "$out" != "$expected" ]]; then
    echo "$(basename "$binary"): exit $status, output '$out' != '$expected'" >&2
    tail -n 40 "$binary.stderr" >&2
    exit 1
  fi
}

run_config() { # name compiler flags...
  local name="$1" compiler="$2"; shift 2
  local out="$scratch/build-$name"
  mkdir -p "$out"
  local pids=()
  "$compiler" "$@" -O1 -pthread -I "$runtime_include" -c cpp/oms-lacal-runtime/src/owp.cpp -o "$out/owp.o" & pids+=($!)
  "$compiler" "$@" -O1 -pthread -I "$runtime_include" -c cpp/oms-lacal-runtime/src/runtime.cpp -o "$out/runtime.o" & pids+=($!)
  "$compiler" "$@" -O1 -pthread -I "$runtime_include" -c tests/cpp-lacal/owp_unit_test.cpp -o "$out/owp_unit.o" & pids+=($!)
  "$compiler" "$@" -O1 -pthread -I "$runtime_include" -I "$scratch/gen-runtime-test" \
    -c tests/cpp-lacal/runtime_mock_test.cpp -o "$out/mock.o" & pids+=($!)
  "$compiler" "$@" -O1 -pthread -I "$runtime_include" -I "$scratch/gen-runtime-test" \
    -c tests/cpp-lacal/runtime_shutdown_test.cpp -o "$out/shutdown.o" & pids+=($!)
  "$compiler" "$@" -O1 -pthread -I "$runtime_include" -I "$scratch/gen-runtime-oam" \
    -c tests/cpp-lacal/runtime_oam_test.cpp -o "$out/oam.o" & pids+=($!)
  local pid
  for pid in "${pids[@]}"; do
    wait "$pid" || { echo "$name: compilation failed" >&2; exit 1; }
  done
  "$compiler" "$@" -pthread "$out/owp_unit.o" "$out/owp.o" -o "$out/owp_unit"
  "$compiler" "$@" -pthread "$out/mock.o" "$out/owp.o" "$out/runtime.o" -o "$out/mock"
  "$compiler" "$@" -pthread "$out/shutdown.o" "$out/owp.o" "$out/runtime.o" -o "$out/shutdown"
  "$compiler" "$@" -pthread "$out/oam.o" "$out/owp.o" "$out/runtime.o" -o "$out/oam"
  local env=(ASAN_OPTIONS=detect_leaks=1:abort_on_error=1 UBSAN_OPTIONS=print_stacktrace=1
             TSAN_OPTIONS=halt_on_error=1:second_deadlock_stack=1)
  expect_line "$out/owp_unit" "CPP_OMS_003_OWP_UNIT_COMPLETE tests=6 checks=101" "${env[@]}"
  local i
  for ((i = 0; i < repeat; i++)); do
    expect_line "$out/mock" "CPP_OMS_003_MOCK_COMPLETE tests=30 checks=364" "${env[@]}"
    expect_line "$out/shutdown" "CPP_OMS_003_SHUTDOWN_COMPLETE tests=12 checks=217" "${env[@]}"
  done
  expect_line "$out/oam" "CPP_OMS_003_OAM_COMPLETE tests=1 checks=13" "${env[@]}"
  echo "  $name: owp_unit 6/101, mock 30/364 (x$repeat), shutdown 12/217 (x$repeat), oam 1/13 PASS"
  ran+=("$name")
}

for compiler in g++ clang++; do
  if ! command -v "$compiler" >/dev/null 2>&1; then
    [[ "$compiler" == clang++ ]] || { echo "g++ is required" >&2; exit 1; }
    skipped+=("clang++ not installed")
    continue
  fi
  case "$compiler" in g++) short=gcc ;; *) short=clang ;; esac
  run_config "$short-strict" "$compiler" "${strict[@]}"
  if usable "$compiler" "${asan[@]}"; then
    run_config "$short-asan-ubsan" "$compiler" "${asan[@]}"
    sanitized+=("$short-asan-ubsan")
  else
    skipped+=("ASan/UBSan unusable with $compiler")
  fi
  # GCC warns (-Wtsan) that Asio's std::atomic_thread_fence is not modelled;
  # the runtime's own cross-thread state is mutex/condition-variable based.
  tsan_flags=("${tsan[@]}")
  [[ "$compiler" == g++ ]] && tsan_flags+=(-Wno-tsan)
  if usable "$compiler" "${tsan_flags[@]}"; then
    run_config "$short-tsan" "$compiler" "${tsan_flags[@]}"
    sanitized+=("$short-tsan")
  else
    skipped+=("TSan unusable with $compiler")
  fi
done

hard=0
for item in "${skipped[@]+"${skipped[@]}"}"; do
  echo "SKIPPED (not claimed as a pass): $item"
  case "$item" in clang++*) [[ -n "${CPP_OMS_ALLOW_CLANG_SKIP:-}" ]] || hard=1 ;; esac
done
if ((${#sanitized[@]} == 0)); then
  [[ -n "${CPP_OMS_ALLOW_SANITIZER_SKIP:-}" ]] || hard=1
  echo "SKIPPED (not claimed as a pass): no sanitizer configuration could run"
fi
echo "configurations passed: ${ran[*]}"
if ((hard)); then
  echo "CPP-OMS-003 runtime: INCOMPLETE (skips above are not passes)" >&2
  exit 3
fi
echo "CPP-OMS-003 runtime: PASS for every configuration listed above"
