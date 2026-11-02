#!/usr/bin/env bash
# Tasks 049/050: run the generated-facade -> runtime-rust -> sleet-client tests
# against the UNMODIFIED pinned Sleet server. Set AMS_GRA_UCI_2_5_ROOT to the
# pinned UCI 2.5 root to also run the real PositionReport integration.
#
# Builds the `sleet` binary from a fresh checkout of the exact pinned
# revision in a temporary directory (never inside this repository), then
# runs the one real-Sleet test and requires that it actually executed.
#
# Usage: scripts/run-real-sleet-test.sh [existing-sleet-checkout]
set -euo pipefail

SLEET_REPO="https://github.com/open-arsenal/ams-gra-hello-world-sk-infra-sleet"
SLEET_REV="e38f61d8ce0d75c8508434a52f2ed77c69cf6a3b"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"

# RUST-OMS-001: explicit qualification may not silently compile no-UCI stubs.
# The focused mode reuses this exact checkout/build, without running unrelated
# Python qualifications. Default invocation retains all existing tests.
if [ "${AMS_GRA_RUST_LOG_ONLY:-}" = 1 ]; then
  export AMS_GRA_REQUIRE_REAL_UCI_LOG=1
fi
if [ "${AMS_GRA_RUST_RDMA_ONLY:-}" = 1 ]; then
  export AMS_GRA_REQUIRE_REAL_UCI_RDMA=1
fi
if [ "${AMS_GRA_RUST_MULTIMESSAGE_ONLY:-}" = 1 ]; then
  export AMS_GRA_REQUIRE_REAL_UCI_MULTIMESSAGE=1
fi
if [ "${AMS_GRA_RUST_SURVIVABILITY_ONLY:-}" = 1 ]; then
  # All inherited regressions use the same fail-closed build configuration.
  export AMS_GRA_REQUIRE_REAL_UCI_LOG=1
  export AMS_GRA_REQUIRE_REAL_UCI_RDMA=1
  export AMS_GRA_REQUIRE_REAL_UCI_MULTIMESSAGE=1
  export AMS_GRA_REQUIRE_REAL_UCI_SURVIVABILITY=1
fi
if [ "${AMS_GRA_RUST_CLIENT_ONLY:-}" = 1 ]; then
  : "${AMS_GRA_UCI_2_5_ROOT:?pinned UCI 2.5 root required}"
  : "${AMS_GRA_UCI_2_6_ROOT:?pinned UCI 2.6 root required}"
fi
if [ "${AMS_GRA_RUST_DLZ_ONLY:-}" = 1 ]; then
  export AMS_GRA_REQUIRE_REAL_UCI_DLZ=1
fi
if [ -n "${AMS_GRA_REQUIRE_REAL_UCI_LOG:-}${AMS_GRA_REQUIRE_REAL_UCI_RDMA:-}${AMS_GRA_REQUIRE_REAL_UCI_MULTIMESSAGE:-}${AMS_GRA_REQUIRE_REAL_UCI_SURVIVABILITY:-}${AMS_GRA_REQUIRE_REAL_UCI_DLZ:-}" ]; then
  : "${AMS_GRA_UCI_2_5_ROOT:?pinned UCI 2.5 root required}"
  : "${AMS_GRA_UCI_2_6_ROOT:?pinned UCI 2.6 root required}"
fi

