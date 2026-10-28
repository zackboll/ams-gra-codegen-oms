#!/usr/bin/env bash
# actionlint 1.7.12 predates GitHub's documented queue:max key. Suppress ONLY
# that unknown-key diagnostic. ci-policy.py strictly validates every queue.
set -euo pipefail
repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo"
python3 scripts/ci-policy.py
"${ACTIONLINT:-actionlint}" -shellcheck= -pyflakes= \
  -ignore 'unexpected key "queue" for "concurrency" section\. expected one of "cancel-in-progress", "group"' \
  .github/workflows/*.yml