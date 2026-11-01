#!/usr/bin/env bash
# actionlint 1.7.12 predates GitHub's documented queue:max key. Suppress ONLY
# that unknown-key diagnostic; keep all other syntax diagnostics fatal.
set -euo pipefail
repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo"
"${ACTIONLINT:-actionlint}" -shellcheck= -pyflakes= \
  -ignore 'unexpected key "queue" for "concurrency" section\. expected one of "cancel-in-progress", "group"' \
  .github/workflows/*.yml