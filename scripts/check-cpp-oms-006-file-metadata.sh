#!/usr/bin/env bash
# CPP-OMS-006: qualify the generated C++17 structured-ASCII OMS JSON codec and
# record what it changes for the AUTHENTIC UCI FileMetadata and the other
# affected messages, for UCI 2.5 AND UCI 2.6. Not part of routine PR smoke
# (network: pinned UCI).
#
#   1. pinned UCI roots via the repository's fetch scripts (or
#      AMS_GRA_UCI_2_{5,6}_ROOT), root + SecurityMarkings SHA-256 checked
#      against scripts/task102/admission.py;
#   2. synthetic qualification in every configuration:
#      tests/cpp-codec/structured_ascii_consumer.cpp (shared Task 059 fixture
#      codec-structured-ascii) and tests/cpp-codec/structured_ascii_rows_consumer.cpp
#      (tests/fixtures/cpp-codec/structured-ascii, all 27 pinned rows), each
#      with an exact summary; the negative controls of
#      tests/cpp-codec/structured_ascii_mutations.py (all must be detected);
#   3. per release, AUTHENTIC FileMetadata: CPP-OMS-007 advances this regression
#      to require codec READY (50/50 and 52/52), generation with --with-codec,
#      and byte-identical model/API with and without the flag. The model probe
#      still checks the authentic FileNameType carrier. Independent codec and
#      real pinned-Sleet qualification live in check-cpp-oms-007-file-metadata.sh;
#      this script does not claim those behavioral results;
#   4. per release, the messages that become codec READY through structured
#      ASCII: exact readiness, model and service API byte-identical with and
#      without --with-codec, and a strict g++ COMPILE SMOKE of model + API +
#      codec whose generated decoder must refuse `{}` at the payload root.
#      A compile smoke is not a behavioral qualification;
#   5. compatibility impact for FileMetadata, AMTI_Command and
#      AO_CapabilityStatus (BEFORE too when a baseline CLI is given), and the
#      whole-release sweep (scripts/check-cpp-oms-006-codec-sweep.py) when
#      CPP_OMS_006_SWEEP=1.
#
# Configurations for step 2: strict g++, strict clang++, g++/clang++
# ASan+UBSan, g++/clang++ TSan. Anything that cannot run is printed as SKIPPED
# and is NOT claimed as a pass; missing clang++ or no sanitizer at all exits 3
# unless CPP_OMS_ALLOW_CLANG_SKIP=1 / CPP_OMS_ALLOW_SANITIZER_SKIP=1.
#
# Optional: CPP_OMS_006_BASELINE_CLI=<CLI built from the CPP-OMS-005 head
# 55469a20> also records the BEFORE readiness (FileMetadata NOT READY at
# FileNameType).
#
# Usage: scripts/check-cpp-oms-006-file-metadata.sh [scratch-dir]
set -euo pipefail
repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
scratch="$(realpath -m -- "${1:-$(mktemp -d)}")"
mkdir -p "$scratch"
cd "$repo"

ROOT_25=ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27
MARK_25=4a8056f1503234d423a0c2495844ab65387febced62bea58b87d4f343e9e7a0b
ROOT_26=af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b
MARK_26=ee6e58d5db9fd80d526bc964b8244ec296d106301640d34c3cb2c5c00b710b88
for pin in "$ROOT_25" "$MARK_25" "$ROOT_26" "$MARK_26"; do
  grep -q "$pin" scripts/task102/admission.py \
    || { echo "pin $pin is not in scripts/task102/admission.py" >&2; exit 1; }
done

# Exact expected summaries (a consumer that silently skipped or added checks
# must not qualify).
SYNTHETIC_SUMMARY="CPP_OMS_006_STRUCTURED_ASCII checks=323 rejections=166"
ROWS_SUMMARY="CPP_OMS_006_STRUCTURED_ROWS rows=27 checks=828 rejections=259"
MUTATION_SUMMARY="CPP_OMS_006_MUTATIONS detected=21/21"
declare -A FILE_CLOSURE=([25]=52 [26]=54)
declare -A FILE_CODEC=([25]="50/50" [26]="52/52")
declare -A FILE_BEFORE=([25]="48/50" [26]="50/52")
# message closure codec-after (both releases unless overridden below)
NEWLY_READY=(
  "AO_Capability 71 69/69"
  "CounterSpaceReport 55 54/54"
  "DMPI_DesignationRequest 56 54/54"
  "OrdersMetadata 82 80/80"
  "StrikeCapability 84 82/82"
  "StrikeCommand 77 74/74"
)
STRIKE_COMMAND_26="StrikeCommand 78 75/75"

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

