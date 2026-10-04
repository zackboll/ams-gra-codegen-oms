#!/usr/bin/env bash
# Task 055: adversarial self-test for scripts/check-ci-split.sh and for the
# marker-check shapes used by the moved Deep CI steps.
#
# 1. The guard passes on the committed workflows.
# 2. Each class of regression (a fetch/Sleet call leaking into Fast CI, a moved
#    check dropped from Deep CI, a `printf | grep -q` reintroduced, the local
#    MSRV check removed) makes the guard fail.
# 3. The here-string marker checks accept known-good libtest output (including
#    the CATEGORY-A marker sharing libtest's "test <name> ... " line), reject
#    output with a required marker removed, and stay correct on large output
#    under pipefail.
set -euo pipefail

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
guard="$repo/scripts/check-ci-split.sh"
fast="$repo/.github/workflows/ci.yml"
deep="$repo/.github/workflows/deep-ci.yml"
sleet="$repo/scripts/run-real-sleet-test.sh"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

passed=0
ok() { passed=$((passed + 1)); }
die() { echo "test-check-ci-split: FAIL: $*" >&2; exit 1; }

expect_pass() { "$guard" "$@" >/dev/null 2>&1 || die "guard rejected: $*"; ok; }
expect_fail() {
  local label="$1"; shift
  if "$guard" "$@" >/dev/null 2>&1; then die "guard accepted mutation: $label"; fi
  ok
}

# ---- 1. committed files pass --------------------------------------------
expect_pass "$fast" "$deep" "$sleet"

# ---- 2. mutations fail --------------------------------------------------
grep -v 'bash scripts/check-task062-fast.sh' "$fast" >"$tmp/fast-no-t062.yml"
expect_fail "fast lost Task062 gate" "$tmp/fast-no-t062.yml" "$deep" "$sleet"
grep -v 'bash scripts/check-task062-pinned.sh' "$deep" >"$tmp/deep-no-t062.yml"
expect_fail "deep lost Task062 gate" "$fast" "$tmp/deep-no-t062.yml" "$sleet"
{ cat "$fast"; printf '      - run: bash scripts/check-task062-pinned.sh\n'; } >"$tmp/fast-t062.yml"
expect_fail "fast runs Task062 pinned gate" "$tmp/fast-t062.yml" "$deep" "$sleet"
{ cat "$fast"; printf '      - run: cargo test --test uci_ipv6_address\n'; } >"$tmp/fast-t062-direct.yml"
expect_fail "fast runs Task062 pinned target directly" "$tmp/fast-t062-direct.yml" "$deep" "$sleet"
sed 's/task060_probes::generated_group_and_carrier_composes_with_production_service_codec/generated_group_and_carrier_composes_with_production_service_codec/g' "$fast" >"$tmp/fast-unqualified-and.yml"
expect_fail "Task060 AND name lost module qualification" "$tmp/fast-unqualified-and.yml" "$deep" "$sleet"
# A real-UCI fetch leaks into Fast CI.
{ cat "$fast"; printf '      - run: scripts/fetch-pinned-uci-2.5.sh "$RUNNER_TEMP/u"\n'; } >"$tmp/fast-uci25.yml"
expect_fail "fast fetches UCI 2.5" "$tmp/fast-uci25.yml" "$deep" "$sleet"
{ cat "$fast"; printf '      - run: scripts/fetch-pinned-uci-2.6.sh "$RUNNER_TEMP/u"\n'; } >"$tmp/fast-uci26.yml"
expect_fail "fast fetches UCI 2.6" "$tmp/fast-uci26.yml" "$deep" "$sleet"
{ cat "$fast"; printf '      - run: scripts/run-real-sleet-test.sh\n'; } >"$tmp/fast-sleet.yml"
expect_fail "fast runs real Sleet" "$tmp/fast-sleet.yml" "$deep" "$sleet"
# A comment that merely mentions the scripts is fine.
{ cat "$fast"; printf '      # see scripts/run-real-sleet-test.sh in deep-ci.yml\n'; } >"$tmp/fast-comment.yml"
expect_pass "$tmp/fast-comment.yml" "$deep" "$sleet"
# The PR-side local MSRV check or the final workspace test disappears.
grep -v 'cargo +1.95.0 check --locked -p ams-gra-oms-runtime-api' "$fast" >"$tmp/fast-nomsrv.yml"
expect_fail "fast lost local MSRV" "$tmp/fast-nomsrv.yml" "$deep" "$sleet"
# Task 056: the real-UCI generated-support gate migrates into Fast CI.
{ cat "$fast"; printf '      - run: cargo test -p ams-gra-codegen-oms --test uci_generated_support\n'; } >"$tmp/fast-t056.yml"
expect_fail "fast runs the Task 056 real-UCI gate" "$tmp/fast-t056.yml" "$deep" "$sleet"
# Task 057: the real-UCI duration gate migrates into Fast CI.
{ cat "$fast"; printf '      - run: cargo test -p ams-gra-codegen-oms --test uci_duration\n'; } >"$tmp/fast-t057.yml"
expect_fail "fast runs the Task 057 real-UCI gate" "$tmp/fast-t057.yml" "$deep" "$sleet"
# Task 058: the real-UCI bounded-ASCII gate migrates into Fast CI.
{ cat "$fast"; printf '      - run: cargo test -p ams-gra-codegen-oms --test uci_bounded_ascii_string\n'; } >"$tmp/fast-t058.yml"
expect_fail "fast runs the Task 058 real-UCI gate" "$tmp/fast-t058.yml" "$deep" "$sleet"
# Task 059 must also remain in Deep CI.
{ cat "$fast"; printf '      - run: cargo test -p ams-gra-codegen-oms --test uci_structured_ascii\n'; } >"$tmp/fast-t059.yml"
expect_fail "fast runs the Task 059 real-UCI gate" "$tmp/fast-t059.yml" "$deep" "$sleet"
grep -v -- '- run: cargo test --workspace' "$fast" >"$tmp/fast-notest.yml"
expect_fail "fast lost workspace test" "$tmp/fast-notest.yml" "$deep" "$sleet"

