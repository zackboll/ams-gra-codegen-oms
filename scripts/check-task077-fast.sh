#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
run_exact() {
  local name="$1" marker="$2"; shift 2
  local output
  output="$("$@" -- --list 2>&1)"
  printf '%s\n' "$output"
  test "$(grep -Fxc -- "$name: test" <<<"$output")" = 1
  output="$("$@" -- --exact "$name" --nocapture 2>&1)"
  printf '%s\n' "$output"
  grep -Fqx -- "$marker" <<<"$output"
  grep -Eq '^test result: ok\. 1 passed; 0 failed; 0 ignored;' <<<"$output"
}
run_exact config::tests::task077_configuration 'TASK077 CONFIGURATION: PASSED' cargo test --locked -p ams-gra-oms-runtime-rust --lib
run_exact duplicate::tests::task077_duplicate_names 'TASK077 DUPLICATE NAMES: PASSED' cargo test --locked -p ams-gra-oms-runtime-rust --lib
run_exact duplicate::tests::task077_value_parity 'TASK077 VALUE PARITY: PASSED' cargo test --locked -p ams-gra-oms-runtime-rust --lib
run_exact duplicate::tests::task077_error_precedence 'TASK077 ERROR PRECEDENCE: PASSED' cargo test --locked -p ams-gra-oms-runtime-rust --lib
run_exact task077_dispatch_recovery_and_independent_policies 'TASK077 DISPATCH RECOVERY: PASSED' cargo test --locked -p ams-gra-oms-runtime-rust-facade-tests --test task077_ingress
run_exact task077_generated_codec_comparison 'TASK077 GENERATED CODEC COMPARISON: PASSED' cargo test --locked -p ams-gra-oms-runtime-rust-facade-tests --test task077_ingress
echo 'TASK077 FAST: PASSED'