# ---- toolchains ---------------------------------------------------------------------
cli="${AMS_GRA_CLI:-}"
if [[ -z "$cli" ]]; then
  cargo build --locked --release -p ams-gra-codegen-oms --bin ams-gra-codegen-oms
  cli="$repo/target/release/ams-gra-codegen-oms"
fi
echo "g++: $(g++ --version | head -1)"
command -v clang++ >/dev/null 2>&1 && echo "clang++: $(clang++ --version | head -1)"

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
  short="${compiler%%+*}"
  if [[ "$short" == g ]]; then short=gcc; fi
  if usable "$compiler" "${asan[@]}"; then
    configs+=("$short-asan-ubsan $compiler asan"); sanitized+=("$short-asan-ubsan")
  else
    skipped+=("ASan+UBSan unusable with $compiler")
  fi
  tflags=("${tsan[@]}")
  if [[ "$compiler" == g++ ]]; then tflags+=(-Wno-tsan); fi
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

# ---- 2. synthetic structured-ASCII qualification -----------------------------------
generate_synthetic() { # label schema contract expected-declarations
  local out="$scratch/gen-$1" report
  rm -rf "$out" "$out-plain"
  local common=(--schema "$2" --contract "$3" --language cpp --world closed-schema)
  report="$("$cli" service-check "${common[@]}" --with-codec)"
  expect "$report" "codec renderable emitted declarations: $4" "$1"
  expect "$report" "codec status: READY" "$1"
  "$cli" service-generate "${common[@]}" --with-codec --output "$out" >/dev/null
  "$cli" service-generate "${common[@]}" --output "$out-plain" >/dev/null
  for file in oam.hpp service_api.hpp; do cmp "$out/$file" "$out-plain/$file"; done
  test ! -e "$out-plain/service_codec.hpp"
  (cd "$out" && sha256sum oam.hpp service_api.hpp service_codec.hpp | sed "s/^/  sha256 $1 /")
}
echo "== structured-ASCII codec (synthetic fixtures, not UCI)"
generate_synthetic structured tests/fixtures/service-generate/codec-structured-ascii.xsd \
  tests/fixtures/service-generate/codec-structured-ascii.yaml 7/7
generate_synthetic rows tests/fixtures/cpp-codec/structured-ascii/structured-ascii.xsd \
  tests/fixtures/cpp-codec/structured-ascii/service.yaml 29/29
for entry in "${configs[@]}"; do
  read -r name compiler flavour <<<"$entry"
  flags_for "$flavour" "$compiler"
  out="$scratch/sa-$name"; mkdir -p "$out"
  "$compiler" "${flags[@]}" -I "$scratch/gen-structured" tests/cpp-codec/structured_ascii_consumer.cpp -o "$out/consumer"
  "$compiler" "${flags[@]}" -I "$scratch/gen-rows" tests/cpp-codec/structured_ascii_rows_consumer.cpp -o "$out/rows"
  summary="$(TSAN_OPTIONS=halt_on_error=1 "$out/consumer")"
  [[ "$summary" == "$SYNTHETIC_SUMMARY" ]] || { echo "$name: synthetic consumer '$summary'" >&2; exit 1; }
  rows="$(TSAN_OPTIONS=halt_on_error=1 "$out/rows")"
  [[ "$rows" == "$ROWS_SUMMARY" ]] || { echo "$name: rows consumer '$rows'" >&2; exit 1; }
  echo "  [$name] $summary"
  echo "  [$name] $rows"
done
mutation_status=0
mutations="$(python3 -I tests/cpp-codec/structured_ascii_mutations.py "$scratch/gen-structured" \
  tests/cpp-codec/structured_ascii_consumer.cpp "$scratch/mutants" 2>&1)" || mutation_status=$?
printf '%s\n' "$mutations" | sed 's/^/  /'
((mutation_status == 0)) || { echo "mutation controls failed (exit $mutation_status)" >&2; exit 1; }
[[ "$(tail -1 <<<"$mutations")" == "$MUTATION_SUMMARY" ]] || { echo "mutation controls incomplete" >&2; exit 1; }

