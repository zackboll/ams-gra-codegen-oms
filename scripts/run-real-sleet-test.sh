#!/usr/bin/env bash
# Task 049: run the generated-facade -> runtime-rust -> sleet-client test
# against the UNMODIFIED pinned Sleet server.
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
output="$(cargo test -p ams-gra-oms-runtime-rust-facade-tests --test real_sleet -- \
  --exact task049_generated_facade_round_trips_through_real_sleet --nocapture 2>&1)"
printf '%s\n' "$output"
printf '%s\n' "$output" | grep -q '^REAL SLEET: PASSED$'
printf '%s\n' "$output" | grep -qE '^test result: ok\. 1 passed'
echo "real pinned Sleet ($SLEET_REV): PASSED"
