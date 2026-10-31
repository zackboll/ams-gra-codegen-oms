#!/usr/bin/env bash
# No download/build: use the existing shared pinned Sleet harness binary.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
test -n "${AMS_GRA_SLEET_BIN:-}"
test -x "$AMS_GRA_SLEET_BIN"
PYTHON="${PYTHON:-python3}"
"$PYTHON" -c 'import websockets; assert websockets.__version__ == "17.2"'
for policy in normal optimized; do
  if [ -n "${RUNNER_TEMP:-}" ] && [ -z "${AMS_GRA_PY010_EVIDENCE_ROOT:-}" ]; then
    version="$("$PYTHON" -c 'import sys; print(str(sys.version_info.major) + str(sys.version_info.minor))')"
    AMS_GRA_PY010_EVIDENCE_ROOT="$RUNNER_TEMP/py010-sleet/$version"
  fi
  if [ -n "${AMS_GRA_PY010_EVIDENCE_ROOT:-}" ]; then
    export AMS_GRA_PY010_EVIDENCE="$AMS_GRA_PY010_EVIDENCE_ROOT/$policy"
  fi
  options=()
  if [ "$policy" = optimized ]; then options=(-O); fi
  status=0
  output="$("$PYTHON" "${options[@]}" -W error -m unittest discover \
    -s python/oms-lacal-runtime/tests -p test_visible_ascii_sleet.py -v 2>&1)" || status=$?
  printf '%s\n' "$output"
  if (( status != 0 )); then exit "$status"; fi
  grep -Fq 'PY010 PINNED SLEET VISIBLE ASCII: PASSED' <<<"$output"
  grep -Eq '^Ran 1 test in ' <<<"$output"
  grep -Fxq 'OK' <<<"$output"
done
