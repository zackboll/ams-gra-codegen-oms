#!/usr/bin/env bash
# CPP-OMS-005: qualify the generated C++17 bounded-ASCII OMS JSON codec and the
# AUTHENTIC UCI AMTI_SettingsCommand, codec and pinned-Sleet interoperability,
# for UCI 2.5 AND UCI 2.6. Not part of routine PR smoke (network: UCI + Sleet).
#
#   1. pinned UCI roots via the repository's fetch scripts (or
#      AMS_GRA_UCI_2_{5,6}_ROOT), root + SecurityMarkings SHA-256 checked
#      against scripts/task102/admission.py;
#   2. Sleet at the existing pin (e38f61d8, tree 7e705f4e), clean, built with
#      its own toolchain file, --locked (or an existing checkout, argument 2);
#   3. synthetic qualification (tests/fixtures/cpp-codec/bounded-ascii):
#      tests/cpp-codec/bounded_ascii_consumer.cpp with an exact summary in every
#      configuration, and the negative controls of
#      tests/cpp-codec/bounded_ascii_mutations.py (all must be detected);
#   4. per release: production `service-check --with-codec` must report
#      closure 61, model READY, codec 59/59 READY for both the production
#      contract (real-bounded-ascii-amti-settings.yaml) and the test-only loop
#      contract (amti-settings-loop.yaml); `service-generate` with and without
#      the flag must give byte-identical model and service API headers;
#      tests/cpp-codec/amti_settings_consumer.cpp with an exact summary;
#   5. per release and configuration: tests/cpp-lacal/real_amti_settings_sleet.cpp
#      against its own freshly started Sleet loaded with that release's full
#      schema, every PASSED marker and the completion line required;
#   6. compatibility impact: AMTI_Command and AO_CapabilityStatus readiness for
#      both releases (and the BEFORE values when a baseline CLI is given).
#
# Configurations: strict g++, strict clang++, g++/clang++ ASan+UBSan, g++/clang++
# TSan (Sleet program). Anything that cannot run is printed as SKIPPED and is
# NOT claimed as a pass; missing clang++ or no sanitizer at all exits 3 unless
# CPP_OMS_ALLOW_CLANG_SKIP=1 / CPP_OMS_ALLOW_SANITIZER_SKIP=1.
#
# Optional: CPP_OMS_005_BASELINE_CLI=<CLI built from the pre-change main
# afe0c510> also records the BEFORE readiness (codec 58/59 NOT READY at
# EmptyType for AMTI_SettingsCommand).
#
# Usage: scripts/check-cpp-oms-005-amti-settings.sh [scratch-dir] [existing-sleet-checkout]
set -euo pipefail
repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
scratch="$(realpath -m -- "${1:-$(mktemp -d)}")"
mkdir -p "$scratch"
cd "$repo"

SLEET_REPO=https://github.com/open-arsenal/ams-gra-hello-world-sk-infra-sleet
SLEET_REV=e38f61d8ce0d75c8508434a52f2ed77c69cf6a3b
SLEET_TREE=7e705f4eb5f658957bd414b4abc8207311f3c1e4
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
SYNTHETIC_SUMMARY="CPP_OMS_005_BOUNDED_ASCII checks=289 rejections=148"
MUTATION_SUMMARY="CPP_OMS_005_MUTATIONS detected=13/13"
declare -A AMTI_SUMMARY=(
  [25]="CPP_OMS_005_AMTI_SETTINGS uci=25 checks=68 rejections=47"
  [26]="CPP_OMS_005_AMTI_SETTINGS uci=26 checks=70 rejections=46"
)

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
(cd "$checkout" && cargo build --release --locked -p sleet)
sleet_bin="$checkout/target/release/sleet"
echo "Sleet $SLEET_REV (tree $SLEET_TREE) clean; built with $(cd "$checkout" && rustc --version)"

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
      if [[ "$2" == g++ ]]; then flags+=(-Wno-tsan); fi
      ;;
  esac
}

# ---- 3. synthetic bounded-ASCII qualification ------------------------------------------
echo "== bounded-ASCII codec (synthetic fixture, not UCI)"
ba="$scratch/gen-bounded-ascii"
rm -rf "$ba" "$ba-plain"
ba_common=(--schema tests/fixtures/cpp-codec/bounded-ascii/bounded-ascii.xsd
           --contract tests/fixtures/cpp-codec/bounded-ascii/service.yaml --language cpp --world closed-schema)
ba_report="$("$cli" service-check "${ba_common[@]}" --with-codec)"
expect "$ba_report" "codec renderable emitted declarations: 15/15" "synthetic"
expect "$ba_report" "codec status: READY" "synthetic"
"$cli" service-generate "${ba_common[@]}" --with-codec --output "$ba" >/dev/null
"$cli" service-generate "${ba_common[@]}" --output "$ba-plain" >/dev/null
for file in oam.hpp service_api.hpp; do cmp "$ba/$file" "$ba-plain/$file"; done
(cd "$ba" && sha256sum oam.hpp service_api.hpp service_codec.hpp | sed 's/^/  sha256 bounded-ascii /')
for entry in "${configs[@]}"; do
  read -r name compiler flavour <<<"$entry"
  flags_for "$flavour" "$compiler"
  out="$scratch/ba-$name"; mkdir -p "$out"
  "$compiler" "${flags[@]}" -I "$ba" tests/cpp-codec/bounded_ascii_consumer.cpp -o "$out/consumer"
  summary="$(TSAN_OPTIONS=halt_on_error=1 "$out/consumer")"
  [[ "$summary" == "$SYNTHETIC_SUMMARY" ]] || { echo "$name: synthetic consumer '$summary'" >&2; exit 1; }
  echo "  [$name] $summary"
