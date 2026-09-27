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
# run_one TEST_FILE TEST_NAME PASS_LINE: exactly one matching test must run,
# pass, and print its PASSED line (a skip prints SKIPPED and fails here).
run_one() {
  output="$(cargo test -p ams-gra-oms-runtime-rust-facade-tests --test "$1" -- \
    --exact "$2" --nocapture 2>&1)"
  printf '%s\n' "$output"
  printf '%s\n' "$output" | grep -q "^$3\$"
  printf '%s\n' "$output" | grep -qE '^test result: ok\. 1 passed'
}

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
