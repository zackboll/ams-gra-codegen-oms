#!/usr/bin/env bash
# CPP-OMS-004: qualify the generated C++17 Float64 OMS JSON codec and the
# AUTHENTIC UCI PositionReport, codec and pinned-Sleet interoperability, for
# UCI 2.5 AND UCI 2.6. Not part of routine PR smoke (network: UCI + Sleet).
#
#   1. pinned UCI roots via the repository's fetch scripts (or
#      AMS_GRA_UCI_2_{5,6}_ROOT), root + SecurityMarkings SHA-256 checked
#      against scripts/task102/admission.py;
#   2. Sleet at the existing pin (94796daf, tree 47da595b), clean, built with
#      its own toolchain file, --locked (or an existing checkout, argument 2);
#   3. numerical qualification: the unchanged Task 096 exact oracle regenerates
#      the frozen corpus (hashes checked); the synthetic Float64 fixture's
#      generated codec decodes all 27,167 conversions and encodes all 20,474
#      bit patterns, every token checked by the oracle; edge suite; comma-
#      decimal locale run; mutation controls (deliberately broken copies of
#      the generated codec must fail);
#   4. per release: production `service-check --with-codec` must report
#      closure 60, model READY, codec 59/59 READY; `service-generate` with and
#      without the flag must give byte-identical model and service API
#      headers; tests/cpp-codec/position_report_consumer.cpp on the pinned
#      golden with an exact summary;
#   5. per release and configuration: tests/cpp-lacal/real_position_report_sleet.cpp
#      against its own freshly started Sleet loaded with that release's full
#      schema, every PASSED marker and the exact completion line required.
#
# Configurations: strict g++, strict clang++, g++ ASan+UBSan, g++ TSan (Sleet
# program), and clang++ ASan+UBSan / TSan when their runtimes exist. Anything
# that cannot run is printed as SKIPPED and is NOT claimed as a pass; missing
# clang++ or no sanitizer at all exits 3 unless CPP_OMS_ALLOW_CLANG_SKIP=1 /
# CPP_OMS_ALLOW_SANITIZER_SKIP=1.
#
# Optional: CPP_OMS_004_BASELINE_CLI=<CLI built from the pre-change main> also
# records the BEFORE readiness (codec 50/59 NOT READY at AccelerationType).
#
# Usage: scripts/check-cpp-oms-004-position-report.sh [scratch-dir] [existing-sleet-checkout]
set -euo pipefail
repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
scratch="$(realpath -m -- "${1:-$(mktemp -d)}")"
mkdir -p "$scratch"
cd "$repo"

SLEET_REPO=https://gitlab.com/zackboll/sleet.git
SLEET_REV=94796daf55ebd1134dce0925e6a204a05ad3423b
SLEET_TREE=47da595b9078cac1a9351e8290d1ae7f3976367f
ROOT_25=ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27
MARK_25=4a8056f1503234d423a0c2495844ab65387febced62bea58b87d4f343e9e7a0b
ROOT_26=af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b
MARK_26=ee6e58d5db9fd80d526bc964b8244ec296d106301640d34c3cb2c5c00b710b88
for pin in "$ROOT_25" "$MARK_25" "$ROOT_26" "$MARK_26"; do
  grep -q "$pin" scripts/task102/admission.py \
    || { echo "pin $pin is not in scripts/task102/admission.py" >&2; exit 1; }
done
grep -q "$SLEET_REV" scripts/run-real-sleet-test.sh \
  || { echo "Sleet pin differs from scripts/run-real-sleet-test.sh" >&2; exit 1; }

# Exact expected summaries (a consumer that silently skipped or added checks
# must not qualify).
EDGE_SUMMARY="CPP_OMS_004_FLOAT64 checks=305 rejections=103"
DECODE_SUMMARY="CPP_OMS_004_DECODE rows=27167 finite=27162 overflow=5 mismatches=0"
ENCODE_SUMMARY="CPP_OMS_004_ENCODE rows=20474 roundtrip_mismatches=0"
ORACLE_SUMMARY="CPP_OMS_004_ORACLE_ENCODING rows=20474 mismatches=0"
PR_SUMMARY_CHECKS=253
PR_SUMMARY_REJECTIONS=35

