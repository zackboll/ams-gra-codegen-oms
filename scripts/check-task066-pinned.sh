#!/usr/bin/env bash
set -euo pipefail
test -n "${AMS_GRA_UCI_2_5_ROOT:-}"
test -n "${AMS_GRA_UCI_2_6_ROOT:-}"
export AMS_GRA_REQUIRE_TASK066_PINNED=1
export AMS_GRA_REQUIRE_GNAT=1
status=0
list="$(cargo test --release -p ams-gra-codegen-oms --test unicode_string_authority -- --list 2>&1)" || status=$?
printf '%s\n' "$list"
if (( status != 0 )); then exit "$status"; fi
name=task066_pinned_unicode_authority_inventory
test "$(grep -Fxc "$name: test" <<<"$list")" = 1
output="$(cargo test --release -p ams-gra-codegen-oms --test unicode_string_authority -- --exact "$name" --nocapture 2>&1)" || status=$?
printf '%s\n' "$output"
if (( status != 0 )); then exit "$status"; fi
grep -Fqx "test $name ... ok" <<<"$output"
grep -qE '^test result: ok\. 1 passed; 0 failed;' <<<"$output"
for release in 2.5 2.6; do
  grep -Fqx "UCI $release TASK066 AUTHORITY INVENTORY: PASSED" <<<"$output"
done
python3 scripts/check-task066-vertical.py "${CARGO_TARGET_DIR:-target}/release/ams-gra-codegen-oms"
echo 'TASK066 PINNED GATES: PASSED'