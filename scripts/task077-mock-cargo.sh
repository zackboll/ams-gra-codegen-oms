#!/usr/bin/env bash
# Test-only Cargo contract for Task 077's two suites. Never executes Rust.
set -euo pipefail
names=(config::tests::task077_configuration duplicate::tests::task077_duplicate_names duplicate::tests::task077_value_parity duplicate::tests::task077_error_precedence task077_dispatch_recovery_and_independent_policies task077_generated_codec_comparison)
markers=('TASK077 CONFIGURATION: PASSED' 'TASK077 DUPLICATE NAMES: PASSED' 'TASK077 VALUE PARITY: PASSED' 'TASK077 ERROR PRECEDENCE: PASSED' 'TASK077 DISPATCH RECOVERY: PASSED' 'TASK077 GENERATED CODEC COMPARISON: PASSED')
contract() {
  echo "MOCK CONTRACT FAILURE: $*" >&2
  echo CONTRACT >> "$TRACE"
  exit 90
}
index=$(wc -l < "$TRACE")
(( index < 12 )) || contract 'extra invocation'
control=$((index / 2))
phase=list
(( index % 2 == 0 )) || phase=execute
package=ams-gra-oms-runtime-rust
suite=lib
suite_args=(--lib)
start=0; end=4
if (( control >= 4 )); then
  package=ams-gra-oms-runtime-rust-facade-tests
  suite=task077_ingress
  suite_args=(--test task077_ingress)
  start=4; end=6
fi
expected=(test --locked -p "$package" "${suite_args[@]}" --)
if [[ "$phase" == list ]]; then
  expected+=(--list)
else
  expected+=(--exact "${names[control]}" --nocapture)
fi
[[ $# == ${#expected[@]} ]] || contract 'argument count'
actual=("$@")
for ((i=0; i<$#; i++)); do
  [[ "${actual[i]}" == "${expected[i]}" ]] || contract "argument $i: ${actual[i]}"
done
printf '%s|%s|%s|%s\n' "$phase" "$package" "$suite" "${names[control]}" >> "$TRACE"
fault=healthy
if (( control == TARGET )); then
  case "$FAULT:$phase" in
    missing:list|failed-list:list|duplicate:list|failed-run:execute|marker:execute|ignored:execute|zero:execute)
      fault=$FAULT
      printf '%s|%s|%s\n' "$control" "$phase" "$fault" >> "$INJECTION"
      ;;
  esac
fi
if [[ "$phase" == list ]]; then
  for ((i=start; i<end; i++)); do
    if [[ "$fault" == missing && "$i" == "$control" ]]; then continue; fi
    echo "${names[i]}: test"
  done
  if [[ "$fault" == duplicate ]]; then echo "${names[control]}: test"; fi
  if [[ "$fault" == failed-list ]]; then exit 7; fi
else
  if [[ "$fault" != marker ]]; then echo "${markers[control]}"; fi
  case "$fault" in
    ignored) echo 'test result: ok. 0 passed; 0 failed; 1 ignored;' ;;
    zero) echo 'test result: ok. 0 passed; 0 failed; 0 ignored;' ;;
    *) echo "test ${names[control]} ... ok"
       echo 'test result: ok. 1 passed; 0 failed; 0 ignored;' ;;
  esac
  if [[ "$fault" == failed-run ]]; then exit 9; fi
fi