verify() { # file expected-sha256 label
  local actual
  actual="$(sha256sum "$1" | cut -d' ' -f1)"
  [[ "$actual" == "$2" ]] || { echo "$3 SHA-256 $actual != $2" >&2; exit 1; }
}
expect() { grep -qxF "$2" <<<"$1" || { echo "missing line ($3): $2" >&2; echo "$1" >&2; exit 1; }; }

# ---- 1. pinned UCI ------------------------------------------------------------
root25="${AMS_GRA_UCI_2_5_ROOT:-}"
root26="${AMS_GRA_UCI_2_6_ROOT:-}"
[[ -n "$root25" ]] || root25="$(scripts/fetch-pinned-uci-2.5.sh "$scratch/uci25-checkout")"
[[ -n "$root26" ]] || root26="$(scripts/fetch-pinned-uci-2.6.sh "$scratch/uci26-checkout")"
root25="$(realpath "$root25")"; root26="$(realpath "$root26")"
verify "$root25" "$ROOT_25" "UCI 2.5 MessageDefinitions"
verify "$(dirname "$root25")/UCI_SecurityMarkings_v2_5_0.xsd" "$MARK_25" "UCI 2.5 SecurityMarkings"
verify "$root26" "$ROOT_26" "UCI 2.6 MessageDefinitions"
verify "$(dirname "$root26")/UCI_SecurityMarkings_v2_6_0.xsd" "$MARK_26" "UCI 2.6 SecurityMarkings"
echo "pinned UCI roots and SecurityMarkings verified"

# ---- 2. pinned, unmodified Sleet --------------------------------------------------
checkout="${2:-$scratch/sleet}"
if [[ ! -d "$checkout/.git" ]]; then
  git clone --quiet "$SLEET_REPO" "$checkout"
fi
git -C "$checkout" checkout --quiet --detach "$SLEET_REV"
[[ "$(git -C "$checkout" rev-parse HEAD)" == "$SLEET_REV" ]]
[[ "$(git -C "$checkout" rev-parse 'HEAD^{tree}')" == "$SLEET_TREE" ]]
[[ -z "$(git -C "$checkout" status --porcelain)" ]] || { echo "Sleet checkout is modified" >&2; exit 1; }
(cd "$checkout" && CARGO_TARGET_DIR="$checkout/target" cargo build --release --locked -p sleet)
sleet_bin="$checkout/target/release/sleet"
echo "Sleet $SLEET_REV (tree $SLEET_TREE) clean; built with $(cd "$checkout" && rustc --version)"
[[ -z "$(git -C "$checkout" status --porcelain)" ]]
sha256sum "$sleet_bin"

# ---- toolchains ---------------------------------------------------------------------
cli="${AMS_GRA_CLI:-}"
if [[ -z "$cli" ]]; then
  cargo build --locked --release -p ams-gra-codegen-oms --bin ams-gra-codegen-oms
  cli="$repo/target/release/ams-gra-codegen-oms"
fi
echo "g++: $(g++ --version | head -1)"
command -v clang++ >/dev/null 2>&1 && echo "clang++: $(clang++ --version | head -1)"
printf '#include <boost/version.hpp>\n#include <cstdio>\nint main(){std::printf("Boost %%s\\n", BOOST_LIB_VERSION);}\n' > "$scratch/boost.cpp"
g++ -std=c++17 "$scratch/boost.cpp" -o "$scratch/boost" && "$scratch/boost"

strict=(-std=c++17 -Wall -Wextra -Wpedantic -Werror -Wconversion -Wsign-conversion
        -Wshadow -Wold-style-cast -Wcast-align -Wnon-virtual-dtor -Woverloaded-virtual
        -Wnull-dereference -Wdouble-promotion -Wformat=2)
