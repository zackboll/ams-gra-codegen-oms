#!/usr/bin/env bash
# Task 050: fetch the pinned public UCI 2.5 release OUTSIDE this repository
# and print the path of its verified root document.
#
# Same source and pin as scripts/build-uci-pages.sh: tag v2.5 of
# https://gitlab.com/open-arsenal/uci/standard.git must resolve to commit
# 093610b7753944059360d3236770ab446d039556, and the root must have the
# established SHA-256. Anything else is a hard failure; nothing floats.
#
# Usage: scripts/fetch-pinned-uci-2.5.sh <empty-or-new-directory>
set -euo pipefail
if [[ $# -ne 1 ]]; then
  echo 'usage: scripts/fetch-pinned-uci-2.5.sh <directory>' >&2
  exit 2
fi
REV=093610b7753944059360d3236770ab446d039556
ROOT_SHA256=ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27
ROOT_PATH=03_OAC-STD-002_RevE_UCI_Schema_v2_5/UCI_MessageDefinitions_v2_5_0.xsd
dest="$(realpath -m -- "$1")"
repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
case "$dest/" in "$repo"/*) echo "refusing to fetch UCI inside the repository" >&2; exit 2;; esac
if [[ ! -d "$dest/.git" ]]; then
  git init -q "$dest"
  git -C "$dest" remote add origin https://gitlab.com/open-arsenal/uci/standard.git
  git -C "$dest" fetch -q --depth=1 origin tag v2.5
fi
revision="$(git -C "$dest" rev-parse 'refs/tags/v2.5^{commit}')"
[[ "$revision" == "$REV" ]] || { echo "v2.5 has unexpected revision $revision" >&2; exit 1; }
git -C "$dest" checkout -q --detach "$REV"
root="$dest/$ROOT_PATH"
actual="$(sha256sum "$root" | cut -d' ' -f1)"
[[ "$actual" == "$ROOT_SHA256" ]] || { echo "UCI 2.5 root SHA-256 $actual != $ROOT_SHA256" >&2; exit 1; }
printf '%s\n' "$root"
