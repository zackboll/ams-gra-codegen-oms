#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
: "${AMS_GRA_UCI_2_5_ROOT:?already-fetched UCI 2.5 root required}"
: "${AMS_GRA_UCI_2_6_ROOT:?already-fetched UCI 2.6 root required}"
unset TASK069_BOOTSTRAP_EVIDENCE
printf '%s  %s\n' ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27 "$AMS_GRA_UCI_2_5_ROOT" | sha256sum -c -
printf '%s  %s\n' af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b "$AMS_GRA_UCI_2_6_ROOT" | sha256sum -c -
output="$(cargo test -p ams-gra-codegen-oms --test schema_diff_pinned -- --ignored --exact task069_real_uci_semantic_diff --nocapture 2>&1)"
printf '%s\n' "$output"
grep -qE '^test result: ok\. 1 passed; 0 failed; 0 ignored;' <<<"$output"
grep -Fxq 'TASK069 REAL UCI SEMANTIC DIFF: PASSED' <<<"$output"