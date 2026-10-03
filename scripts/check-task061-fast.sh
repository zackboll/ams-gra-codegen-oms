#!/usr/bin/env bash
# Exact registered Task 061 synthetic gates, shared by local and hosted Fast CI.
set -euo pipefail
require_one_test() {
  local name="$1" status=0 output
  shift
  output="$("$@" "$name" -- --exact --nocapture 2>&1)" || status=$?
  printf '%s\n' "$output"
  if (( status != 0 )); then return "$status"; fi
  grep -Fqx "test $name ... ok" <<<"$output"
  grep -qE '^test result: ok\. 1 passed' <<<"$output"
}
for backend in ada rust cpp; do
  require_one_test task061_time_corpus_lifecycle_and_storage_compile \
    cargo test -p "ams-gra-oms-backend-$backend" --test time_zulu
done
for name in \
  task061_exact_named_time_and_neighbors_share_the_boundary \
  task061_generated_support_only_time_blocks_final_readiness \
  task061_private_scanner_helpers_reserve_no_global_names \
  task061_direct_time_is_still_unsupported_in_records_and_choices; do
  require_one_test "$name" cargo test -p ams-gra-codegen-oms --test time_zulu_capability
done
require_one_test task061_compiled_time_codec_round_trips_and_rejects_invalid_values \
  cargo test -p ams-gra-oms-runtime-rust-facade-tests --test generated_codec_time_zulu
require_one_test task061_invalid_time_is_not_delivered_to_the_typed_handler \
  cargo test -p ams-gra-oms-runtime-rust-facade-tests --test generated_codec_mock_owp
echo 'TASK061 FAST GATES: PASSED'