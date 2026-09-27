#!/usr/bin/env bash
# Task 052: fetch the pinned public UCI 2.6 release OUTSIDE this repository
# and print the path of its verified root document.
#
# Same source and pin as scripts/build-uci-pages.sh: tag v2.6 of
# https://gitlab.com/open-arsenal/uci/standard.git must resolve to commit
# 78eb61b6112c8bffa40820c33124b57787fc5bd9. The 2.6 schema ships inside the
# release CDRL zip; it is extracted with the repository's safe extractor
# (scripts/validate-uci-pages.py extract) and the root must have the
# established SHA-256. Anything else is a hard failure; nothing floats.
#
# Usage: scripts/fetch-pinned-uci-2.6.sh <empty-or-new-directory>
set -euo pipefail
if [[ $# -ne 1 ]]; then
  echo 'usage: scripts/fetch-pinned-uci-2.6.sh <directory>' >&2
  exit 2
fi
REV=78eb61b6112c8bffa40820c33124b57787fc5bd9
ROOT_SHA256=af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b
ARCHIVE_PATH='UCI Release Documentation - UCI Schema/03_UCI-STD-002_Rev6_UCI_Schema_v2_6-CDRL.zip'
dest="$(realpath -m -- "$1")"
repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
case "$dest/" in "$repo"/*) echo "refusing to fetch UCI inside the repository" >&2; exit 2;; esac
if [[ ! -d "$dest/.git" ]]; then
  git init -q "$dest"
  git -C "$dest" remote add origin https://gitlab.com/open-arsenal/uci/standard.git
  git -C "$dest" fetch -q --depth=1 origin tag v2.6
fi
revision="$(git -C "$dest" rev-parse 'refs/tags/v2.6^{commit}')"
[[ "$revision" == "$REV" ]] || { echo "v2.6 has unexpected revision $revision" >&2; exit 1; }
git -C "$dest" checkout -q --detach "$REV"
extracted="$dest.extracted"
rm -rf -- "$extracted"
root="$(python3 "$repo/scripts/validate-uci-pages.py" extract "$dest/$ARCHIVE_PATH" "$extracted")"
actual="$(sha256sum "$root" | cut -d' ' -f1)"
[[ "$actual" == "$ROOT_SHA256" ]] || { echo "UCI 2.6 root SHA-256 $actual != $ROOT_SHA256" >&2; exit 1; }
printf '%s\n' "$root"
