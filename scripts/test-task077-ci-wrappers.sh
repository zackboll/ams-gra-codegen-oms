#!/usr/bin/env bash
# Exercise failure propagation without rebuilding any workspace crate.
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
scratch="$(mktemp -d -t task077-wrappers-XXXXXX)"
trap 'rm -rf "$scratch"' EXIT
cat > "$scratch/cargo" <<'EOF'
#!/usr/bin/env bash
name=config::tests::task077_configuration
if [[ "$*" == *--list* ]]; then
  [[ "$CASE" == missing ]] || echo "$name: test"
  [[ "$CASE" != failed-list ]] || exit 7
else
  [[ "$CASE" == marker ]] || echo 'TASK077 CONFIGURATION: PASSED'
  if [[ "$CASE" == ignored ]]; then
    echo 'test result: ok. 0 passed; 0 failed; 1 ignored;'
  else
    echo 'test result: ok. 1 passed; 0 failed; 0 ignored;'
  fi
  [[ "$CASE" != failed-run ]] || exit 9
fi
EOF
chmod +x "$scratch/cargo"
for case in missing failed-list failed-run marker ignored; do
  if CASE="$case" PATH="$scratch:$PATH" bash "$root/scripts/check-task077-fast.sh" > "$scratch/$case.log" 2>&1; then
    echo "Task077 wrapper falsely passed: $case" >&2
    exit 1
  fi
done
echo 'TASK077 WRAPPER NEGATIVES: PASSED'