# Every moved Deep CI check is individually load-bearing.
{ cat "$fast"; printf '      - run: bash scripts/check-task061-pinned.sh\n'; } >"$tmp/fast-t061.yml"
expect_fail "fast runs Task061 pinned gate" "$tmp/fast-t061.yml" "$deep" "$sleet"
grep -v 'bash scripts/check-task061-fast.sh' "$fast" >"$tmp/fast-no-t061.yml"
expect_fail "fast lost Task061 gate" "$tmp/fast-no-t061.yml" "$deep" "$sleet"
for line in \
  'bash scripts/check-task061-pinned.sh' \
  'python3 scripts/check-task061-service-impact.py target/release/ams-gra-codegen-oms' \
  'UCI 2.6 BINARY PROVENANCE INVENTORY: PASSED' \
  'UCI 2.5 CONSTRAINED BINARY MESSAGE IMPACT: PASSED' \
  'UCI 2.6 WHOLE-SCHEMA FIRST BLOCKER PROBE: PASSED' \
  'REAL CATEGORY-A MEMBER REMAPPING: PASSED' \
  'fetch-pinned-uci-2.6.sh' \
  'scripts/run-real-sleet-test.sh' \
  'REAL SUBSYSTEMSTREAM HEXBINARY CODEC: PASSED' \
  'cargo +1.95.0 check --locked -p ams-gra-oms-runtime-rust-facade-tests' \
  '--test uci_generated_support' \
  'UCI 2.5 ORDEROFBATTLE GENERATED SUPPORT PARITY: PASSED' \
  'UCI 2.6 ORDEROFBATTLE GENERATED SUPPORT PARITY: PASSED' \
  'UCI 2.5 CATEGORY-A GENERATED SUPPORT PARITY: PASSED' \
  '--test uci_duration' \
  'UCI 2.5 DURATION INVENTORY: PASSED' \
  'UCI 2.6 DURATION INVENTORY: PASSED' \
  'UCI 2.5 DURATION MESSAGE IMPACT: RECORDED' \
  'UCI 2.6 DURATION MESSAGE IMPACT: RECORDED' \
  'REAL CATEGORY-A DURATION SERVICE: PASSED' \
  '--test uci_bounded_ascii_string' \
  '--test uci_structured_ascii' \
  '--test uci_alternating_admission' \
  '--test uci_alternating_after' \
  'UCI 2.5 TASK060 PROJECTED MESSAGE IMPACT: PASSED' \
  'UCI 2.6 TASK060 PROJECTED MESSAGE IMPACT: PASSED' \
  'UCI 2.5 TASK060 OrderOfBattle COMPILER CODEC VERTICAL: PASSED' \
  'UCI 2.6 TASK060 OrderOfBattle COMPILER CODEC VERTICAL: PASSED' \
  'UCI 2.5 TASK060 SMTI_SettingsCommand COMPILER CODEC VERTICAL: PASSED' \
  'UCI 2.6 TASK060 SMTI_SettingsCommand COMPILER CODEC VERTICAL: PASSED' \
  'UCI 2.5 STRUCTURED ASCII INVENTORY: PASSED' \
  'UCI 2.6 STRUCTURED ASCII INVENTORY: PASSED' \
  'UCI 2.5 STRUCTURED ASCII MESSAGE IMPACT: RECORDED' \
  'UCI 2.6 STRUCTURED ASCII MESSAGE IMPACT: RECORDED' \
  'UCI 2.5 BOUNDED ASCII INVENTORY: PASSED' \
  'UCI 2.6 BOUNDED ASCII INVENTORY: PASSED' \
  'UCI 2.5 BOUNDED ASCII MESSAGE IMPACT: RECORDED' \
  'UCI 2.6 BOUNDED ASCII MESSAGE IMPACT: RECORDED' \
  'REAL NEWLY-READY BOUNDED ASCII SERVICE: PASSED' \
  'REAL AMTI_SETTINGSCOMMAND EMPTYTYPE CODEC: PASSED' \
  'REAL FILEMETADATA STRUCTURED ASCII CODEC: PASSED'; do
  grep -Fv -- "$line" "$deep" >"$tmp/deep-drop.yml"
  expect_fail "deep dropped: $line" "$fast" "$tmp/deep-drop.yml" "$sleet"
