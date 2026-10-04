#!/usr/bin/env bash
set -euo pipefail
require_one() {
  local name="$1" marker="$2" output status=0
  shift 2
  output="$("$@" "$name" -- --exact --nocapture 2>&1)" || status=$?
  printf '%s\n' "$output"
  if (( status != 0 )); then return "$status"; fi
  grep -Fqx "test $name ... ok" <<<"$output"
  grep -qE '^test result: ok\. 1 passed' <<<"$output"
  grep -Fqx "$marker" <<<"$output"
}
for entry in \
  'task063_exact_profile_and_fail_closed_neighbors|TASK063 EXACT AND NEIGHBOR GATES: PASSED' \
  'task063_generated_support_only_readiness_control|TASK063 GENERATED SUPPORT GATE: PASSED' \
  'task063_direct_patterned_integer_remains_unsupported|TASK063 DIRECT FIELD GATE: PASSED' \
  'task063_numeric_rendering_matches_ordinary_bounded_profile|TASK063 NUMERIC RENDERING EQUIVALENCE: PASSED' \
  'task063_generated_numeric_models_compile_and_execute|TASK063 Cpp NUMERIC COMPILER: PASSED'; do
  require_one "${entry%%|*}" "${entry#*|}" cargo test -p ams-gra-codegen-oms --test patterned_integral
done
for backend in ada rust cpp; do
  output="$(cargo test -p "ams-gra-oms-backend-$backend" --lib tests::lexical_constraints_fail_before_rendering -- --exact --nocapture 2>&1)" || { status=$?; printf '%s\n' "$output"; exit "$status"; }
  printf '%s\n' "$output"
  grep -Fqx 'test tests::lexical_constraints_fail_before_rendering ... ok' <<<"$output"
  grep -qE '^test result: ok\. 1 passed' <<<"$output"
done
require_one task063_generated_integer_codec_and_raw_json_semantics \
  'TASK063 GENERATED INTEGER CODEC AND JSON: PASSED' \
  cargo test -p ams-gra-oms-runtime-rust-facade-tests --test generated_codec_patterned_integral
require_one task063_out_of_range_integer_never_reaches_typed_handler \
  'TASK063 INTEGER MOCK OWP: PASSED' \
  cargo test -p ams-gra-oms-runtime-rust-facade-tests --test generated_codec_mock_owp
echo 'TASK063 FAST GATES: PASSED'