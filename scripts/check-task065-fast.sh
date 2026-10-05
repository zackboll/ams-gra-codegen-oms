#!/usr/bin/env bash
set -euo pipefail
status=0
list="$(cargo test -p ams-gra-codegen-oms --test ada_companion_naming -- --list 2>&1)" || status=$?
printf '%s\n' "$list"
if (( status != 0 )); then exit "$status"; fi
for name in task065_synthetic_collision_repaired task065_semantic_name_matrix_and_gnat task065_broad_rename_design_gate_inventory; do
  test "$(grep -Fxc "$name: test" <<<"$list")" = 1
  status=0
  output="$(cargo test -p ams-gra-codegen-oms --test ada_companion_naming -- --exact "$name" --nocapture 2>&1)" || status=$?
  printf '%s\n' "$output"
  if (( status != 0 )); then exit "$status"; fi
  grep -qE '^test result: ok\. 1 passed; 0 failed;' <<<"$output"
done
echo 'TASK065 FAST GATES: PASSED'