# ---- contracts and readiness helpers -------------------------------------------------
measure_contract() { # message release out-file
  cat > "$3" <<EOF
contract_version: "0.1"
service:
  name: measure-$1
  version: "0.1.0"
  kind: service
standards:
  oms_version: "$2"
  uci_schema_version: "$2"
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
readiness_line() { # cli schema contract -> "closure N; model S; codec R/E S; first blocker"
  local report
  report="$("$1" service-check --schema "$2" --contract "$3" --language cpp --world closed-schema --with-codec 2>/dev/null || true)"
  printf 'closure %s; model %s; codec %s %s; first blocker: %s' \
    "$(sed -n 's/^selected type closure: //p' <<<"$report")" \
    "$(sed -n 's/^status: //p' <<<"$report")" \
    "$(sed -n 's/^codec renderable emitted declarations: //p' <<<"$report")" \
    "$(sed -n 's/^codec status: //p' <<<"$report")" \
    "$(sed -n 's/^service codec boundary: //p' <<<"$report" | sed 's/ (supported:.*//' | head -1)"
}

passed=()
for release in 2.5 2.6; do
  version="${release/./}"
  if [[ "$release" == 2.5 ]]; then root="$root25"; else root="$root26"; fi
  # ---- 3. authentic FileMetadata ------------------------------------------------------
  echo "== UCI $release FileMetadata"
  contract="$scratch/FileMetadata-$version.yaml"
  measure_contract FileMetadata "$release" "$contract"
  if [[ -n "${CPP_OMS_006_BASELINE_CLI:-}" ]]; then
    before="$("$CPP_OMS_006_BASELINE_CLI" service-check --schema "$root" --contract "$contract" --language cpp \
      --world closed-schema --with-codec 2>/dev/null || true)"
    expect "$before" "selected type closure: ${FILE_CLOSURE[$version]}" "baseline $release"
    expect "$before" "codec renderable emitted declarations: ${FILE_BEFORE[$version]}" "baseline $release"
    expect "$before" "codec status: NOT READY" "baseline $release"
    grep -q '^service codec boundary: FileNameType: named String profile structured ASCII has no C++ codec mapping' \
      <<<"$before" || { echo "baseline $release: unexpected blocker" >&2; echo "$before" >&2; exit 1; }
    echo "  BEFORE (baseline CLI): codec ${FILE_BEFORE[$version]} NOT READY at FileNameType (structured ASCII)"
  fi
  status=0
  report="$("$cli" service-check --schema "$root" --contract "$contract" --language cpp --world closed-schema \
    --with-codec 2>/dev/null)" || status=$?
  ((status == 0)) || { echo "$report" >&2; exit 1; }
  expect "$report" "selected type closure: ${FILE_CLOSURE[$version]}" "$release FileMetadata"
  expect "$report" "status: READY" "$release FileMetadata"
  expect "$report" "codec renderable emitted declarations: ${FILE_CODEC[$version]}" "$release FileMetadata"
  expect "$report" "codec status: READY" "$release FileMetadata"
  if grep -q FileNameType <<<"$report"; then echo "FileNameType still reported" >&2; exit 1; fi
  echo "  AFTER CPP-OMS-007: closure ${FILE_CLOSURE[$version]}, model READY, codec ${FILE_CODEC[$version]} READY"
  gen="$scratch/gen-file-metadata-$version"
  rm -rf "$gen" "$gen-codec"
  "$cli" service-generate --schema "$root" --contract "$contract" --language cpp --world closed-schema \
       --with-codec --output "$gen-codec" >/dev/null
  "$cli" service-generate --schema "$root" --contract "$contract" --language cpp --world closed-schema \
    --output "$gen" >/dev/null
  for file in oam.hpp service_api.hpp; do cmp "$gen/$file" "$gen-codec/$file"; done
  echo "  model and API byte-identical; authentic codec/Sleet qualification is in CPP-OMS-007"
  for entry in "${configs[@]}"; do
    read -r name compiler flavour <<<"$entry"
    [[ "$flavour" == tsan ]] && continue
    flags_for "$flavour" "$compiler"
    out="$scratch/fm-$version-$name"; mkdir -p "$out"
    "$compiler" "${flags[@]}" "-DUCI_FILE_METADATA_VERSION=$version" -I "$gen" \
      tests/cpp-codec/file_metadata_model_probe.cpp -o "$out/probe"
    summary="$("$out/probe")"
    [[ "$summary" == "CPP_OMS_006_FILE_METADATA_MODEL uci=$version checks=20" ]] \
      || { echo "UCI $release $name: model probe '$summary'" >&2; exit 1; }
    echo "  [$name] $summary"
  done
  echo "  FileMetadata codec/Sleet behavior is qualified separately by CPP-OMS-007; not claimed by this regression"

  # ---- 4. newly READY messages: compile smoke ---------------------------------------
  echo "== UCI $release messages made C++ codec READY by structured ASCII (compile smoke only)"
  for row in "${NEWLY_READY[@]}"; do
    if [[ "$release" == 2.6 && "$row" == StrikeCommand* ]]; then row="$STRIKE_COMMAND_26"; fi
    read -r message closure codec <<<"$row"
    contract="$scratch/$message-$version.yaml"
    measure_contract "$message" "$release" "$contract"
    if [[ -n "${CPP_OMS_006_BASELINE_CLI:-}" ]]; then
      before="$(readiness_line "$CPP_OMS_006_BASELINE_CLI" "$root" "$contract")"
      grep -q "codec .* NOT READY; first blocker: [A-Za-z_]*: named String profile structured ASCII" <<<"$before" \
        || { echo "baseline $release $message: $before" >&2; exit 1; }
    fi
    report="$("$cli" service-check --schema "$root" --contract "$contract" --language cpp --world closed-schema --with-codec)"
    expect "$report" "selected type closure: $closure" "$release $message"
    expect "$report" "status: READY" "$release $message"
    expect "$report" "codec renderable emitted declarations: $codec" "$release $message"
    expect "$report" "codec status: READY" "$release $message"
    out="$scratch/gen-$message-$version"
    rm -rf "$out" "$out-plain"
    common=(--schema "$root" --contract "$contract" --language cpp --world closed-schema)
    "$cli" service-generate "${common[@]}" --with-codec --output "$out" >/dev/null
    "$cli" service-generate "${common[@]}" --output "$out-plain" >/dev/null
    for file in oam.hpp service_api.hpp; do cmp "$out/$file" "$out-plain/$file"; done
    decoder="$(sed -n 's/^inline CodecResult<.*> \(decode_[A-Za-z0-9_]*\)(std::string_view json) {$/\1/p' \
      "$out/service_codec.hpp")"
    [[ "$(wc -l <<<"$decoder")" == 1 && -n "$decoder" ]] || { echo "$message: decoder '$decoder'" >&2; exit 1; }
    printf '#include "oam.hpp"\n#include "service_api.hpp"\n#include "service_codec.hpp"\n#include <iostream>\nint main() {\n    const auto bad = ::service_codec::%s("{}");\n    if (bad.ok() || bad.error().message.rfind("missing required member", 0) != 0) return 1;\n    std::cout << bad.error().path << "\\n";\n    return 0;\n}\n' \
      "$decoder" > "$out/probe.cpp"
    g++ "${strict[@]}" -O0 -I "$out" "$out/probe.cpp" -o "$out/probe"
    where="$("$out/probe")"
    echo "  $message: closure $closure, codec $codec READY, model/API byte-identical, strict g++ compile," \
         "{} refused at $where"
    passed+=("UCI $release $message compile smoke")
  done

  # ---- 5. compatibility impact ----------------------------------------------------------
  echo "== UCI $release compatibility impact (C++ --with-codec readiness)"
  for message in FileMetadata AMTI_Command AO_CapabilityStatus; do
    contract="$scratch/impact-$message-$version.yaml"
    measure_contract "$message" "$release" "$contract"
    if [[ -n "${CPP_OMS_006_BASELINE_CLI:-}" ]]; then
      echo "  $message BEFORE: $(readiness_line "$CPP_OMS_006_BASELINE_CLI" "$root" "$contract")"
    fi
    echo "  $message AFTER:  $(readiness_line "$cli" "$root" "$contract")"
  done
done

if [[ -n "${CPP_OMS_006_SWEEP:-}" ]]; then
  echo "== whole-release sweep"
  sweep=(python3 -I scripts/check-cpp-oms-006-codec-sweep.py --cli "$cli" --uci-2-5 "$root25" --uci-2-6 "$root26"
         --scratch "$scratch/sweep" --workers "${CPP_OMS_006_SWEEP_WORKERS:-2}")
  if [[ -n "${CPP_OMS_006_BASELINE_CLI:-}" ]]; then
    sweep+=(--baseline "$CPP_OMS_006_BASELINE_CLI" --require-no-loss)
  fi
  "${sweep[@]}" | sed 's/^/  /'
fi

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
  echo "CPP-OMS-006 structured ASCII: INCOMPLETE (skips above are not passes)" >&2
  exit 3
fi
echo "CPP-OMS-006 structured-ASCII regression: PASS for everything listed above;" \
     "FileMetadata is now READY; see CPP-OMS-007 for independent codec and Sleet results"