done
mutation_status=0
mutations="$(python3 -I tests/cpp-codec/bounded_ascii_mutations.py "$ba" tests/cpp-codec/bounded_ascii_consumer.cpp \
  "$scratch/mutants" 2>&1)" || mutation_status=$?
printf '%s\n' "$mutations" | sed 's/^/  /'
((mutation_status == 0)) || { echo "mutation controls failed (exit $mutation_status)" >&2; exit 1; }
[[ "$(tail -1 <<<"$mutations")" == "$MUTATION_SUMMARY" ]] || { echo "mutation controls incomplete" >&2; exit 1; }

# ---- 4/5. authentic AMTI_SettingsCommand per release ----------------------------------------
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
service_id = "amti-settings-loop"
service_uuid = "550e8400-e29b-41d4-a716-446655440005"
allowed_topics = ["amti.settings"]
EOF
  local port
  port="$(free_port)"
  cat > "$sleet_dir/server.toml" <<EOF
bind_addr = "127.0.0.1:$port"
server_id = "cpp-oms-005"
system_label = "C++ generated UCI AMTI_SettingsCommand"
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

measure_contract() { # message out-file
  cat > "$2" <<EOF
contract_version: "0.1"
service:
  name: measure-$1
  version: "0.1.0"
  kind: service
standards:
  oms_version: "2.5"
  uci_schema_version: "2.5"
functions:
  - id: f
    name: F
    category: specific
    applicability: applicable
    exchanges:
      - id: out
        kind: oms_message
        direction: output
        mandate: mandatory
        message: $1
        topic: t
        timing:
          kind: asynchronous
EOF
}
readiness_line() { # cli schema contract -> "closure N; model S; codec R/E S; blocker"
  local report
  report="$("$1" service-check --schema "$2" --contract "$3" --language cpp --world closed-schema --with-codec 2>/dev/null || true)"
  printf 'closure %s; model %s; codec %s %s; first blocker: %s' \
    "$(sed -n 's/^selected type closure: //p' <<<"$report")" \
    "$(sed -n 's/^status: //p' <<<"$report")" \
    "$(sed -n 's/^codec renderable emitted declarations: //p' <<<"$report")" \
    "$(sed -n 's/^codec status: //p' <<<"$report")" \
    "$(sed -n 's/^service codec boundary: //p' <<<"$report" | head -1)"
}

