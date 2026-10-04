#!/usr/bin/env bash
# Exact registered tests, original failure propagation and diagnostics first.
set -euo pipefail
one() {
  local name="$1" list output status=0
  shift
  list="$("$@" -- --list 2>&1)" || status=$?
  printf '%s\n' "$list"
  if (( status != 0 )); then return "$status"; fi
  test "$(grep -Fxc "$name: test" <<<"$list")" = 1
  output="$("$@" -- --exact "$name" --nocapture 2>&1)" || status=$?
  printf '%s\n' "$output"
  if (( status != 0 )); then return "$status"; fi
  grep -Fqx "test $name ... ok" <<<"$output"
  grep -qE '^test result: ok\. 1 passed; 0 failed;' <<<"$output"
  case "$name" in
    *exact_ipv6_semantics*) grep -Fq 'TASK062 INDEPENDENT SEMANTIC CORPUS: PASSED' <<<"$output" ;;
    *standalone_ipv6*)
      grep -Fq 'WRONG EXPECTATION DETECTED AND RESTORED: PASSED (26946 cases)' <<<"$output"
      grep -Fq 'CORPUS LIFECYCLE: PASSED' <<<"$output" ;;
    task062_name_free*) grep -Fqx 'TASK062 SHARED IPV6 CAPABILITY: PASSED' <<<"$output" ;;
    task062_generated_support*) grep -Fqx 'TASK062 SUPPORT-ONLY IPV6 FAIL-CLOSED: PASSED' <<<"$output" ;;
    task062_production_source*) grep -Fqx 'TASK062 PRIVATE IPV6 HELPERS NAME PREFLIGHT: PASSED' <<<"$output" ;;
    task062_compiled*) grep -Fqx 'TASK062 COMPILED IPV6 JSON CODEC: PASSED' <<<"$output" ;;
    task062_invalid*) grep -Fqx 'TASK062 IPV6 MOCK OWP: PASSED' <<<"$output" ;;
  esac
}
one ipv6_address::tests::exact_ipv6_semantics_match_independent_partition_corpus cargo test -p ams-gra-oms-codegen-core --lib
one ipv6_address::tests::exact_ipv6_profile_and_all_facet_neighbors_fail_closed cargo test -p ams-gra-oms-codegen-core --lib
for backend in ada rust cpp; do
  one ipv6_address::tests::standalone_ipv6_compiler_corpus_and_lifecycle cargo test -p "ams-gra-oms-backend-$backend" --lib
done
one task062_name_free_exact_profile_and_neighbors_share_all_capability_gates cargo test -p ams-gra-codegen-oms --test ipv6_capability
one task062_generated_support_only_ipv6_controls_final_readiness cargo test -p ams-gra-codegen-oms --test ipv6_capability
one task062_production_source_size_and_private_helper_names cargo test -p ams-gra-codegen-oms --test ipv6_capability
one task062_compiled_checked_ipv6_codec_preserves_spelling_and_rejects_invalid cargo test -p ams-gra-oms-runtime-rust-facade-tests --test generated_codec_ipv6
one task062_invalid_ipv6_is_not_delivered_to_the_typed_handler cargo test -p ams-gra-oms-runtime-rust-facade-tests --test generated_codec_mock_owp
echo 'TASK062 FAST GATES: PASSED'