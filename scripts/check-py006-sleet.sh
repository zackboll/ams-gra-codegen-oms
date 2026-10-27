#!/usr/bin/env bash
# Reuse the shared harness's exact broker. No clone/build/download here.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
test -n "${AMS_GRA_SLEET_BIN:-}"
test -x "$AMS_GRA_SLEET_BIN"
PYTHON="${PYTHON:-python3}"
"$PYTHON" -c 'import websockets; assert websockets.__version__ == "17.2"'
for policy in normal optimized; do
  options=()
  if [ "$policy" = optimized ]; then options=(-O); fi
  status=0
  output="$("$PYTHON" "${options[@]}" -W error -m unittest discover \
    -s python/oms-lacal-runtime/tests -p test_header_sleet.py -v 2>&1)" || status=$?
  printf '%s\n' "$output"
  if (( status != 0 )); then exit "$status"; fi
  grep -Fq 'PY006 PINNED SLEET HEADER RECEIPT AND TYPED DELIVERY: PASSED' <<<"$output"
  grep -Eq '^Ran 1 test in ' <<<"$output"
  grep -Fxq 'OK' <<<"$output"
done