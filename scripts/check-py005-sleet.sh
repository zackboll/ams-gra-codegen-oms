#!/usr/bin/env bash
# Focused campaign; caller supplies the existing harness's verified broker.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
test -n "${AMS_GRA_SLEET_BIN:-}"
test -x "$AMS_GRA_SLEET_BIN"
PYTHON="${PYTHON:-python3}"
"$PYTHON" -c 'import websockets; assert websockets.__version__ == "17.2"'
cargo build --locked -p ams-gra-codegen-oms
for policy in normal optimized; do
  options=()
  if [ "$policy" = optimized ]; then options=(-O); fi
  status=0
  output="$("$PYTHON" "${options[@]}" -m unittest discover \
    -s python/oms-lacal-runtime/tests -p test_sleet.py -v 2>&1)" || status=$?
  printf '%s\n' "$output"
  if (( status != 0 )); then exit "$status"; fi
  grep -Fq 'PY005 PINNED SLEET RECEIPT AND TYPED DELIVERY: PASSED' <<<"$output"
  grep -Eq '^Ran 1 test in ' <<<"$output"
  grep -Fxq 'OK' <<<"$output"
done