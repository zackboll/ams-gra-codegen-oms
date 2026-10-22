#!/usr/bin/env bash
# Exact-count, must-execute Fast CI. No broker/UCI fetch or build here.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
PYTHON="${PYTHON:-python3}"
"$PYTHON" -c 'import websockets; assert websockets.__version__ == "17.2"'
for policy in normal optimized; do
  options=()
  if [ "$policy" = optimized ]; then options=(-O); fi
  status=0
  output="$("$PYTHON" "${options[@]}" -W error -m unittest discover \
    -s python/oms-lacal-runtime/tests -p test_mock.py -v 2>&1)" || status=$?
  printf '%s\n' "$output"
  if (( status != 0 )); then exit "$status"; fi
  grep -Eq '^Ran 19 tests in ' <<<"$output"
  grep -Fxq 'OK' <<<"$output"
done
echo 'PY005 MOCK OWP NETWORK: PASSED'