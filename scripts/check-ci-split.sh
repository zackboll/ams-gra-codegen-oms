#!/usr/bin/env bash
# Task 055: structural guard for the Fast CI / Deep CI split.
#
#   * Fast CI (.github/workflows/ci.yml) must not fetch real UCI or run the
#     real-Sleet harness, and must keep the local 1.95.0 MSRV check, the GNAT
#     gate and the final workspace test.
#   * Deep CI (.github/workflows/deep-ci.yml) must keep every check that was
#     moved out of Fast CI: the fetch scripts, each real-UCI test target, each
#     PASSED marker, the real-UCI 1.95.0 compile, and the real-Sleet harness.
#   * Deep CI and the real-Sleet harness must not check markers with
#     `printf ... | grep -q` (SIGPIPE hazard under pipefail).
#
# Only non-comment lines are inspected, so comments may mention anything and
# ordinary reformatting (indentation, reordering, step names) stays legal.
#
# Usage: scripts/check-ci-split.sh [fast-ci.yml] [deep-ci.yml] [sleet-script]
set -euo pipefail

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
fast="${1:-$repo/.github/workflows/ci.yml}"
deep="${2:-$repo/.github/workflows/deep-ci.yml}"
sleet="${3:-$repo/scripts/run-real-sleet-test.sh}"

failures=0
fail() {
  echo "check-ci-split: $*" >&2
  failures=$((failures + 1))
}

# Strip whole-line comments; keep everything else verbatim.
code() { grep -v '^[[:space:]]*#' "$1" || true; }

fast_code="$(code "$fast")"
deep_code="$(code "$deep")"
sleet_code="$(code "$sleet")"

# ---- Fast CI must stay free of external/reference work ------------------
for forbidden in \
  fetch-pinned-uci-2.5.sh \
  fetch-pinned-uci-2.6.sh \
  run-real-sleet-test.sh \
  AMS_GRA_UCI_2_5_ROOT \
  AMS_GRA_UCI_2_6_ROOT \
  '--test uci_generated_support' \
  '--test uci_duration'; do
  if grep -Fq -- "$forbidden" <<<"$fast_code"; then
    fail "Fast CI must not reference $forbidden"
  fi
done

# ---- Fast CI must keep its pre-merge floor --------------------------------
for required in \
  'gnatmake --version' \
  'cargo fmt --all -- --check' \
  'cargo check --workspace --all-targets' \
  'cargo clippy --workspace --all-targets -- -D warnings' \
  'rustup toolchain install 1.95.0 --profile minimal' \
  'cargo +1.95.0 check --locked -p ams-gra-oms-runtime-api' \
  'cargo +1.95.0 check --locked -p ams-gra-oms-runtime-rust --all-targets' \
  'cargo +1.95.0 check --locked -p ams-gra-oms-runtime-rust-facade-tests --all-targets' \
  'cargo test --workspace' \
  'AMS_GRA_REQUIRE_GNAT: "1"'; do
  if ! grep -Fq -- "$required" <<<"$fast_code"; then
    fail "Fast CI lost required check: $required"
  fi
done

# ---- Deep CI must keep every moved check ----------------------------------
# An INVOCATION (script followed by its quoted destination argument), not a
# mere mention such as the pull_request.paths filter entries.
fetch25='scripts/fetch-pinned-uci-2.5.sh "$'
fetch26='scripts/fetch-pinned-uci-2.6.sh "$'
for required in \
  "$fetch25" \
  "$fetch26" \
  'AMS_GRA_UCI_2_5_ROOT=' \
  'AMS_GRA_UCI_2_6_ROOT=' \
  '--test uci_binary_provenance' \
  'UCI 2.5 BINARY PROVENANCE INVENTORY: PASSED' \
  'UCI 2.6 BINARY PROVENANCE INVENTORY: PASSED' \
  '--test uci_constrained_binary' \
  'UCI 2.5 CONSTRAINED BINARY INVENTORY: PASSED' \
  'UCI 2.6 CONSTRAINED BINARY INVENTORY: PASSED' \
  'UCI 2.5 CONSTRAINED BINARY MESSAGE IMPACT: PASSED' \
  'UCI 2.6 CONSTRAINED BINARY MESSAGE IMPACT: PASSED' \
  '--test uci_member_names' \
  'UCI 2.5 MEMBER IDENTIFIER INVENTORY: PASSED' \
  'UCI 2.6 MEMBER IDENTIFIER INVENTORY: PASSED' \
  'UCI 2.5 WHOLE-SCHEMA FIRST BLOCKER PROBE: PASSED' \
  'UCI 2.6 WHOLE-SCHEMA FIRST BLOCKER PROBE: PASSED' \
  'test task054_real_uci_category_a_selection_generates_and_compiles' \
  'REAL CATEGORY-A MEMBER REMAPPING: PASSED' \
  '--test uci_generated_support' \
  'UCI 2.5 ORDEROFBATTLE GENERATED SUPPORT PARITY: PASSED' \
  'UCI 2.6 ORDEROFBATTLE GENERATED SUPPORT PARITY: PASSED' \
  'UCI 2.5 CATEGORY-A GENERATED SUPPORT PARITY: PASSED' \
  '--test uci_duration' \
  'UCI 2.5 DURATION INVENTORY: PASSED' \
  'UCI 2.6 DURATION INVENTORY: PASSED' \
  'UCI 2.5 DURATION MESSAGE IMPACT: RECORDED' \
  'UCI 2.6 DURATION MESSAGE IMPACT: RECORDED' \
  'REAL CATEGORY-A DURATION SERVICE: PASSED' \
  'AMS_GRA_REQUIRE_GNAT: "1"' \
  'gnatmake --version' \
  'rustup toolchain install 1.95.0 --profile minimal' \
  'cargo +1.95.0 check --locked -p ams-gra-oms-runtime-rust-facade-tests --all-targets' \
  'real::task050_real_position_report_decodes_and_re_encodes' \
  'REAL POSITIONREPORT CODEC: PASSED' \
  'real::task052_real_subsystem_stream_hex_binary_round_trips' \
  'REAL SUBSYSTEMSTREAM HEXBINARY CODEC: PASSED' \
  'scripts/run-real-sleet-test.sh' \
  'set -euo pipefail'; do
  if ! grep -Fq -- "$required" <<<"$deep_code"; then
    fail "Deep CI lost moved check: $required"
  fi
done

# Each real-UCI release is fetched at most once per job: no fetch script may
# appear more often than the number of jobs that use it (real-uci,
# msrv-real-uci, real-sleet for 2.5; real-uci for 2.6).
count() { grep -Fc -- "$1" <<<"$2" || true; }
[[ "$(count "$fetch25" "$deep_code")" -le 3 ]] ||
  fail "Deep CI fetches UCI 2.5 more than once per job"
[[ "$(count "$fetch26" "$deep_code")" -le 1 ]] ||
  fail "Deep CI fetches UCI 2.6 more than once"

# ---- No `printf | grep -q` marker checks (SIGPIPE under pipefail) ---------
pipe_hazard='printf .*\| *grep +(-[A-Za-z]*q|.* -[A-Za-z]*q)'
if grep -Eq -- "$pipe_hazard" <<<"$deep_code"; then
  fail "Deep CI uses a 'printf | grep -q' marker check"
fi
if grep -Eq -- "$pipe_hazard" <<<"$sleet_code"; then
  fail "run-real-sleet-test.sh uses a 'printf | grep -q' marker check"
fi

if [[ "$failures" -ne 0 ]]; then
  echo "check-ci-split: FAILED ($failures problem(s))" >&2
  exit 1
fi
echo "check-ci-split: PASSED"
