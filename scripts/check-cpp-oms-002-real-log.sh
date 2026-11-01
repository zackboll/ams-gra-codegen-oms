#!/usr/bin/env bash
# CPP-OMS-002: qualify the generated C++17 OMS JSON codec against the AUTHENTIC
# pinned UCI `Log` message in BOTH UCI 2.5 and UCI 2.6.
#
# For each release this script
#   1. obtains the pinned root through the repository's own fetch script (or
#      the caller's AMS_GRA_UCI_2_{5,6}_ROOT) and verifies the root AND the
#      imported SecurityMarkings SHA-256;
#   2. runs the production `service-check --language cpp --with-codec` on the
#      unchanged Task 057/093/102 Log contract and requires the exact closure
#      and codec readiness numbers;
#   3. runs the production `service-generate` with and without `--with-codec`
#      and requires the model and service API headers to be byte-identical;
#   4. compiles tests/cpp-codec/log_consumer.cpp against the generated headers
#      under strict C++17 flags with g++ and clang++ (when installed) and under
#      ASan+UBSan, and runs it on the existing Task 093 golden JSON.
#
# Sanitizers run "where supported": every compiler/sanitizer combination that
# cannot run is printed as SKIPPED and is NOT claimed as a pass. The script
# exits 3 if clang++ is missing (unless CPP_OMS_ALLOW_CLANG_SKIP=1) or if NO
# sanitizer ran for a release (unless CPP_OMS_ALLOW_SANITIZER_SKIP=1); the final
# line always lists exactly what was and was not exercised.
#
# Usage: scripts/check-cpp-oms-002-real-log.sh [scratch-directory]
# Inputs/checkouts stay OUTSIDE the repository (the fetch scripts refuse
# otherwise); nothing from UCI is written into it.
set -euo pipefail
repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
scratch="$(realpath -m -- "${1:-$(mktemp -d)}")"
mkdir -p "$scratch"
cd "$repo"

# Pinned identities (same values as scripts/task102/admission.py; cross-checked
# below so this file cannot silently drift from it).
ROOT_25=ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27
MARK_25=4a8056f1503234d423a0c2495844ab65387febced62bea58b87d4f343e9e7a0b
ROOT_26=af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b
MARK_26=ee6e58d5db9fd80d526bc964b8244ec296d106301640d34c3cb2c5c00b710b88
for pin in "$ROOT_25" "$MARK_25" "$ROOT_26" "$MARK_26"; do
  grep -q "$pin" scripts/task102/admission.py \
    || { echo "pin $pin is not in scripts/task102/admission.py" >&2; exit 1; }
done

verify() { # file expected-sha256 label
  local actual
  actual="$(sha256sum "$1" | cut -d' ' -f1)"
  [[ "$actual" == "$2" ]] || { echo "$3 SHA-256 $actual != $2" >&2; exit 1; }
}

root25="${AMS_GRA_UCI_2_5_ROOT:-}"
root26="${AMS_GRA_UCI_2_6_ROOT:-}"
[[ -n "$root25" ]] || root25="$(scripts/fetch-pinned-uci-2.5.sh "$scratch/uci25-checkout")"
[[ -n "$root26" ]] || root26="$(scripts/fetch-pinned-uci-2.6.sh "$scratch/uci26-checkout")"
verify "$root25" "$ROOT_25" "UCI 2.5 MessageDefinitions"
verify "$(dirname "$root25")/UCI_SecurityMarkings_v2_5_0.xsd" "$MARK_25" "UCI 2.5 SecurityMarkings"
verify "$root26" "$ROOT_26" "UCI 2.6 MessageDefinitions"
verify "$(dirname "$root26")/UCI_SecurityMarkings_v2_6_0.xsd" "$MARK_26" "UCI 2.6 SecurityMarkings"
echo "pinned roots and SecurityMarkings verified"

cargo build --locked --release -p ams-gra-codegen-oms --bin ams-gra-codegen-oms
cli="$repo/target/release/ams-gra-codegen-oms"
contract=tests/fixtures/service-generate/real-duration-log.yaml

strict=(-std=c++17 -Wall -Wextra -Wpedantic -Werror -Wconversion -Wsign-conversion
        -Wshadow -Wold-style-cast -Wcast-align -Wnon-virtual-dtor -Woverloaded-virtual
        -Wnull-dereference -Wdouble-promotion -Wformat=2)
sanitize=(-std=c++17 -O1 -g -fsanitize=address,undefined -fno-sanitize-recover=all)