if [ $# -ge 1 ]; then
  CHECKOUT="$1"
else
  CHECKOUT="$(mktemp -d)/sleet"
  git clone --quiet "$SLEET_REPO" "$CHECKOUT"
fi
git -C "$CHECKOUT" checkout --quiet --detach "$SLEET_REV"
test "$(git -C "$CHECKOUT" rev-parse HEAD)" = "$SLEET_REV"
# Unmodified: no tracked or untracked changes in the checkout.
test -z "$(git -C "$CHECKOUT" status --porcelain)"

# Build with Sleet's own pinned toolchain (rust-toolchain.toml).
(cd "$CHECKOUT" && cargo build --release --locked -p sleet)
export AMS_GRA_SLEET_BIN="$CHECKOUT/target/release/sleet"

cd "$ROOT"
# RUST-OMS-005: genuine standalone executable acceptance, using this same
# unmodified broker build and the existing independent Task092/093 goldens.
# The existing main-only real-sleet job supplies both roots; no new runner lane.
if [ "${AMS_GRA_RUST_DLZ_ONLY:-}" != 1 ] && [ -n "${AMS_GRA_UCI_2_5_ROOT:-}" ] && [ -n "${AMS_GRA_UCI_2_6_ROOT:-}" ]; then
  manifest="$ROOT/examples/rust-uci-client/Cargo.toml"
  cargo +1.95.0 fmt --manifest-path "$manifest" -- --check
  for release in 2.5 2.6; do
    export AMS_GRA_UCI_RELEASE="$release"
    if [ "$release" = 2.5 ]; then
      export AMS_GRA_UCI_ROOT="$AMS_GRA_UCI_2_5_ROOT"
    else
      export AMS_GRA_UCI_ROOT="$AMS_GRA_UCI_2_6_ROOT"
    fi
    cargo +1.95.0 build --locked --manifest-path "$manifest"
    cargo +1.95.0 check --locked --all-targets --manifest-path "$manifest"
    cargo +1.95.0 clippy --locked --all-targets --manifest-path "$manifest" -- -D warnings
    status=0
    output="$(cargo +1.95.0 test --locked --manifest-path "$manifest" -- \
      --include-ignored --nocapture --test-threads=1 2>&1)" || status=$?
    printf '%s\n' "$output"
    if (( status != 0 )); then exit "$status"; fi
    grep -Fqx "RUST-OMS-005 002.${release#2.}.0 standalone typed/external/OWP/error/unsubscribe/process cleanup: PASSED" <<<"$output"
  done
fi
if [ "${AMS_GRA_RUST_CLIENT_ONLY:-}" = 1 ]; then
  echo "RUST-OMS-005 real pinned Sleet ($SLEET_REV): PASSED"
  exit 0
fi
# run_one TEST_FILE TEST_NAME PASS_LINE: exactly one matching test must run,
# pass, and print its PASSED line (a skip prints SKIPPED and fails here).
run_one() {
  local status=0
  output="$(cargo test -p ams-gra-oms-runtime-rust-facade-tests --test "$1" -- \
    --exact "$2" --nocapture 2>&1)" || status=$?
  printf '%s\n' "$output"
  if (( status != 0 )); then return "$status"; fi
  # Task 055: here-strings, not `printf | grep -q`. Under pipefail a grep that
  # exits on its first match can SIGPIPE a writer still emitting a large
  # output and fail an otherwise-passing check. A here-string has no writer
  # process. `-x` keeps the same whole-line match the `^...$` regex had.
  grep -Fqx -- "$3" <<<"$output"
  grep -qE '^test result: ok\. 1 passed' <<<"$output"
}

if [ -n "${AMS_GRA_UCI_2_5_ROOT:-}" ] && [ -n "${AMS_GRA_UCI_2_6_ROOT:-}" ]; then
  # Task101's independent corpus remains the oracle, not generated output.
  PYTHONDONTWRITEBYTECODE=1 python3 - <<'PY'
import importlib.util
import json
from pathlib import Path
spec = importlib.util.spec_from_file_location('task101_cases', 'scripts/task101/cases.py')
cases = importlib.util.module_from_spec(spec)
spec.loader.exec_module(cases)
for version in ['25', '26']:
    expected = dict(positives=cases.positives(version), negatives=cases.negatives(version))
    actual = json.loads(Path(f'tests/fixtures/rust-oms-006/corpus{version}.json').read_text())
    assert actual == json.loads(json.dumps(expected)), version
PY
  status=0
  output="$(AMS_GRA_REQUIRE_REAL_UCI_DLZ=1 CARGO_PROFILE_RELEASE_BUILD_OVERRIDE_OPT_LEVEL=3 \
    cargo +1.95.0 test --release --locked -p ams-gra-oms-runtime-rust-facade-tests \
    --test real_uci_dlz --test real_dlz_consumer -- --show-output --test-threads=1 2>&1)" || status=$?
  printf '%s\n' "$output"
  if (( status != 0 )); then exit "$status"; fi
  for version in 002.5.0 002.6.0; do
    grep -Fqx "RUST-OMS-006 $version MODEL CODEC BITS AND TIME: PASSED" <<<"$output"
    grep -Fqx "RUST-OMS-006 $version SLEET AND RECOVERY: PASSED" <<<"$output"
  done
  grep -Fqx 'EXTERNAL DLZ 25: PASSED' <<<"$output"
  grep -Fqx 'EXTERNAL DLZ 26: PASSED' <<<"$output"
  grep -qE '^test result: ok\. 5 passed; 0 failed; 0 ignored;' <<<"$output"
  grep -qE '^test result: ok\. 1 passed; 0 failed; 0 ignored;' <<<"$output"
fi
if [ "${AMS_GRA_RUST_DLZ_ONLY:-}" = 1 ]; then
  echo "RUST-OMS-006 real pinned Sleet ($SLEET_REV): PASSED"
  exit 0
fi

if [ -n "${AMS_GRA_UCI_2_5_ROOT:-}" ] && [ -n "${AMS_GRA_UCI_2_6_ROOT:-}" ]; then
  # Release keeps repeated authentic-schema parsing bounded; build.rs runs
  # production readiness, generates each model twice and asserts source parity.
  status=0
  output="$(CARGO_PROFILE_RELEASE_BUILD_OVERRIDE_OPT_LEVEL=3 \
    cargo +1.95.0 test --release --locked -p ams-gra-oms-runtime-rust-facade-tests \
    --test real_uci_log -- --show-output --test-threads=1 2>&1)" || status=$?
  printf '%s\n' "$output"
  if (( status != 0 )); then exit "$status"; fi
  for version in 002.5.0 002.6.0; do
    grep -Fqx "RUST-OMS-001 $version GOLDEN: PASSED" <<<"$output"
    grep -Fqx "RUST-OMS-001 $version SLEET AND RECOVERY: PASSED" <<<"$output"
  done
  grep -qE '^test result: ok\. 4 passed; 0 failed; 0 ignored;' <<<"$output"
  output="$(CARGO_PROFILE_RELEASE_BUILD_OVERRIDE_OPT_LEVEL=3 \
    cargo +1.95.0 test --release --locked -p ams-gra-oms-runtime-rust-facade-tests \
    --test real_rdma_consumer -- --show-output 2>&1)"
  printf '%s\n' "$output"
  grep -Fqx 'EXTERNAL RDMA 25: PASSED' <<<"$output"
  grep -Fqx 'EXTERNAL RDMA 26: PASSED' <<<"$output"
  grep -qE '^test result: ok\. 1 passed; 0 failed; 0 ignored;' <<<"$output"
fi
if [ -n "${AMS_GRA_UCI_2_5_ROOT:-}" ] && [ -n "${AMS_GRA_UCI_2_6_ROOT:-}" ]; then
  status=0
  output="$(AMS_GRA_REQUIRE_REAL_UCI_RDMA=1 CARGO_PROFILE_RELEASE_BUILD_OVERRIDE_OPT_LEVEL=3 \
    cargo +1.95.0 test --release --locked -p ams-gra-oms-runtime-rust-facade-tests \
    --test real_uci_rdma -- --show-output --test-threads=1 2>&1)" || status=$?
  printf '%s\n' "$output"
  if (( status != 0 )); then exit "$status"; fi
  for version in 002.5.0 002.6.0; do
    grep -Fqx "RUST-OMS-002 $version GOLDEN: PASSED" <<<"$output"
    grep -Fqx "RUST-OMS-002 $version SLEET AND RECOVERY: PASSED" <<<"$output"
  done
  grep -qE '^test result: ok\. 4 passed; 0 failed; 0 ignored;' <<<"$output"
fi
if [ -n "${AMS_GRA_UCI_2_5_ROOT:-}" ] && [ -n "${AMS_GRA_UCI_2_6_ROOT:-}" ]; then
  status=0
  output="$(AMS_GRA_REQUIRE_REAL_UCI_MULTIMESSAGE=1 CARGO_PROFILE_RELEASE_BUILD_OVERRIDE_OPT_LEVEL=3 \
    cargo +1.95.0 test --release --locked -p ams-gra-oms-runtime-rust-facade-tests \
    --test real_uci_multimessage -- --show-output --test-threads=1 2>&1)" || status=$?
  printf '%s\n' "$output"
  if (( status != 0 )); then exit "$status"; fi
  for version in 002.5.0 002.6.0; do
    grep -Fqx "RUST-OMS-003 $version SINGLE CONNECTION AND ISOLATION: PASSED" <<<"$output"
  done
  grep -qE '^test result: ok\. 2 passed; 0 failed; 0 ignored;' <<<"$output"
fi
if [ -n "${AMS_GRA_UCI_2_5_ROOT:-}" ] && [ -n "${AMS_GRA_UCI_2_6_ROOT:-}" ]; then
  # Keep the independent Task104/105 corpora byte-for-value authoritative.
  # No generated encoding is used to author the expected documents.
  PYTHONDONTWRITEBYTECODE=1 python3 - <<'PY'
import importlib.util
import json
from pathlib import Path
for version, task in [('25', '104'), ('26', '105')]:
    source = f'scripts/task{task}/cases.py'
    spec = importlib.util.spec_from_file_location('cases', source)
    cases = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(cases)
    expected = dict(source=source, positives=cases.positives(version),
                    invalid=[(label, doc, *cases.BROKER_DETAILS[label])
                             for label, doc in cases.invalid_documents(version)])
    actual = json.loads(Path(f'tests/fixtures/rust-oms-004/corpus{version}.json').read_text())
    assert actual == json.loads(json.dumps(expected)), version
PY
  status=0
  output="$(AMS_GRA_REQUIRE_REAL_UCI_SURVIVABILITY=1 CARGO_PROFILE_RELEASE_BUILD_OVERRIDE_OPT_LEVEL=3 \
    cargo +1.95.0 test --release --locked -p ams-gra-oms-runtime-rust-facade-tests \
    --test real_uci_survivability -- --show-output --test-threads=1 2>&1)" || status=$?
  printf '%s\n' "$output"
  if (( status != 0 )); then exit "$status"; fi
  for version in 002.5.0 002.6.0; do
    grep -Fqx "RUST-OMS-004 $version MODEL AND CODEC: PASSED" <<<"$output"
    grep -Fqx "RUST-OMS-004 $version SINGLE CONNECTION AND ISOLATION: PASSED" <<<"$output"
  done
  grep -qE '^test result: ok\. 4 passed; 0 failed; 0 ignored;' <<<"$output"
  # TLS teardown barriers prove actual joining, without a procfs exit race.
  output="$(CARGO_PROFILE_RELEASE_BUILD_OVERRIDE_OPT_LEVEL=3 \
    cargo +1.95.0 test --release --locked -p ams-gra-oms-runtime-rust-facade-tests \
    --test runtime_lifecycle -- --show-output --test-threads=1 2>&1)"
  printf '%s\n' "$output"
  grep -qE '^test result: ok\. 7 passed; 0 failed; 0 ignored;' <<<"$output"
fi
if [ "${AMS_GRA_RUST_SURVIVABILITY_ONLY:-}" = 1 ]; then
  echo "RUST-OMS-004 real pinned Sleet ($SLEET_REV): PASSED"
  exit 0
fi
if [ "${AMS_GRA_RUST_MULTIMESSAGE_ONLY:-}" = 1 ]; then
  echo "RUST-OMS-003 real pinned Sleet ($SLEET_REV): PASSED"
  exit 0
fi
if [ "${AMS_GRA_RUST_RDMA_ONLY:-}" = 1 ]; then
  echo "RUST-OMS-002 real pinned Sleet ($SLEET_REV): PASSED"
  exit 0
fi
if [ "${AMS_GRA_RUST_LOG_ONLY:-}" = 1 ]; then
  echo "RUST-OMS-001 real pinned Sleet ($SLEET_REV): PASSED"
  exit 0
fi

# PY-005 reuses this exact unmodified broker build; no second broker checkout
# or build. Python package with websockets==17.2 is installed by Deep CI.
bash "$ROOT/scripts/check-py005-sleet.sh"
# Bind the existing Python discovery tests to this harness's just-built CLI.
# Required by the PY-012/PY-013 transport probes as well as source-bound receipts.
export AMS_GRA_GENERATOR="$ROOT/target/debug/ams-gra-codegen-oms"
for interpreter in ${PY006_PYTHONS:-${PYTHON:-python3}}; do
  PYTHON="$interpreter" bash "$ROOT/scripts/check-py006-sleet.sh"
  PYTHON="$interpreter" bash "$ROOT/scripts/check-py007-sleet.sh"
  PYTHON="$interpreter" bash "$ROOT/scripts/check-py008-sleet.sh"
  PYTHON="$interpreter" bash "$ROOT/scripts/check-py009-sleet.sh"
  PYTHON="$interpreter" bash "$ROOT/scripts/check-py010-sleet.sh"
  "$interpreter" "$ROOT/python/qualification/run.py" sleet
  "$interpreter" -O "$ROOT/python/qualification/run.py" sleet
  "$interpreter" "$ROOT/scripts/run-owned-python.py" sleet
  "$interpreter" -O "$ROOT/scripts/run-owned-python.py" sleet
done

# Task 049: generated facade + HANDWRITTEN codec (proves the runtime seam).
run_one real_sleet task049_generated_facade_round_trips_through_real_sleet \
  'REAL SLEET: PASSED'
# Task 050: generated facade + GENERATED codec, synthetic OAM fixture.
run_one generated_codec_sleet task050_generated_codec_round_trips_through_real_sleet \
  'REAL SLEET GENERATED CODEC: PASSED'
# Task 051: COMPATIBILITY PROBE, qualified NON-OAM generated codec. Pinned
# Sleet keys global elements by bare local name, so it rejects the
# spec-correct Clark-form names; the probe asserts and records that exact
# outcome (see docs/task-051-member-qname-provenance.md).
run_one generated_codec_sleet_non_oam task051_non_oam_generated_codec_probe_against_real_sleet \
  'SLEET NON-OAM PROBE: RECORDED'
# Task 052: generated xs:hexBinary codec (synthetic OAM fixture); also records
# that pinned Sleet validates hexBinary only as a JSON string.
run_one generated_codec_sleet_hexbinary task052_hex_binary_generated_codec_round_trips_through_real_sleet \
  'REAL SLEET GENERATED HEXBINARY CODEC: PASSED'
# Task 053: generated constrained named Binary carriers (synthetic OAM fixture
# constrained-binary-sleet.xsd); records that pinned Sleet counts hexBinary
# length facets in CHARACTERS, not octets (see docs/task-053-*).
run_one generated_codec_sleet_constrained_binary task053_constrained_binary_generated_codec_round_trips_through_real_sleet \
  'REAL SLEET GENERATED CONSTRAINED BINARY CODEC: PASSED'
# Task071 reuses this same pinned checkout/build; no second server build.
run_one service_route_policy task071_real_sleet_policy \
  'TASK071 REAL SLEET POLICY: PASSED'
# Task 077 is ordinarily ignored; requested execution must run exactly once.
output="$(cargo test --locked -p ams-gra-oms-runtime-rust-facade-tests --test task077_sleet -- \
  --ignored --exact task077_real_sleet_client_policy --nocapture 2>&1)"
printf '%s\n' "$output"
grep -Fqx 'TASK077 REAL SLEET CLIENT POLICY: PASSED' <<<"$output"
grep -qE '^test result: ok\. 1 passed; 0 failed; 0 ignored;' <<<"$output"
# Task 050: the REAL UCI 2.5 PositionReport, only when the caller supplies the
# pinned root (build.rs verifies its SHA-256 and fails on a mismatch).
# Task 052: the REAL UCI 2.5 SubsystemStream carrying a hexBinary value.
if [ -n "${AMS_GRA_UCI_2_5_ROOT:-}" ]; then
  run_one real_uci_position_report real::task050_real_position_report_round_trips_through_real_sleet \
    'REAL UCI POSITIONREPORT THROUGH REAL SLEET: PASSED'
  run_one real_uci_subsystem_stream real::task052_real_subsystem_stream_round_trips_through_real_sleet \
    'REAL UCI SUBSYSTEMSTREAM THROUGH REAL SLEET: PASSED'
fi
echo "real pinned Sleet ($SLEET_REV): PASSED"
