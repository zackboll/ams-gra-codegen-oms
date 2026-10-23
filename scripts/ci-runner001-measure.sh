#!/usr/bin/env bash
# GitHub custom shell: keep bash -eo pipefail semantics and the workload exit.
# GNU time measures the step's process tree, not total machine/VM memory.
set -euo pipefail

evidence=/tmp/task095-evidence/ci-runner001
mkdir -p "$evidence"
record="$(mktemp -d "$evidence/step-XXXXXX")"
cp "$1" "$record/command.sh"
cat /proc/meminfo > "$record/memory-before.txt"
date -u +%Y-%m-%dT%H:%M:%SZ > "$record/started.txt"
status=0
time_binary="$(type -P time)"
"$time_binary" -o "$record/resources.txt" \
  -f 'user_cpu_seconds=%U\nsystem_cpu_seconds=%S\nwall_seconds=%e\nmax_process_rss_kib=%M\nexit_status=%x' \
  bash --noprofile --norc -eo pipefail "$1" || status=$?
cat /proc/meminfo > "$record/memory-after.txt"
date -u +%Y-%m-%dT%H:%M:%SZ > "$record/completed.txt"
cat "$record/resources.txt"
exit "$status"