asan=(-std=c++17 -g -fsanitize=address,undefined -fno-sanitize-recover=all -fno-omit-frame-pointer)
tsan=(-std=c++17 -g -fsanitize=thread)
printf 'int main() { return 0; }\n' > "$scratch/usable.cpp"
usable() { # compiler flags...
  local compiler="$1"; shift
  "$compiler" "$@" -pthread "$scratch/usable.cpp" -o "$scratch/usable" >/dev/null 2>&1 && "$scratch/usable"
}

skipped=()
configs=("gcc-strict g++ strict")  # name compiler flavour
if command -v clang++ >/dev/null 2>&1; then
  configs+=("clang-strict clang++ strict")
else
  skipped+=("clang++ not installed")
fi
sanitized=()
for compiler in g++ clang++; do
  command -v "$compiler" >/dev/null 2>&1 || continue
  short="${compiler%%+*}"; [[ "$short" == g ]] && short=gcc
  if usable "$compiler" "${asan[@]}"; then
    configs+=("$short-asan-ubsan $compiler asan"); sanitized+=("$short-asan-ubsan")
  else
    skipped+=("ASan+UBSan unusable with $compiler")
  fi
  tflags=("${tsan[@]}"); [[ "$compiler" == g++ ]] && tflags+=(-Wno-tsan)
  if usable "$compiler" "${tflags[@]}"; then
    configs+=("$short-tsan $compiler tsan"); sanitized+=("$short-tsan")
  else
    skipped+=("TSan unusable with $compiler")
  fi
done
flags_for() { # flavour compiler -> sets $flags
  case "$1" in
    strict) flags=("${strict[@]}" -O1) ;;
    asan) flags=("${asan[@]}" -O1) ;;
    tsan)
      flags=("${tsan[@]}" -O1)
      # An `if`, not `[[ ]] && ...`: as the function's last command a false
      # test would return 1 and stop the script under `set -e` for clang++.
      if [[ "$2" == g++ ]]; then flags+=(-Wno-tsan); fi
      ;;
  esac
}

# ---- 3. numerical qualification --------------------------------------------------------
echo "== Float64 numerical qualification (synthetic fixture, not UCI)"
python3 -I tests/cpp-codec/float64_oracle.py selftest
rm -rf "$scratch/corpus"
python3 -I tests/cpp-codec/float64_oracle.py corpus "$scratch/corpus"
corpus="$scratch/corpus/full"
f64="$scratch/gen-float64"
rm -rf "$f64"
"$cli" service-generate --schema tests/fixtures/cpp-codec/float64/float64.xsd \
  --contract tests/fixtures/cpp-codec/float64/service.yaml --language cpp --world closed-schema \
  --with-codec --output "$f64" >/dev/null
(cd "$f64" && sha256sum oam.hpp service_codec.hpp | sed 's/^/  sha256 float64 /')
for entry in "${configs[@]}"; do
  read -r name compiler flavour <<<"$entry"
  [[ "$flavour" == tsan ]] && continue  # single-threaded; TSan runs on the Sleet program
  flags_for "$flavour" "$compiler"
  out="$scratch/f64-$name"; mkdir -p "$out"
  "$compiler" "${flags[@]}" -I "$f64" tests/cpp-codec/float64_consumer.cpp -o "$out/consumer"
  [[ "$("$out/consumer")" == "$EDGE_SUMMARY" ]] || { echo "$name: edge suite" >&2; exit 1; }
  [[ "$("$out/consumer" decode "$corpus/conversions.tsv")" == "$DECODE_SUMMARY" ]] || { echo "$name: decode corpus" >&2; exit 1; }
  [[ "$("$out/consumer" encode "$corpus/encoding.tsv" "$out/tokens.tsv")" == "$ENCODE_SUMMARY" ]] || { echo "$name: encode corpus" >&2; exit 1; }
  [[ "$(python3 -I tests/cpp-codec/float64_oracle.py check-encoding "$out/tokens.tsv")" == "$ORACLE_SUMMARY" ]] \
    || { echo "$name: oracle rejected encoder tokens" >&2; exit 1; }
  echo "  $name: $EDGE_SUMMARY; $DECODE_SUMMARY; $ORACLE_SUMMARY"
