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
status=0
list="$(cargo test -p ams-gra-codegen-oms --test ada_component_hiding -- --list 2>&1)" || status=$?
printf '%s\n' "$list"
if (( status != 0 )); then exit "$status"; fi
for entry in \
  'task065_component_hiding_compile_matrix|TASK065 COMPONENT COMPILE MATRIX: PASSED (23 cells)' \
  'task065_component_hiding_planted_failure_controls|TASK065 COMPONENT PLANTED FAILURE CONTROLS: PASSED (4 cells)' \
  'task065_preceding_component_hiding_controls|TASK065 PRECEDING COMPONENT CONTROLS: PASSED (4 cells)' \
  'task065_discriminant_hiding_controls|TASK065 DISCRIMINANT CONTROLS: PASSED (2 cells)' \
  'task065_package_prefix_hiding_control|TASK065 PACKAGE PREFIX CONTROL: PASSED (1 cell)' \
  'task065_component_nonhiding_output_stability|TASK065 COMPONENT NONHIDING STABILITY: PASSED (2 cells)'; do
  name="${entry%%|*}"
  test "$(grep -Fxc "$name: test" <<<"$list")" = 1
  status=0
  output="$(cargo test -p ams-gra-codegen-oms --test ada_component_hiding -- --exact "$name" --nocapture 2>&1)" || status=$?
  printf '%s\n' "$output"
  if (( status != 0 )); then exit "$status"; fi
  grep -qE '^test result: ok\. 1 passed; 0 failed;' <<<"$output"
  grep -Fqx "${entry#*|}" <<<"$output"
done
echo 'TASK065 FAST GATES: PASSED'