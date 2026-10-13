#!/usr/bin/env bash
# Build both pinned releases with the production Task 043 CLI.
set -euo pipefail
if [[ $# -ne 1 ]]; then
    echo 'usage: scripts/build-uci-pages.sh <empty-output-directory>' >&2
    exit 2
fi
repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
output="$(realpath -m -- "$1")"
if [[ -e "$output" ]]; then
    if [[ ! -d "$output" || -n "$(ls -A -- "$output")" ]]; then
        echo "output must be an empty directory: $output" >&2
        exit 2
    fi
fi
mkdir -p -- "$output"
work="$(mktemp -d)"
trap 'rm -rf -- "$work"' EXIT
source_repo="$work/standard"
git init -q "$source_repo"
git -C "$source_repo" remote add origin https://gitlab.com/open-arsenal/uci/standard.git
# Fetch only the two named tags; tags are checked against their pinned commits.
git -C "$source_repo" fetch -q --depth=1 origin tag v2.5 tag v2.6
extract_release() {
    local tag="$1" expected="$2"
    local revision
    revision="$(git -C "$source_repo" rev-parse "refs/tags/$tag^{commit}")"
    [[ "$revision" == "$expected" ]] || { echo "$tag has unexpected revision $revision" >&2; exit 1; }
    git -C "$source_repo" checkout -q --detach "$expected"
    [[ "$(git -C "$source_repo" rev-parse HEAD)" == "$expected" ]] || exit 1
}
extract_release v2.5 093610b7753944059360d3236770ab446d039556
schema25="$source_repo/03_OAC-STD-002_RevE_UCI_Schema_v2_5/UCI_MessageDefinitions_v2_5_0.xsd"
[[ -f "$schema25" ]] || { echo 'missing 2.5 schema root' >&2; exit 1; }
echo "ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27  $schema25" | sha256sum -c -
# Preserve the complete verified dependency closure before switching revisions.
git -C "$source_repo" worktree add -q --detach "$work/before" 093610b7753944059360d3236770ab446d039556
before="$work/before/${schema25#"$source_repo/"}"
# Run docs before switching the same checkout to the other release. Pass a
# checkout-relative root so the existing IR provenance does not embed a random
# temporary directory in the HTML (and remains identical on later builds).
cd "$repo"
cargo build -p ams-gra-codegen-oms --locked
cd "$source_repo"
"$repo/target/debug/ams-gra-codegen-oms" docs --schema "${schema25#"$source_repo/"}" --output "$output/2.5"
extract_release v2.6 78eb61b6112c8bffa40820c33124b57787fc5bd9
archive="$source_repo/UCI Release Documentation - UCI Schema/03_UCI-STD-002_Rev6_UCI_Schema_v2_6-CDRL.zip"
[[ -f "$archive" ]] || { echo 'missing 2.6 archive' >&2; exit 1; }
schema26="$(python3 "$repo/scripts/validate-uci-pages.py" extract "$archive" "$work/extracted")"
echo "af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b  $schema26" | sha256sum -c -
cd "$work/extracted"
"$repo/target/debug/ams-gra-codegen-oms" docs --schema "${schema26#"$work/extracted/"}" --output "$output/2.6"
SECONDS=0
"$repo/target/debug/ams-gra-codegen-oms" schema-diff --before "$before" --after "$schema26" --format tsv > "$work/comparison.tsv"
echo "semantic comparison: ${SECONDS}s"
mkdir -p "$output/compare/assets"
cp "$repo/site/compare/index.html" "$output/compare/index.html"
cp "$repo/site/compare/compare.css" "$repo/site/compare/compare.js" "$output/compare/assets/"
python3 "$repo/site/compare/package.py" "$work/comparison.tsv" "$output/2.5/assets/explorer-data.js" "$output/2.6/assets/explorer-data.js" "$output/compare/assets/compare-data.js"
cp "$repo/site/index.html" "$output/index.html"
python3 "$repo/scripts/validate-uci-pages.py" validate "$output"
