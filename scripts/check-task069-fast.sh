#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
run_exact() {
  local package="$1" target="$2" name="$3" output
  output="$(cargo test -p "$package" --test "$target" -- --exact "$name" --nocapture 2>&1)"
  printf '%s\n' "$output"
  grep -qE '^test result: ok\. 1 passed; 0 failed; 0 ignored;' <<<"$output"
}
for name in \
  task069_primitive_alias_base_abstract_and_kind \
  task069_members_and_choice_exact_properties \
  task069_constraints_all_normalized_fields \
  task069_binary_and_list \
  task069_enum_values_and_order \
  task069_qnames_presence_messages_direction \
  task069_noise_declaration_order_and_deterministic_tsv; do
  run_exact ams-gra-oms-schema-diff semantic "$name"
done
for name in \
  task069_cli_usage_controls \
  task069_cli_synthetic_exact_and_deterministic \
  task069_cli_errors_are_not_empty_diffs \
  task069_cli_overlay_order_and_malformed_schema; do
  run_exact ams-gra-codegen-oms schema_diff "$name"
done
echo 'TASK069 FAST SEMANTIC DIFF: PASSED'