#!/usr/bin/env bash
set -euo pipefail
run_exact() {
  local package=$1 target=$2 name=$3 marker=$4 status=0 list output
  list="$(cargo test -p "$package" --test "$target" -- --list 2>&1)" || status=$?
  printf '%s\n' "$list"
  if (( status != 0 )); then return "$status"; fi
  test "$(grep -Fxc "$name: test" <<<"$list")" = 1
  output="$(cargo test -p "$package" --test "$target" -- --exact "$name" --nocapture 2>&1)" || status=$?
  printf '%s\n' "$output"
  if (( status != 0 )); then return "$status"; fi
  grep -Fqx "test $name ... ok" <<<"$output"
  grep -qE '^test result: ok\. 1 passed; 0 failed;' <<<"$output"
  if [[ -n "$marker" ]]; then grep -Fq "$marker" <<<"$output"; fi
}
export AMS_GRA_REQUIRE_GNAT=1
run_exact ams-gra-oms-codegen-core unicode_string task066_exact_classifier_and_fail_closed_neighbors ''
run_exact ams-gra-oms-codegen-core unicode_string task066_support_names_and_renamed_profile_are_preflight_consistent ''
run_exact ams-gra-oms-codegen-core unicode_string task066_authority_membership_and_independent_profile_corpus 'TASK066 CORE CORPUS: PASSED'
run_exact ams-gra-codegen-oms unicode_string_compilers task066_production_compiler_corpus_lifecycle_and_planted_failure 'TASK066 PRODUCTION COMPILER CORPUS: PASSED'
run_exact ams-gra-codegen-oms unicode_string_compilers task066_generated_support_neighbor_fails_closed_everywhere 'TASK066 GENERATED SUPPORT FAIL-CLOSED: PASSED'
run_exact ams-gra-oms-runtime-rust-facade-tests generated_codec_unicode31 task066_checked_unicode_codec_preserves_text_and_rejects_neighbors 'TASK066 CHECKED UNICODE JSON CODEC: PASSED'
run_exact ams-gra-oms-runtime-rust-facade-tests generated_codec_mock_owp task066_unicode_values_are_checked_before_typed_delivery 'TASK066 UNICODE MOCK OWP: PASSED'
echo 'TASK066 FAST GATES: PASSED'