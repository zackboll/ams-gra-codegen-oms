#!/usr/bin/env bash
set -euo pipefail
test -n "${AMS_GRA_UCI_2_5_ROOT:-}"
test -n "${AMS_GRA_UCI_2_6_ROOT:-}"
cargo fetch --locked
for name in task063_pinned_inventory_coverage_and_service_impact task063_smallest_real_service_compiler_codec_vertical; do
  status=0
  output="$(cargo test --release -p ams-gra-codegen-oms --test uci_patterned_integral "$name" -- --exact --nocapture 2>&1)" || status=$?
  printf '%s\n' "$output"
  if (( status != 0 )); then exit "$status"; fi
  grep -Fqx "test $name ... ok" <<<"$output"
  grep -qE '^test result: ok\. 1 passed' <<<"$output"
  if [[ "$name" == task063_pinned_inventory_coverage_and_service_impact ]]; then
    for release in 2.5 2.6; do grep -Fqx "UCI $release TASK063 PINNED INVENTORY AND IMPACT: PASSED" <<<"$output"; done
  else
    for release in 2.5 2.6; do grep -Fqx "UCI $release TASK063 SMALLEST SERVICE VERTICAL: PASSED" <<<"$output"; done
  fi
done
python3 scripts/check-task063-service-impact.py "${CARGO_TARGET_DIR:-target}/release/ams-gra-codegen-oms"
echo 'TASK063 PINNED GATES: PASSED'