#!/usr/bin/env bash
# Synthetic wrapper evidence only: healthy path, attributed faults, and mutants.
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
gate="$root/scripts/check-task077-fast.sh"
mutations=1
if [[ $# != 0 ]]; then
  [[ $# == 2 && "$1" == --check-only ]] || { echo 'usage: test-task077-ci-wrappers.sh [--check-only GATE]' >&2; exit 2; }
  gate="$2"
  mutations=0
fi
scratch="$(mktemp -d -t task077a-wrappers-XXXXXX)"
trap 'rm -rf "$scratch"' EXIT
mkdir "$scratch/bin"
cp "$root/scripts/task077-mock-cargo.sh" "$scratch/bin/cargo"
chmod +x "$scratch/bin/cargo"
# Independent expected inventory: deleting a gate control must not delete this.
names=(config::tests::task077_configuration duplicate::tests::task077_duplicate_names duplicate::tests::task077_value_parity duplicate::tests::task077_error_precedence task077_dispatch_recovery_and_independent_policies task077_generated_codec_comparison)
: > "$scratch/expected"
for i in "${!names[@]}"; do
  package=ams-gra-oms-runtime-rust; suite=lib
  if (( i >= 4 )); then package=ams-gra-oms-runtime-rust-facade-tests; suite=task077_ingress; fi
  for phase in list execute; do
    printf '%s|%s|%s|%s\n' "$phase" "$package" "$suite" "${names[i]}" >> "$scratch/expected"
  done
done
die() { echo "Task077 wrapper regression: $*" >&2; exit 1; }
serial=0
execute() {
  local selected_gate="$1" fault="$2" target="$3"
  serial=$((serial + 1))
  trace="$scratch/$serial.trace"; injection="$scratch/$serial.injection"; output="$scratch/$serial.output"
  : > "$trace"; : > "$injection"
  status=0
  TRACE="$trace" INJECTION="$injection" FAULT="$fault" TARGET="$target" PATH="$scratch/bin:$PATH" \
    bash "$selected_gate" > "$output" 2>&1 || status=$?
  if grep -Fq CONTRACT "$trace" || grep -Fq 'MOCK CONTRACT FAILURE' "$output"; then
    cat "$output" >&2
    die "unexpected mock-contract failure ($fault control $target)"
  fi
}
check_success() {
  [[ "$status" == 0 ]] || die "healthy/weak path exit $status"
  cmp -s "$scratch/expected" "$trace" || die 'healthy/weak path did not execute the expected 12 invocations'
  grep -Fqx 'TASK077 FAST: PASSED' "$output" || die 'missing final success marker'
}
healthy() {
  execute "$1" healthy -1
  check_success
  [[ ! -s "$injection" ]] || die 'healthy path injected a fault'
}
check_fault() {
  local selected_gate="$1" fault="$2" target="$3" phase=execute length
  case "$fault" in missing|failed-list|duplicate) phase=list ;; esac
  execute "$selected_gate" "$fault" "$target"
  [[ "$status" != 0 ]] || die "$fault control $target incorrectly passed"
  if grep -Fqx 'TASK077 FAST: PASSED' "$output"; then die "$fault control $target reached success"; fi
  length=$((target * 2 + 1))
  if [[ "$phase" == execute ]]; then length=$((length + 1)); fi
  head -n "$length" "$scratch/expected" > "$scratch/prefix"
  cmp -s "$scratch/prefix" "$trace" || die "$fault control $target rejected at the wrong invocation"
  printf '%s|%s|%s\n' "$target" "$phase" "$fault" > "$scratch/injected"
  cmp -s "$scratch/injected" "$injection" || die "$fault control $target injection mismatch"
  if [[ "$fault" == failed-list && "$status" != 7 ]]; then die 'listing exit status not propagated'; fi
  if [[ "$fault" == failed-run && "$status" != 9 ]]; then die 'execution exit status not propagated'; fi
  echo "TASK077 REJECTION: $fault control $target $phase invocation $length"
}
healthy "$gate"
echo 'TASK077 HEALTHY MOCK: PASSED (6 registrations, 6 executions)'
for target in 0 5; do
  for fault in missing failed-list failed-run marker ignored zero duplicate; do
    check_fault "$gate" "$fault" "$target"
  done
done
if (( mutations )); then
  marker='  grep -Fqx -- "$marker" <<<"$output"'
  count="  grep -Eq '^test result: ok\\. 1 passed; 0 failed; 0 ignored;' <<<\"\$output\""
  test "$(grep -Fxc "$marker" "$gate")" = 1 || die 'marker assertion identity'
  test "$(grep -Fxc "$count" "$gate")" = 1 || die 'count assertion identity'
  for mutant in marker count both; do
    copy="$scratch/$mutant.sh"
    : > "$copy"
    removed=0
    # Preserve all nonremoved bytes, including a non-newline-terminated EOF.
    while true; do
      newline=1
      IFS= read -r line || newline=0
      if [[ "$newline" == 0 && -z "$line" ]]; then break; fi
      if [[ "$mutant" != count && "$line" == "$marker" ]] || [[ "$mutant" != marker && "$line" == "$count" ]]; then
        removed=$((removed + 1))
      else
        printf '%s' "$line" >> "$copy"
        if (( newline )); then printf '\n' >> "$copy"; fi
      fi
      if (( ! newline )); then break; fi
    done < "$gate"
    expected_removed=1; if [[ "$mutant" == both ]]; then expected_removed=2; fi
    [[ "$removed" == "$expected_removed" ]] || die 'mutation removal count'
    diff_status=0
    diff --unchanged-line-format='' --old-line-format='%L' --new-line-format='ADDED:%L' "$gate" "$copy" > "$scratch/delta" || diff_status=$?
    [[ "$diff_status" == 1 ]] || die 'mutation diff status'
    : > "$scratch/removed"
    if [[ "$mutant" != count ]]; then printf '%s\n' "$marker" >> "$scratch/removed"; fi
    if [[ "$mutant" != marker ]]; then printf '%s\n' "$count" >> "$scratch/removed"; fi
    cmp -s "$scratch/removed" "$scratch/delta" || die 'mutation changed unexpected bytes/lines'
    healthy "$copy"
    for fault in marker ignored; do
      if [[ "$mutant:$fault" == marker:ignored || "$mutant:$fault" == count:marker ]]; then continue; fi
      execute "$copy" "$fault" 5
      check_success
      printf '5|execute|%s\n' "$fault" > "$scratch/injected"
      cmp -s "$scratch/injected" "$injection" || die 'mutant fault injection mismatch'
    done
    checker_status=0
    bash "$root/scripts/test-task077-ci-wrappers.sh" --check-only "$copy" > "$scratch/$mutant.checker" 2>&1 || checker_status=$?
    [[ "$checker_status" != 0 ]] || die "$mutant mutant escaped checker"
    fault=marker; if [[ "$mutant" == count ]]; then fault=ignored; fi
    grep -Fqx "Task077 wrapper regression: $fault control 0 incorrectly passed" "$scratch/$mutant.checker" || die "$mutant checker failed for unrelated reason"
    echo "TASK077 MUTATION: $mutant healthy/weak paths passed; checker rejected intended regression"
  done
  echo 'TASK077 MUTATIONS: PASSED (3 mutants, 4 faulty passes detected)'
fi
echo 'TASK077 WRAPPER NEGATIVES: PASSED'