done
# UCI 2.6 fetched twice in one workflow.
{ cat "$deep"; printf '          scripts/fetch-pinned-uci-2.6.sh "$RUNNER_TEMP/again"\n'; } >"$tmp/deep-refetch.yml"
expect_fail "deep refetches UCI 2.6" "$fast" "$tmp/deep-refetch.yml" "$sleet"
for target in uci_alternating_admission uci_alternating_after; do
  { cat "$fast"; printf '      - run: cargo test -p ams-gra-codegen-oms --test %s\n' "$target"; } >"$tmp/fast-task060.yml"
  expect_fail "Fast leaked Task 060 $target" "$tmp/fast-task060.yml" "$deep" "$sleet"
done
# `printf | grep -q` reintroduced in Deep CI or in the Sleet harness.
{ cat "$deep"; printf "          printf '%%s\\\\n' \"\$output\" | grep -q '^UCI 2.5 BINARY PROVENANCE INVENTORY: PASSED\$'\n"; } >"$tmp/deep-pipe.yml"
expect_fail "deep printf|grep -q" "$fast" "$tmp/deep-pipe.yml" "$sleet"
{ cat "$sleet"; printf "printf '%%s\\\\n' \"\$output\" | grep -qE 'x'\n"; } >"$tmp/sleet-pipe.sh"
expect_fail "sleet printf|grep -qE" "$fast" "$deep" "$tmp/sleet-pipe.sh"

# Task 064: losing compact pinned integrity or single-pass evidence must fail.
sed '/task064_real_uci_generated_support_binding/d' "$deep" >"$tmp/deep-task064.yml"
expect_fail "Task064 pinned test removed" "$fast" "$tmp/deep-task064.yml" "$sleet"
sed '/UCI 2.5 GENERATED SUPPORT BINDING: PASSED/d' "$deep" >"$tmp/deep-task064-marker.yml"
expect_fail "Task064 pinned marker removed" "$fast" "$tmp/deep-task064-marker.yml" "$sleet"
sed '/TASK064 SINGLE PROJECTION EXPANSION ANALYSIS: PASSED/d' "$fast" >"$tmp/fast-task064.yml"
expect_fail "Task064 instrumentation removed" "$tmp/fast-task064.yml" "$deep" "$sleet"

