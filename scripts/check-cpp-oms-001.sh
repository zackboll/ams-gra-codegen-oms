#!/usr/bin/env bash
# CPP-OMS-001: reproducible local validation of the generated C++17 OMS JSON
# codec, independent of the Rust test harness.
#
# Generates the synthetic fixture through the production CLI, then compiles and
# runs tests/cpp-codec/consumer.cpp with strict C++17 flags under every
# available compiler (g++, clang++) and, when usable, AddressSanitizer plus
# UndefinedBehaviorSanitizer. Finally it checks that the model and service API
# headers are byte-identical with and without --with-codec.
#
# Usage: scripts/check-cpp-oms-001.sh [scratch-directory]
set -euo pipefail
repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
scratch="${1:-$(mktemp -d)}"
mkdir -p "$scratch"
cd "$repo"
cargo build --locked -p ams-gra-codegen-oms --bin ams-gra-codegen-oms
cli="$repo/target/debug/ams-gra-codegen-oms"
schema=tests/fixtures/cpp-codec/oam.xsd
contract=tests/fixtures/cpp-codec/service.yaml
common=(--schema "$schema" --contract "$contract" --language cpp --world closed-schema)

"$cli" service-check "${common[@]}" --with-codec
rm -rf "$scratch/with" "$scratch/without"
"$cli" service-generate "${common[@]}" --with-codec --output "$scratch/with"
"$cli" service-generate "${common[@]}" --output "$scratch/without"
for file in oam.hpp service_api.hpp; do
  cmp "$scratch/with/$file" "$scratch/without/$file"
done
test ! -e "$scratch/without/service_codec.hpp"
cmp "$scratch/with/service_codec.hpp" tests/fixtures/cpp-codec/expected/service_codec.hpp

strict=(-std=c++17 -Wall -Wextra -Wpedantic -Werror -Wconversion -Wsign-conversion
        -Wshadow -Wold-style-cast -Wcast-align -Wnon-virtual-dtor -Woverloaded-virtual
        -Wnull-dereference -Wdouble-promotion -Wformat=2)
for compiler in g++ clang++; do
  if ! command -v "$compiler" >/dev/null 2>&1; then
    echo "skipping $compiler (not installed)"
    continue
  fi
  "$compiler" "${strict[@]}" -I "$scratch/with" tests/cpp-codec/consumer.cpp -o "$scratch/consumer-$compiler"
  echo "== $compiler strict"
  "$scratch/consumer-$compiler"
done

sanitize=(-std=c++17 -O1 -g -fsanitize=address,undefined -fno-sanitize-recover=all)
echo 'int main() { return 0; }' > "$scratch/probe.cpp"
if g++ "${sanitize[@]}" "$scratch/probe.cpp" -o "$scratch/probe" 2>/dev/null; then
  g++ "${sanitize[@]}" -I "$scratch/with" tests/cpp-codec/consumer.cpp -o "$scratch/consumer-san"
  echo "== g++ ASan+UBSan"
  "$scratch/consumer-san"
else
  echo "skipping sanitizers (unusable with this g++)"
fi
echo 'CPP-OMS-001 CHECKS: PASSED'