done
# Locale independence: a comma-decimal C locale must not change anything.
locale_status=0
mkdir -p "$scratch/locales"
if command -v localedef >/dev/null 2>&1 && [[ -d /usr/share/i18n/locales ]] \
   && localedef -i de_DE -f UTF-8 "$scratch/locales/de_DE.UTF-8" >/dev/null 2>&1; then
  LOCPATH="$scratch/locales" "$scratch/f64-gcc-strict/consumer" locale || locale_status=$?
else
  "$scratch/f64-gcc-strict/consumer" locale || locale_status=$?
fi
case "$locale_status" in
  0) ;;
  77) skipped+=("comma-decimal locale run (no locale available)") ;;
  *) echo "locale run failed" >&2; exit 1 ;;
esac
# Mutation controls: each broken copy of the generated codec must be caught.
mutate() { # label old new
  local dir="$scratch/mutant-$1"
  rm -rf "$dir"; cp -r "$f64" "$dir"
  python3 -I - "$dir/service_codec.hpp" "$2" "$3" <<'EOF'
import sys
path, old, new = sys.argv[1:4]
text = open(path).read()
assert text.count(old) >= 1, old
open(path, "w").write(text.replace(old, new, 1))
EOF
  g++ -std=c++17 -O1 -I "$dir" tests/cpp-codec/float64_consumer.cpp -o "$dir/consumer"
  local caught=0
  "$dir/consumer" >/dev/null 2>&1 || caught=1
  "$dir/consumer" decode "$corpus/conversions.tsv" >/dev/null 2>&1 || caught=1
  if "$dir/consumer" encode "$corpus/encoding.tsv" "$dir/tokens.tsv" >/dev/null 2>&1; then
    python3 -I tests/cpp-codec/float64_oracle.py check-encoding "$dir/tokens.tsv" >/dev/null 2>&1 || caught=1
  else
    caught=1
  fi
  ((caught)) || { echo "mutation $1 was NOT detected" >&2; exit 1; }
  echo "  mutation control $1: detected"
}
mutate unsigned-underflow "return value.text[0] == '-' ? -0.0 : 0.0;" "return 0.0;"
mutate accept-overflow 'return fail(error, path, "number overflows binary64' \
  'return std::numeric_limits<double>::infinity(); return fail(error, path, "number overflows binary64'
mutate binary32-intermediate "std::from_chars(first, last, result, std::chars_format::general);" \
  "std::from_chars(first, last, result, std::chars_format::general); result = static_cast<double>(static_cast<float>(result));"
mutate fixed-precision "std::chars_format::scientific)" "std::chars_format::scientific, 16)"
mutate lowercase-special 'if (value.text == "NaN") {' 'if (value.text == "NaN" || value.text == "nan") {'
mutate no-encode-recheck "if (!Carrier::create(value.value())) {" "if (false) {"