# ---- 3. marker-check shapes (verbatim from deep-ci.yml job real-uci) ------
task054_markers() {
  set -euo pipefail
  local output="$1"
  grep -Fqx 'UCI 2.5 MEMBER IDENTIFIER INVENTORY: PASSED' <<<"$output"
  grep -Fqx 'UCI 2.6 MEMBER IDENTIFIER INVENTORY: PASSED' <<<"$output"
  grep -Fqx 'UCI 2.5 WHOLE-SCHEMA FIRST BLOCKER PROBE: PASSED' <<<"$output"
  grep -Fqx 'UCI 2.6 WHOLE-SCHEMA FIRST BLOCKER PROBE: PASSED' <<<"$output"
  grep -Eqx '(test task054_real_uci_category_a_selection_generates_and_compiles \.\.\. )?UCI 2\.5 REAL CATEGORY-A MEMBER REMAPPING: PASSED' <<<"$output"
  grep -qE '^test result: ok\. 4 passed' <<<"$output"
}
task052_markers() {
  set -euo pipefail
  local output="$1"
  grep -Fqx 'UCI 2.5 BINARY PROVENANCE INVENTORY: PASSED' <<<"$output"
  grep -Fqx 'UCI 2.6 BINARY PROVENANCE INVENTORY: PASSED' <<<"$output"
  grep -qE '^test result: ok\. 2 passed' <<<"$output"
}
task053_markers() {
  set -euo pipefail
  local output="$1"
  grep -Fqx 'UCI 2.5 CONSTRAINED BINARY INVENTORY: PASSED' <<<"$output"
  grep -Fqx 'UCI 2.6 CONSTRAINED BINARY INVENTORY: PASSED' <<<"$output"
  grep -Fqx 'UCI 2.5 CONSTRAINED BINARY MESSAGE IMPACT: PASSED' <<<"$output"
  grep -Fqx 'UCI 2.6 CONSTRAINED BINARY MESSAGE IMPACT: PASSED' <<<"$output"
  grep -qE '^test result: ok\. 3 passed' <<<"$output"
}
task056_markers() {
  set -euo pipefail
  local output="$1"
  grep -Fqx 'UCI 2.5 ORDEROFBATTLE GENERATED SUPPORT PARITY: PASSED' <<<"$output"
  grep -Fqx 'UCI 2.6 ORDEROFBATTLE GENERATED SUPPORT PARITY: PASSED' <<<"$output"
  grep -Fqx 'UCI 2.5 CATEGORY-A GENERATED SUPPORT PARITY: PASSED' <<<"$output"
  grep -qE '^test result: ok\. 2 passed' <<<"$output"
}
task057_markers() {
  set -euo pipefail
  local output="$1"
  grep -Fqx 'UCI 2.5 DURATION INVENTORY: PASSED' <<<"$output"
  grep -Fqx 'UCI 2.6 DURATION INVENTORY: PASSED' <<<"$output"
  grep -Fqx 'UCI 2.5 DURATION MESSAGE IMPACT: RECORDED' <<<"$output"
  grep -Fqx 'UCI 2.6 DURATION MESSAGE IMPACT: RECORDED' <<<"$output"
  grep -Eqx '(test task057_real_uci_category_a_log_generates_and_compiles \.\.\. )?UCI 2\.5 REAL CATEGORY-A DURATION SERVICE: PASSED' <<<"$output"
  grep -qE '^test result: ok\. 4 passed' <<<"$output"
}
task058_markers() {
  set -euo pipefail
  local output="$1"
  grep -Fqx 'UCI 2.5 BOUNDED ASCII INVENTORY: PASSED' <<<"$output"
  grep -Fqx 'UCI 2.6 BOUNDED ASCII INVENTORY: PASSED' <<<"$output"
  grep -Fqx 'UCI 2.5 BOUNDED ASCII MESSAGE IMPACT: RECORDED' <<<"$output"
  grep -Fqx 'UCI 2.6 BOUNDED ASCII MESSAGE IMPACT: RECORDED' <<<"$output"
  grep -Eqx '(test task058_real_uci_newly_ready_service_generates_and_compiles \.\.\. )?UCI 2\.5 REAL NEWLY-READY BOUNDED ASCII SERVICE: PASSED' <<<"$output"
  grep -qE '^test result: ok\. 5 passed' <<<"$output"
}
task060_markers() {
  set -euo pipefail
  local output="$1"
  for release in 2.5 2.6; do
    grep -Fqx "UCI $release TASK060 PROJECTED MESSAGE IMPACT: PASSED" <<<"$output"
    grep -Fqx "UCI $release TASK060 OrderOfBattle COMPILER CODEC VERTICAL: PASSED" <<<"$output"
    grep -Fqx "UCI $release TASK060 SMTI_SettingsCommand COMPILER CODEC VERTICAL: PASSED" <<<"$output"
  done
  grep -qE '^test result: ok\. 3 passed' <<<"$output"
}
# Run a marker-check function as a PLAIN statement (never inside if/||/&&):
# bash disables errexit for everything in a conditional context, subshells
# included, which would let all but the last grep fail silently.
rc=0
run_markers() {
  set +e
  ( "$@" ) >/dev/null 2>&1
  rc=$?
  set -e
}
must_accept() { run_markers "$@"; [[ "$rc" -eq 0 ]] || die "$1 rejected known-good output"; ok; }
must_reject() {
  local label="$1"; shift
  run_markers "$@"; [[ "$rc" -ne 0 ]] || die "$1 accepted bad output: $label"; ok
}
# Markers FIRST, then a large tail: `printf | grep -q` would exit early here
# and could SIGPIPE the writer; here-strings must still pass.
noise="$(for _ in $(seq 1 20000); do echo 'inventory row: SomeType.SomeMember -> Some_Member'; done)"

