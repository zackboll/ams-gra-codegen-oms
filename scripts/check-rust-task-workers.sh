#!/usr/bin/env bash
# Independent executable lifecycle observation tests; retain all live negatives.
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
scratch="$(mktemp -d)"
trap 'rm -rf "$scratch"' EXIT
rustc +1.95.0 --edition 2024 --test -D warnings "$root/tests/rust-task-workers.rs" -o "$scratch/task-workers"
"$scratch/task-workers" --nocapture --test-threads=1