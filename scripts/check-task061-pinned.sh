#!/usr/bin/env bash
# Both pinned roots are mandatory; early-return developer skips are not evidence.
set -euo pipefail
test -n "${AMS_GRA_UCI_2_5_ROOT:-}"
test -n "${AMS_GRA_UCI_2_6_ROOT:-}"
# Generated standalone probes deliberately run offline. The CLI-only build
# does not fetch runtime crates (e.g. block-buffer); seed the LOCKED workspace
# cache explicitly rather than depending on another hosted job's private cache.
cargo fetch --locked
status=0
output="$(cargo test --release -p ams-gra-codegen-oms --test uci_time_zulu \
  task061_pinned_time_inventory_and_impact -- --exact --nocapture 2>&1)" || status=$?
printf '%s\n' "$output"
if (( status != 0 )); then exit "$status"; fi
grep -Fqx 'test task061_pinned_time_inventory_and_impact ... ok' <<<"$output"
grep -qE '^test result: ok\. 1 passed' <<<"$output"
grep -Fqx 'UCI 2.5 TASK061 TIME INVENTORY AND IMPACT: PASSED' <<<"$output"
grep -Fqx 'UCI 2.6 TASK061 TIME INVENTORY AND IMPACT: PASSED' <<<"$output"
status=0
output="$(cargo test --release -p ams-gra-codegen-oms --test uci_time_zulu \
  task061_smallest_real_service_compilers_and_time_codec -- --exact --nocapture 2>&1)" || status=$?
printf '%s\n' "$output"
if (( status != 0 )); then exit "$status"; fi
grep -Fqx 'test task061_smallest_real_service_compilers_and_time_codec ... ok' <<<"$output"
grep -qE '^test result: ok\. 1 passed' <<<"$output"
grep -Fqx 'UCI 2.5 TASK061 DLZ COMPILER CODEC VERTICAL: PASSED' <<<"$output"
grep -Fqx 'UCI 2.6 TASK061 DLZ COMPILER CODEC VERTICAL: PASSED' <<<"$output"
echo 'TASK061 PINNED GATES: PASSED'