good060="UCI 2.5 TASK060 PROJECTED MESSAGE IMPACT: PASSED
UCI 2.6 TASK060 PROJECTED MESSAGE IMPACT: PASSED
UCI 2.5 TASK060 OrderOfBattle COMPILER CODEC VERTICAL: PASSED
UCI 2.6 TASK060 OrderOfBattle COMPILER CODEC VERTICAL: PASSED
UCI 2.5 TASK060 SMTI_SettingsCommand COMPILER CODEC VERTICAL: PASSED
UCI 2.6 TASK060 SMTI_SettingsCommand COMPILER CODEC VERTICAL: PASSED
test result: ok. 3 passed; 0 failed
$noise"
must_accept task060_markers "$good060"
for marker in \
  'UCI 2.6 TASK060 PROJECTED MESSAGE IMPACT: PASSED' \
  'UCI 2.6 TASK060 OrderOfBattle COMPILER CODEC VERTICAL: PASSED' \
  'UCI 2.5 TASK060 SMTI_SettingsCommand COMPILER CODEC VERTICAL: PASSED' \
  'test result: ok. 3 passed'; do
  must_reject "missing Task060 $marker" task060_markers "${good060/"$marker"/}"
done
must_reject "prefixed Task060 marker" task060_markers "${good060/UCI 2.5 TASK060 PROJECTED/x UCI 2.5 TASK060 PROJECTED}"

good054="running 4 tests
UCI 2.5 MEMBER IDENTIFIER INVENTORY: PASSED
test task054_real_uci_2_5 ... ok
UCI 2.6 MEMBER IDENTIFIER INVENTORY: PASSED
UCI 2.5 WHOLE-SCHEMA FIRST BLOCKER PROBE: PASSED
UCI 2.6 WHOLE-SCHEMA FIRST BLOCKER PROBE: PASSED
test task054_real_uci_category_a_selection_generates_and_compiles ... UCI 2.5 REAL CATEGORY-A MEMBER REMAPPING: PASSED
ok
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
$noise"
must_accept task054_markers "$good054"
standalone="${good054/test task054_real_uci_category_a_selection_generates_and_compiles ... /}"
must_accept task054_markers "$standalone"
for marker in \
  'UCI 2.6 MEMBER IDENTIFIER INVENTORY: PASSED' \
  'UCI 2.5 WHOLE-SCHEMA FIRST BLOCKER PROBE: PASSED' \
  'UCI 2.5 REAL CATEGORY-A MEMBER REMAPPING: PASSED' \
  'test result: ok. 4 passed'; do
  must_reject "missing $marker" task054_markers "${good054/"$marker"/}"
