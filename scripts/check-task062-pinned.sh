#!/usr/bin/env bash
set -euo pipefail
test -n "${AMS_GRA_UCI_2_5_ROOT:-}"
test -n "${AMS_GRA_UCI_2_6_ROOT:-}"
export AMS_GRA_REQUIRE_TASK062_PINNED=1
export TMPDIR="${TMPDIR:-${RUNNER_TEMP:-/tmp}}"
status=0
list="$(cargo test --release -p ams-gra-codegen-oms --test uci_ipv6_address -- --list 2>&1)" || status=$?
printf '%s\n' "$list"
if (( status != 0 )); then exit "$status"; fi
name=task062_pinned_ipv6_inventory_coverage_and_impact
test "$(grep -Fxc "$name: test" <<<"$list")" = 1
output="$(cargo test --release -p ams-gra-codegen-oms --test uci_ipv6_address -- --exact "$name" --nocapture 2>&1)" || status=$?
printf '%s\n' "$output"
if (( status != 0 )); then exit "$status"; fi
grep -qE '^test result: ok\. 1 passed; 0 failed;' <<<"$output"
for release in 2.5 2.6; do
  for world in ClosedSchemaSet OpenExtensions; do
    grep -Fqx "UCI $release TASK062 $world INVENTORY COVERAGE IMPACT: PASSED" <<<"$output"
  done
done
python3 scripts/check-task062-service-impact.py "${CARGO_TARGET_DIR:-target}/release/ams-gra-codegen-oms"
python3 scripts/check-task062-vertical.py "${CARGO_TARGET_DIR:-target}/release/ams-gra-codegen-oms"
echo 'TASK062 PINNED GATES: PASSED'