# ---- 4/5. authentic PositionReport per release -------------------------------------------
free_port() { python3 -c 'import socket; s=socket.socket(); s.bind(("127.0.0.1",0)); print(s.getsockname()[1]); s.close()'; }
sleet_pid=""
sleet_dir=""
stop_sleet() {
  if [[ -n "$sleet_pid" ]]; then
    kill "$sleet_pid" 2>/dev/null || true
    wait "$sleet_pid" 2>/dev/null || true
    sleet_pid=""
  fi
  if [[ -n "$sleet_dir" ]]; then
    rm -rf "$sleet_dir"
    sleet_dir=""
  fi
}
trap stop_sleet EXIT
start_sleet() { # schema-root schema-version label
  sleet_dir="$scratch/sleet-run-$3"
  rm -rf "$sleet_dir"
  mkdir -p "$sleet_dir/services"
  cat > "$sleet_dir/services/service.toml" <<EOF
service_id = "position-report-loop"
service_uuid = "550e8400-e29b-41d4-a716-446655440004"
allowed_topics = ["mission.position-report"]
EOF
  local port
  port="$(free_port)"
  cat > "$sleet_dir/server.toml" <<EOF
bind_addr = "127.0.0.1:$port"
server_id = "cpp-oms-004"
system_label = "C++ generated UCI PositionReport"
schema_path = "$1"
schema_version = "$2"
services_dir = "$sleet_dir/services"
EOF
  RUST_LOG=warn "$sleet_bin" "$sleet_dir/server.toml" >"$sleet_dir/sleet.log" 2>&1 &
  sleet_pid=$!
  local i
  for ((i = 0; i < 600; i++)); do
    if (exec 3<>"/dev/tcp/127.0.0.1/$port") 2>/dev/null; then
      sleet_url="ws://127.0.0.1:$port"
      return 0
    fi
    kill -0 "$sleet_pid" 2>/dev/null || { cat "$sleet_dir/sleet.log" >&2; echo "Sleet exited" >&2; exit 1; }
    sleep 0.1
  done
  echo "Sleet did not listen within 60 s" >&2
  exit 1
}

