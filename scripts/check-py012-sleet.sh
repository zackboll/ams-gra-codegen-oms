#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
test -x "${AMS_GRA_SLEET_BIN:?pinned Sleet required}"
PYTHON="${PYTHON:-python3}"
"$PYTHON" -c 'import websockets; assert websockets.__version__ == "17.2"'
for policy in normal optimized; do
  options=()
  if [ "$policy" = optimized ]; then options=(-O); fi
  if [ -n "${AMS_GRA_PY012_EVIDENCE_ROOT:-}" ]; then
    export AMS_GRA_PY012_EVIDENCE="$AMS_GRA_PY012_EVIDENCE_ROOT/$policy"
  fi
  "$PYTHON" "${options[@]}" -W error -m unittest discover \
    -s python/oms-lacal-runtime/tests -p test_unbounded_repetition_sleet.py -v
done