done
wrong="${good054/test task054_real_uci_category_a_selection_generates_and_compiles/test some_other_test}"
must_reject "foreign CATEGORY-A prefix" task054_markers "$wrong"

good052="UCI 2.5 BINARY PROVENANCE INVENTORY: PASSED
UCI 2.6 BINARY PROVENANCE INVENTORY: PASSED
test result: ok. 2 passed; 0 failed
$noise"
must_accept task052_markers "$good052"
must_reject "missing 2.6 marker" task052_markers "${good052/UCI 2.6 BINARY PROVENANCE INVENTORY: PASSED/}"
must_reject "non-whole-line marker" task052_markers "${good052/UCI 2.5 BINARY/x UCI 2.5 BINARY}"

good053="UCI 2.5 CONSTRAINED BINARY INVENTORY: PASSED
UCI 2.6 CONSTRAINED BINARY INVENTORY: PASSED
UCI 2.5 CONSTRAINED BINARY MESSAGE IMPACT: PASSED
UCI 2.6 CONSTRAINED BINARY MESSAGE IMPACT: PASSED
test result: ok. 3 passed; 0 failed
$noise"
must_accept task053_markers "$good053"
must_reject "missing 053 test result" task053_markers "${good053/test result: ok. 3 passed/}"
must_reject "missing message-impact marker" task053_markers "${good053/UCI 2.6 CONSTRAINED BINARY MESSAGE IMPACT: PASSED/}"

# Task 056: each test prints a detail line first, which is the one that lands
# on libtest's unterminated "test <name> ... " line, so every marker is whole.
good056="running 2 tests
test task056_real_uci_category_a_readiness_matches_generation ... UCI 2.5 CATEGORY-A ada: READY+generated 34 NOT READY [\"OrderOfBattle\"]
UCI 2.5 CATEGORY-A rust: READY+generated 34 NOT READY [\"OrderOfBattle\"]
UCI 2.5 CATEGORY-A GENERATED SUPPORT PARITY: PASSED
ok
test task056_real_uci_order_of_battle_support_parity ... UCI 2.5 ada ORDEROFBATTLE: selected 55
UCI 2.5 ORDEROFBATTLE GENERATED SUPPORT PARITY: PASSED
UCI 2.6 ORDEROFBATTLE GENERATED SUPPORT PARITY: PASSED
ok
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
$noise"
must_accept task056_markers "$good056"
for marker in \
  'UCI 2.5 ORDEROFBATTLE GENERATED SUPPORT PARITY: PASSED' \
  'UCI 2.6 ORDEROFBATTLE GENERATED SUPPORT PARITY: PASSED' \
  'UCI 2.5 CATEGORY-A GENERATED SUPPORT PARITY: PASSED' \
  'test result: ok. 2 passed'; do
  must_reject "missing $marker" task056_markers "${good056/"$marker"/}"
done
# A marker glued to libtest's prefix is not a whole line and must not count.
glued="${good056/UCI 2.5 ada ORDEROFBATTLE: selected 55
/}"
must_reject "prefixed 056 marker" task056_markers "$glued"

# Task 057: each test's FIRST printed line is a detail row, which is the one
# libtest's unterminated "test <name> ... " prefix lands on; markers are whole.
good057="running 4 tests
test task057_real_uci_category_a_log_generates_and_compiles ... UCI 2.5 REAL CATEGORY-A DURATION SERVICE: PASSED
ok
test task057_real_uci_2_5_duration_inventory ... UCI 2.5 NAMED DURATION: DurationType | x
UCI 2.5 DURATION SUMMARY: named=1 direct_refs=9
UCI 2.5 DURATION INVENTORY: PASSED
ok
test task057_real_uci_2_6_duration_inventory ... UCI 2.6 NAMED DURATION: DurationType | x
UCI 2.6 DURATION INVENTORY: PASSED
ok
test task057_real_uci_duration_message_impact ... UCI 2.5 NAMED DURATION: DurationType | x
UCI 2.5 DURATION MESSAGE IMPACT: RECORDED
UCI 2.6 DURATION MESSAGE IMPACT: RECORDED
ok
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
$noise"
must_accept task057_markers "$good057"
# The category-A marker also passes when it lands on its own line.
must_accept task057_markers "${good057/test task057_real_uci_category_a_log_generates_and_compiles ... /}"
# ...but not behind some OTHER test's prefix.
must_reject "foreign 057 CATEGORY-A prefix" task057_markers \
  "${good057/test task057_real_uci_category_a_log_generates_and_compiles/test some_other_test}"
for marker in \
  'UCI 2.5 DURATION INVENTORY: PASSED' \
  'UCI 2.6 DURATION INVENTORY: PASSED' \
  'UCI 2.5 DURATION MESSAGE IMPACT: RECORDED' \
  'UCI 2.6 DURATION MESSAGE IMPACT: RECORDED' \
  'UCI 2.5 REAL CATEGORY-A DURATION SERVICE: PASSED' \
  'test result: ok. 4 passed'; do
  must_reject "missing $marker" task057_markers "${good057/"$marker"/}"
done
# A marker glued to libtest's prefix is not a whole line and must not count.
glued057="${good057/UCI 2.6 NAMED DURATION: DurationType | x
/}"
must_reject "prefixed 057 marker" task057_markers "$glued057"