contract=tests/fixtures/service-generate/position-report-loop.yaml
passed=()
for release in 2.5 2.6; do
  version="${release/./}"
  if [[ "$release" == 2.5 ]]; then
    root="$root25"; golden=tests/fixtures/oms-json/sleet-e38f61d8/PositionReport.json
  else
    root="$root26"; golden=tests/fixtures/python-whitespace-visible/goldens/2.6-PositionReport.json
  fi
  schema_version="002.${release#2.}.0"
  common=(--schema "$root" --contract "$contract" --language cpp --world closed-schema)
  echo "== UCI $release PositionReport"
  if [[ -n "${CPP_OMS_004_BASELINE_CLI:-}" ]]; then
    before="$("$CPP_OMS_004_BASELINE_CLI" service-check "${common[@]}" --with-codec 2>/dev/null || true)"
    expect "$before" "codec renderable emitted declarations: 50/59" "baseline $release"
    expect "$before" "codec status: NOT READY" "baseline $release"
    expect "$before" "service codec boundary: AccelerationType: named Float64 has no C++ codec mapping" "baseline $release"
    echo "  BEFORE (baseline CLI): codec 50/59 NOT READY at AccelerationType (named Float64)"
  fi
  report="$("$cli" service-check "${common[@]}" --with-codec)"
  expect "$report" "selected oms messages: 1" "$release"
  expect "$report" "selected type closure: 60" "$release"
  expect "$report" "renderable selected types: 60" "$release"
  expect "$report" "status: READY" "$release"
  expect "$report" "codec renderable emitted declarations: 59/59" "$release"
  expect "$report" "codec status: READY" "$release"
  echo "  AFTER: closure 60, model READY, codec 59/59 READY"
  gen="$scratch/gen-pr$version"
  rm -rf "$gen" "$gen-plain"
  "$cli" service-generate "${common[@]}" --with-codec --output "$gen" >/dev/null
  "$cli" service-generate "${common[@]}" --output "$gen-plain" >/dev/null
  for file in oam.hpp service_api.hpp; do cmp "$gen/$file" "$gen-plain/$file"; done
  test ! -e "$gen-plain/service_codec.hpp"
  echo "  model and service API byte-identical with and without --with-codec"
  (cd "$gen" && sha256sum oam.hpp service_api.hpp service_codec.hpp | sed 's/^/  sha256 /')
  grep -q 'message_name = "PositionReport";' "$gen/service_api.hpp"

  for entry in "${configs[@]}"; do
    read -r name compiler flavour <<<"$entry"
    flags_for "$flavour" "$compiler"
    out="$scratch/build-$version-$name"
    mkdir -p "$out"
    if [[ "$flavour" != tsan ]]; then
      "$compiler" "${flags[@]}" "-DUCI_PR_VERSION=$version" -I "$gen" -I tests/cpp-codec \
        tests/cpp-codec/position_report_consumer.cpp -o "$out/consumer"
      summary="$("$out/consumer" "$golden")"
      [[ "$summary" == "CPP_OMS_004_POSITION_REPORT uci=$version checks=$PR_SUMMARY_CHECKS rejections=$PR_SUMMARY_REJECTIONS" ]] \
        || { echo "UCI $release $name: consumer '$summary'" >&2; exit 1; }
      echo "  [$name] $summary"
    fi
    "$compiler" "${flags[@]}" -pthread -I cpp/oms-lacal-runtime/include -c cpp/oms-lacal-runtime/src/owp.cpp -o "$out/owp.o" &
    p1=$!
    "$compiler" "${flags[@]}" -pthread -I cpp/oms-lacal-runtime/include -c cpp/oms-lacal-runtime/src/runtime.cpp -o "$out/runtime.o" &
    p2=$!
    "$compiler" "${flags[@]}" -pthread "-DUCI_PR_VERSION=$version" -I cpp/oms-lacal-runtime/include \
      -I tests/cpp-lacal -I tests/cpp-codec -I "$gen" \
      -c tests/cpp-lacal/real_position_report_sleet.cpp -o "$out/test.o"
    wait "$p1"; wait "$p2"
    "$compiler" "${flags[@]}" -pthread "$out/test.o" "$out/owp.o" "$out/runtime.o" -o "$out/real_position_report_sleet"

    start_sleet "$root" "$schema_version" "$version-$name"
    status=0
    output="$(CPP_OMS_004_SLEET_URL="$sleet_url" CPP_OMS_004_GOLDEN="$golden" \
      ASAN_OPTIONS=detect_leaks=1:abort_on_error=1 TSAN_OPTIONS=halt_on_error=1:second_deadlock_stack=1 \
      timeout 900 "$out/real_position_report_sleet" 2>"$out/stderr")" || status=$?
    stop_sleet
    printf '%s\n' "$output" | sed "s/^/  [$name] /"
    if ((status != 0)); then
      cat "$out/stderr" >&2
      echo "UCI $release $name: exit $status" >&2
      exit 1
    fi
    for marker in "GOLDEN: PASSED" "typed MSG golden: PASSED" "typed MSG full Float64: PASSED" \
                  "typed MSG NaN/Infinity/-Infinity: PASSED" "raw pinned golden accepted by Sleet and decoded: PASSED" \
                  "raw non-canonical Float64 spellings: PASSED" "SLEET AND RECOVERY: PASSED"; do
      grep -qxF "CPP-OMS-004 $schema_version $marker" <<<"$output" \
        || { echo "UCI $release $name: missing marker '$marker'" >&2; exit 1; }
    done
    grep -qE "^CPP_OMS_004_REAL_POSITION_REPORT_COMPLETE uci=$version tests=2 checks=[0-9]+$" <<<"$output" \
      || { echo "UCI $release $name: no completion line" >&2; exit 1; }
    passed+=("UCI $release $name ($(grep -oE 'checks=[0-9]+' <<<"$output" | tail -1))")
  done
done

printf 'passed: %s\n' "${passed[@]}"
hard=0
if ((${#skipped[@]})); then
  printf 'SKIPPED (not claimed as a pass): %s\n' "${skipped[@]}"
  for item in "${skipped[@]}"; do
    case "$item" in clang++*) [[ -n "${CPP_OMS_ALLOW_CLANG_SKIP:-}" ]] || hard=1 ;; esac
  done
fi
if ((${#sanitized[@]} == 0)); then
  [[ -n "${CPP_OMS_ALLOW_SANITIZER_SKIP:-}" ]] || hard=1
  echo "SKIPPED (not claimed as a pass): no sanitizer configuration could run"
fi
printf 'sanitizers exercised: %s\n' "${sanitized[*]:-none}"
if ((hard)); then
  echo "CPP-OMS-004 PositionReport: INCOMPLETE (skips above are not passes)" >&2
  exit 3
fi
echo "CPP-OMS-004 Float64 codec and authentic PositionReport through pinned Sleet $SLEET_REV: PASS for everything listed above"
