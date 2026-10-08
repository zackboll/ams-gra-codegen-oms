#!/usr/bin/env bash
set -euo pipefail
test -n "${AMS_GRA_UCI_2_5_ROOT:-}"
test -n "${AMS_GRA_UCI_2_6_ROOT:-}"
export AMS_GRA_REQUIRE_TASK065_PINNED=1
export AMS_GRA_REQUIRE_GNAT=1
status=0
list="$(cargo test --release -p ams-gra-codegen-oms --test ada_companion_naming -- --list 2>&1)" || status=$?
printf '%s\n' "$list"
if (( status != 0 )); then exit "$status"; fi
name=task065_pinned_naming_evidence
test "$(grep -Fxc "$name: test" <<<"$list")" = 1
log="$(mktemp "${TMPDIR:-/tmp}/task065-pinned.XXXXXX.log")"
cargo test --release -p ams-gra-codegen-oms --test ada_companion_naming -- --exact "$name" --nocapture >"$log" 2>&1 || status=$?
output="$(cat "$log")"
printf '%s\n' "$output"
if (( status != 0 )); then exit "$status"; fi
grep -qE '^test result: ok\. 1 passed; 0 failed;' <<<"$output"
for release in 2.5 2.6; do
  grep -Fqx "UCI $release TASK065 ORIGINAL COLLISION ATTRIBUTION: PASSED" <<<"$output"
  grep -Fqx "UCI $release TASK065 HIDING INVENTORY: PASSED" <<<"$output"
  grep -Fqx "UCI $release TASK065 NAMING EVIDENCE: PASSED" <<<"$output"
  grep -Fqx "UCI $release TASK065 AUTHORIZATION MODEL API GNAT: PASSED" <<<"$output"
  test "$(grep -c "^FORMER-GAP-PROJECTION"$'\t'"$release"$'\t' <<<"$output")" = 33
  test "$(grep -c "^GAP-COUNT"$'\t'"$release"$'\t'".*"$'\t'"0$" <<<"$output")" = 2
done
echo 'TASK065 PINNED GATES: PASSED'