# Task 058: same shape as Task 057 -- detail rows first, whole-line markers,
# the service marker possibly on libtest's "test <name> ... " line.
good058="running 5 tests
test task058_closed_schema_ada_gap_evidence ... UCI 2.5 CLOSED-SCHEMA ADA GAP: PASSED ["Authorization", "AuthorizationRequest", "CommSupportActivity"]
UCI 2.6 CLOSED-SCHEMA ADA GAP: PASSED ["Authorization", "AuthorizationRequest", "CommSupportActivity"]
ok
test task058_real_uci_newly_ready_service_generates_and_compiles ... UCI 2.5 REAL NEWLY-READY BOUNDED ASCII SERVICE: PASSED
ok
test task058_real_uci_2_5_bounded_ascii_inventory ... UCI 2.5 CONSTRAINED STRING: AircraftIdentifierType | x
UCI 2.5 BOUNDED ASCII SUMMARY: members=61 alphabets=19 still-unsupported=46
UCI 2.5 BOUNDED ASCII INVENTORY: PASSED
ok
test task058_real_uci_2_6_bounded_ascii_inventory ... UCI 2.6 CONSTRAINED STRING: AircraftIdentifierType | x
UCI 2.6 BOUNDED ASCII INVENTORY: PASSED
ok
test task058_real_uci_bounded_ascii_message_impact ... UCI 2.5 BOUNDED ASCII MESSAGE IMPACT: Log x
UCI 2.5 BOUNDED ASCII MESSAGE IMPACT: RECORDED
UCI 2.6 BOUNDED ASCII MESSAGE IMPACT: RECORDED
ok
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
$noise"
must_accept task058_markers "$good058"
must_accept task058_markers "${good058/test task058_real_uci_newly_ready_service_generates_and_compiles ... /}"
must_reject "foreign 058 NEWLY-READY prefix" task058_markers \
  "${good058/test task058_real_uci_newly_ready_service_generates_and_compiles/test some_other_test}"
for marker in \
  'UCI 2.5 BOUNDED ASCII INVENTORY: PASSED' \
  'UCI 2.6 BOUNDED ASCII INVENTORY: PASSED' \
  'UCI 2.5 BOUNDED ASCII MESSAGE IMPACT: RECORDED' \
  'UCI 2.6 BOUNDED ASCII MESSAGE IMPACT: RECORDED' \
  'UCI 2.5 REAL NEWLY-READY BOUNDED ASCII SERVICE: PASSED' \
  'test result: ok. 5 passed'; do
  must_reject "missing $marker" task058_markers "${good058/"$marker"/}"
done
glued058="${good058/UCI 2.6 CONSTRAINED STRING: AircraftIdentifierType | x
/}"
must_reject "prefixed 058 marker" task058_markers "$glued058"