skipped=()
sanitized=()
sanitizer_ok() { # compiler
  local probe="$scratch/probe-$1"
  mkdir -p "$probe"
  printf 'int main() { return 0; }\n' > "$probe/probe.cpp"
  "$1" "${sanitize[@]}" "$probe/probe.cpp" -o "$probe/probe" >/dev/null 2>&1 && "$probe/probe"
}

run_consumer() { # label compiler version include-dir flags...
  local label="$1" compiler="$2" version="$3" include="$4"
  shift 4
  local binary="$scratch/log-consumer-$label"
  "$compiler" "$@" "-DUCI_LOG_VERSION=$version" -I "$include" \
    tests/cpp-codec/log_consumer.cpp -o "$binary"
  local out
  out="$("$binary" "tests/fixtures/service-generate/task093-real-log$version.json")"
  [[ "$out" == CPP_OMS_002_LOG_COMPLETE\ uci=$version\ checks=* ]] \
    || { echo "$label: unexpected consumer output: $out" >&2; exit 1; }
  echo "  $label: $out"
}

expect() { # report pattern label
  grep -qxF "$2" <<<"$1" || { echo "missing report line for $3: $2" >&2; echo "$1" >&2; exit 1; }
}

for release in 2.5 2.6; do
  version="${release/./}"
  if [[ "$release" == 2.5 ]]; then
    root="$root25"; closure=42; emitted=41
  else
    root="$root26"; closure=44; emitted=43
  fi
  common=(--schema "$root" --contract "$contract" --language cpp --world closed-schema)
  echo "== UCI $release Log"
  report="$("$cli" service-check "${common[@]}" --with-codec)"
  expect "$report" "selected oms messages: 1" "$release"
  expect "$report" "selected type closure: $closure" "$release"
  expect "$report" "renderable selected types: $closure" "$release"
  expect "$report" "status: READY" "$release"
  expect "$report" "codec renderable emitted declarations: $emitted/$emitted" "$release"
  expect "$report" "codec status: READY" "$release"
  echo "  service-check: closure $closure, codec $emitted/$emitted, READY"

  rm -rf "$scratch/with-$version" "$scratch/without-$version"
  "$cli" service-generate "${common[@]}" --with-codec --output "$scratch/with-$version" >/dev/null
  "$cli" service-generate "${common[@]}" --output "$scratch/without-$version" >/dev/null
  for file in oam.hpp service_api.hpp; do
    cmp "$scratch/with-$version/$file" "$scratch/without-$version/$file"
  done
  test ! -e "$scratch/without-$version/service_codec.hpp"
  test -s "$scratch/with-$version/service_codec.hpp"
  echo "  model and service API byte-identical with and without --with-codec"
  (cd "$scratch/with-$version" && sha256sum oam.hpp service_api.hpp service_codec.hpp | sed 's/^/  sha256 /')

  for compiler in g++ clang++; do
    if ! command -v "$compiler" >/dev/null 2>&1; then
      [[ "$compiler" == clang++ ]] || { echo "g++ is required" >&2; exit 1; }
      skipped+=("clang++ not installed ($release)")
      continue
    fi
    run_consumer "$version-$compiler" "$compiler" "$version" "$scratch/with-$version" "${strict[@]}"
  done
  for compiler in g++ clang++; do
    command -v "$compiler" >/dev/null 2>&1 || continue
    if sanitizer_ok "$compiler"; then
      run_consumer "$version-$compiler-asan-ubsan" "$compiler" "$version" \
        "$scratch/with-$version" "${sanitize[@]}"
      sanitized+=("$compiler ($release)")
    else
      skipped+=("ASan/UBSan unusable with $compiler ($release)")
    fi
  done
done

hard=0
if ((${#skipped[@]})); then
  printf 'SKIPPED (not claimed as a pass): %s\n' "${skipped[@]}"
  for item in "${skipped[@]}"; do
    case "$item" in
      clang++*) [[ -n "${CPP_OMS_ALLOW_CLANG_SKIP:-}" ]] || hard=1 ;;
    esac
  done
fi
if ((${#sanitized[@]} < 2)); then
  [[ -n "${CPP_OMS_ALLOW_SANITIZER_SKIP:-}" ]] || hard=1
  echo "SKIPPED (not claimed as a pass): sanitizers did not run for both releases"
fi
printf 'sanitizers exercised: %s\n' "${sanitized[*]:-none}"
if ((hard)); then
  echo "CPP-OMS-002 real Log: INCOMPLETE (skips above are not passes)" >&2
  exit 3
fi
if ((${#skipped[@]})); then
  echo "CPP-OMS-002 real Log: PASS for everything that ran (skips above are not claimed)"
else
  echo "CPP-OMS-002 real Log: PASS (2.5 and 2.6; strict g++/clang++ and ASan+UBSan)"
fi
