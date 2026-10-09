#!/usr/bin/env bash
set -euo pipefail
require_exact() {
  local name="$1"; shift
  local status=0 listing output
  listing="$("$@" -- --list 2>&1)" || status=$?
  printf '%s\n' "$listing"
  if (( status != 0 )); then return "$status"; fi
  test "$(grep -Fxc "$name: test" <<<"$listing")" = 1
  status=0
  output="$("$@" -- --exact "$name" --nocapture 2>&1)" || status=$?
  printf '%s\n' "$output"
  if (( status != 0 )); then return "$status"; fi
  grep -Eq '^test result: ok\. 1 passed; 0 failed;' <<<"$output"
}
for name in task070_policy_validation task070_backoff_progression_and_overflow task070_attempt_budget_and_no_final_delay task070_success_stops_attempts task070_typed_error_classification task070_real_refused_then_info; do
  require_exact "retry::tests::$name" cargo test --locked -p ams-gra-oms-runtime-rust --lib
done
for name in task070_terminal_refusal_preserves_error_and_joins_worker task070_init_rejection_is_terminal task070_established_session_never_reconnects_or_replays task070_generated_publish_once_and_close; do
  require_exact "$name" cargo test --locked -p ams-gra-oms-runtime-rust-facade-tests --test runtime_initial_retry
done
echo 'TASK070 INITIAL CONNECT RETRY: PASSED'