task059_markers() {
  set -euo pipefail
  local output="$1"
  grep -Fqx 'UCI 2.5 STRUCTURED ASCII INVENTORY: PASSED' <<<"$output"
  grep -Fqx 'UCI 2.6 STRUCTURED ASCII INVENTORY: PASSED' <<<"$output"
  grep -Fqx 'UCI 2.5 STRUCTURED ASCII MESSAGE IMPACT: RECORDED' <<<"$output"
  grep -Fqx 'UCI 2.6 STRUCTURED ASCII MESSAGE IMPACT: RECORDED' <<<"$output"
  grep -qE '^test result: ok\. 2 passed' <<<"$output"
}
good059="test task059_real_uci_inventory_and_coverage ... UCI 2.5 STRUCTURED ASCII: x
UCI 2.5 STRUCTURED ASCII INVENTORY: PASSED
UCI 2.6 STRUCTURED ASCII INVENTORY: PASSED
test task059_real_uci_message_impact ... UCI 2.5 STRUCTURED ASCII MESSAGE: x
UCI 2.5 STRUCTURED ASCII MESSAGE IMPACT: RECORDED
UCI 2.6 STRUCTURED ASCII MESSAGE IMPACT: RECORDED
test result: ok. 2 passed; 0 failed"
must_accept task059_markers "$good059"
for marker in \
  'UCI 2.5 STRUCTURED ASCII INVENTORY: PASSED' \
  'UCI 2.6 STRUCTURED ASCII INVENTORY: PASSED' \
  'UCI 2.5 STRUCTURED ASCII MESSAGE IMPACT: RECORDED' \
  'UCI 2.6 STRUCTURED ASCII MESSAGE IMPACT: RECORDED' \
  'test result: ok. 2 passed'; do
  must_reject "missing $marker" task059_markers "${good059/"$marker"/}"
done
must_reject "prefixed 059 marker" task059_markers "${good059/UCI 2.6 STRUCTURED ASCII INVENTORY: PASSED/foreign UCI 2.6 STRUCTURED ASCII INVENTORY: PASSED}"

# The Deep CI marker lines must be exactly the shapes tested above.
for shape in \
  "grep -Eqx '(test task054_real_uci_category_a_selection_generates_and_compiles \\.\\.\\. )?UCI 2\\.5 REAL CATEGORY-A MEMBER REMAPPING: PASSED' <<<\"\$output\"" \
  "grep -Fqx 'UCI 2.6 BINARY PROVENANCE INVENTORY: PASSED' <<<\"\$output\"" \
  "grep -Fqx 'UCI 2.6 CONSTRAINED BINARY MESSAGE IMPACT: PASSED' <<<\"\$output\"" \
  "grep -Fqx 'UCI 2.6 ORDEROFBATTLE GENERATED SUPPORT PARITY: PASSED' <<<\"\$output\"" \
  "grep -Fqx 'UCI 2.5 CATEGORY-A GENERATED SUPPORT PARITY: PASSED' <<<\"\$output\"" \
  "grep -Fqx 'UCI 2.6 DURATION INVENTORY: PASSED' <<<\"\$output\"" \
  "grep -Fqx 'UCI 2.6 DURATION MESSAGE IMPACT: RECORDED' <<<\"\$output\"" \
  "grep -Eqx '(test task057_real_uci_category_a_log_generates_and_compiles \\.\\.\\. )?UCI 2\\.5 REAL CATEGORY-A DURATION SERVICE: PASSED' <<<\"\$output\"" \
  "grep -Fqx 'UCI 2.6 BOUNDED ASCII INVENTORY: PASSED' <<<\"\$output\"" \
  "grep -Fqx 'UCI 2.6 BOUNDED ASCII MESSAGE IMPACT: RECORDED' <<<\"\$output\"" \
  "grep -Fqx 'UCI 2.6 STRUCTURED ASCII INVENTORY: PASSED' <<<\"\$output\"" \
  "grep -Fqx 'UCI 2.6 STRUCTURED ASCII MESSAGE IMPACT: RECORDED' <<<\"\$output\"" \
  "grep -Eqx '(test task058_real_uci_newly_ready_service_generates_and_compiles \\.\\.\\. )?UCI 2\\.5 REAL NEWLY-READY BOUNDED ASCII SERVICE: PASSED' <<<\"\$output\""; do
  grep -Fq -- "$shape" "$deep" || die "deep-ci.yml no longer uses tested shape: $shape"
  ok
done

echo "test-check-ci-split: PASSED ($passed checks)"

