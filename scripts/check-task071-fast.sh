#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
run_exact() {
  local package="$1" suite="$2" name="$3" output status=0
  output="$(cargo test -p "$package" --test "$suite" -- --list 2>&1)" || status=$?
  printf '%s\n' "$output"
  if (( status != 0 )); then return "$status"; fi
  grep -Fqx "$name: test" <<<"$output"
  output="$(cargo test -p "$package" --test "$suite" -- --exact "$name" --nocapture 2>&1)" || status=$?
  printf '%s\n' "$output"
  if (( status != 0 )); then return "$status"; fi
  grep -qE '^test result: ok\. 1 passed; 0 failed;' <<<"$output"
}
for name in occurrences_and_first_occurrence_aggregation exact_tsv_escaping_and_group_presence empty_routes_are_valid sleet_validation_fails_closed exact_pair_duplicates_and_mixed_direction aggregation_preserves_namespace_and_topic_equality all_non_oms_kinds_are_counted_not_invented unknown_and_ambiguous_names_remain_resolver_errors; do
  run_exact ams-gra-oms-service-routes projection "$name"
done
for name in task071_cli_argument_controls task071_cli_determinism_namespace_and_failures task071_private_overlay_uses_existing_resolution task071_oam_and_empty_production_cli; do
  run_exact ams-gra-codegen-oms service_routes "$name"
done
echo 'TASK071 ROUTE MANIFEST FAST: PASSED'