loop=tests/fixtures/service-generate/amti-settings-loop.yaml
production=tests/fixtures/service-generate/real-bounded-ascii-amti-settings.yaml
passed=()
for release in 2.5 2.6; do
  version="${release/./}"
  if [[ "$release" == 2.5 ]]; then root="$root25"; else root="$root26"; fi
  schema_version="002.${release#2.}.0"
  echo "== UCI $release AMTI_SettingsCommand"
  if [[ -n "${CPP_OMS_005_BASELINE_CLI:-}" ]]; then
    before="$("$CPP_OMS_005_BASELINE_CLI" service-check --schema "$root" --contract "$production" --language cpp \
      --world closed-schema --with-codec 2>/dev/null || true)"
    expect "$before" "selected type closure: 61" "baseline $release"
    expect "$before" "codec renderable emitted declarations: 58/59" "baseline $release"
    expect "$before" "codec status: NOT READY" "baseline $release"
    grep -q '^service codec boundary: EmptyType: named String profile bounded ASCII has no C++ codec mapping' <<<"$before" \
      || { echo "baseline $release: unexpected blocker" >&2; echo "$before" >&2; exit 1; }
    echo "  BEFORE (baseline CLI): codec 58/59 NOT READY at EmptyType (bounded ASCII)"
  fi
  for contract in "$production" "$loop"; do
    report="$("$cli" service-check --schema "$root" --contract "$contract" --language cpp --world closed-schema --with-codec)"
    expect "$report" "selected oms messages: 1" "$release $contract"
    expect "$report" "selected type closure: 61" "$release $contract"
    expect "$report" "renderable selected types: 61" "$release $contract"
    expect "$report" "status: READY" "$release $contract"
    expect "$report" "codec renderable emitted declarations: 59/59" "$release $contract"
    expect "$report" "codec status: READY" "$release $contract"
  done
  echo "  AFTER: closure 61, model READY, codec 59/59 READY (production and loop contracts)"
  common=(--schema "$root" --contract "$loop" --language cpp --world closed-schema)
  gen="$scratch/gen-amti$version"
  rm -rf "$gen" "$gen-plain"
  "$cli" service-generate "${common[@]}" --with-codec --output "$gen" >/dev/null
  "$cli" service-generate "${common[@]}" --output "$gen-plain" >/dev/null
  for file in oam.hpp service_api.hpp; do cmp "$gen/$file" "$gen-plain/$file"; done
  test ! -e "$gen-plain/service_codec.hpp"
  echo "  model and service API byte-identical with and without --with-codec"
  (cd "$gen" && sha256sum oam.hpp service_api.hpp service_codec.hpp | sed 's/^/  sha256 /')
  grep -q 'message_name = "AMTI_SettingsCommand";' "$gen/service_api.hpp"

  for entry in "${configs[@]}"; do
    read -r name compiler flavour <<<"$entry"
    flags_for "$flavour" "$compiler"
    out="$scratch/build-$version-$name"
    mkdir -p "$out"
    if [[ "$flavour" != tsan ]]; then
      "$compiler" "${flags[@]}" "-DUCI_AMTI_VERSION=$version" -I "$gen" -I tests/cpp-codec \
        tests/cpp-codec/amti_settings_consumer.cpp -o "$out/consumer"
      summary="$("$out/consumer")"
      [[ "$summary" == "${AMTI_SUMMARY[$version]}" ]] || { echo "UCI $release $name: consumer '$summary'" >&2; exit 1; }
      echo "  [$name] $summary"
    fi
    "$compiler" "${flags[@]}" -pthread -I cpp/oms-lacal-runtime/include -c cpp/oms-lacal-runtime/src/owp.cpp -o "$out/owp.o" &
    p1=$!
    "$compiler" "${flags[@]}" -pthread -I cpp/oms-lacal-runtime/include -c cpp/oms-lacal-runtime/src/runtime.cpp -o "$out/runtime.o" &
    p2=$!
    "$compiler" "${flags[@]}" -pthread "-DUCI_AMTI_VERSION=$version" -I cpp/oms-lacal-runtime/include \
      -I tests/cpp-lacal -I tests/cpp-codec -I "$gen" \
      -c tests/cpp-lacal/real_amti_settings_sleet.cpp -o "$out/test.o"
    wait "$p1"; wait "$p2"
    "$compiler" "${flags[@]}" -pthread "$out/test.o" "$out/owp.o" "$out/runtime.o" -o "$out/real_amti_settings_sleet"

    start_sleet "$root" "$schema_version" "$version-$name"
    status=0
    output="$(CPP_OMS_005_SLEET_URL="$sleet_url" \
      ASAN_OPTIONS=detect_leaks=1:abort_on_error=1 TSAN_OPTIONS=halt_on_error=1:second_deadlock_stack=1 \
      timeout 900 "$out/real_amti_settings_sleet" 2>"$out/stderr")" || status=$?
    stop_sleet
    printf '%s\n' "$output" | sed "s/^/  [$name] /"
    if ((status != 0)); then
      cat "$out/stderr" >&2
      echo "UCI $release $name: exit $status" >&2
      exit 1
    fi
    for marker in 'DOCUMENTS: PASSED' 'typed MSG UnassignAll present "": PASSED' 'typed MSG UnassignAll absent: PASSED' \
                  'typed MSG full MessageData: PASSED' 'raw reordered UnassignAll "" accepted by Sleet and decoded: PASSED' \
                  'raw UnassignAll absent accepted by Sleet and decoded as absent: PASSED' 'SLEET AND RECOVERY: PASSED'; do
      grep -qxF "CPP-OMS-005 $schema_version $marker" <<<"$output" \
        || { echo "UCI $release $name: missing marker '$marker'" >&2; exit 1; }
    done
    grep -qE "^CPP_OMS_005_REAL_AMTI_SETTINGS_COMPLETE uci=$version tests=2 checks=[0-9]+$" <<<"$output" \
      || { echo "UCI $release $name: no completion line" >&2; exit 1; }
    passed+=("UCI $release $name ($(grep -oE 'checks=[0-9]+' <<<"$output" | tail -1))")
  done
done

# ---- 6. compatibility impact ----------------------------------------------------------------
echo "== compatibility impact (C++ --with-codec readiness)"
for message in AMTI_SettingsCommand AMTI_Command AO_CapabilityStatus; do
  measure_contract "$message" "$scratch/measure-$message.yaml"
  for release in 2.5 2.6; do
    if [[ "$release" == 2.5 ]]; then root="$root25"; else root="$root26"; fi
    if [[ -n "${CPP_OMS_005_BASELINE_CLI:-}" ]]; then
      echo "  $message UCI $release BEFORE: $(readiness_line "$CPP_OMS_005_BASELINE_CLI" "$root" "$scratch/measure-$message.yaml")"
    fi
    echo "  $message UCI $release AFTER:  $(readiness_line "$cli" "$root" "$scratch/measure-$message.yaml")"
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
  echo "CPP-OMS-005 AMTI_SettingsCommand: INCOMPLETE (skips above are not passes)" >&2
  exit 3
fi
echo "CPP-OMS-005 bounded-ASCII codec and authentic AMTI_SettingsCommand through pinned Sleet $SLEET_REV: PASS for everything listed above"
