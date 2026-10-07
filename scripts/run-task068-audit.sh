#!/usr/bin/env bash
# Evidence-only probes. No production source is modified.
set -euo pipefail
repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
scratch=/tmp/task068
export CARGO_TARGET_DIR="$scratch/target" TMPDIR="$scratch/tmp"
mkdir -p "$scratch"/{logs,evidence,probes,manifests,fixtures,tmp,target}
"$repo/scripts/fetch-pinned-uci-2.5.sh" "$scratch/uci25" > "$scratch/manifests/uci25-root.txt"
"$repo/scripts/fetch-pinned-uci-2.6.sh" "$scratch/uci26" > "$scratch/manifests/uci26-root.txt"
probe="$scratch/probes/reproduce"
mkdir -p "$probe/src/bin"
python3 - "$repo" "$probe" <<'PY'
import pathlib, sys
repo, probe = map(pathlib.Path, sys.argv[1:])
manifest = '[package]\nname="task068-reproduce"\nversion="0.1.0"\nedition="2024"\n[dependencies]\n'
for name in ['ir', 'xsd-frontend', 'codegen-core', 'service-contract', 'backend-ada', 'backend-rust', 'backend-cpp']:
    manifest += f'ams-gra-oms-{name}={{path="{repo / "crates" / name}"}}\n'
(probe / 'Cargo.toml').write_text(manifest)
for source in (repo / 'scripts/task068').glob('*.rs'):
    if source.name.endswith('-probe.rs'):
        continue
    (probe / 'src/bin' / source.name).write_text(source.read_text())
PY
for bin in ${TASK068_PROBES:-inventory paths readiness closed-readiness synthetic}; do
  cargo +1.95.0 run --release --manifest-path "$probe/Cargo.toml" --bin "$bin" \
    > "$scratch/evidence/reproduced-$bin.txt" 2> "$scratch/logs/reproduced-$bin.log"
done

# Production CLI overlay/codec experiment and compiler controls.
fixture="$repo/tests/fixtures/open-world/task068-private"
cargo +1.95.0 build -p ams-gra-codegen-oms --manifest-path "$repo/Cargo.toml"
for shape in private concrete; do
  schema="$fixture/public.xsd"
  [[ "$shape" != concrete ]] || schema="$fixture/concrete.xsd"
  output="$scratch/probes/reproduced-$shape-codec"
  "$CARGO_TARGET_DIR/debug/ams-gra-codegen-oms" service-generate \
    --schema "$schema" --contract "$fixture/private.yaml" \
    --extension "private=$fixture/private.xsd" --language rust \
    --world closed-schema --with-codec --output "$output"
  mkdir -p "$output/src"
  cp "$repo/scripts/task068/$shape-codec-probe.rs" "$output/src/main.rs"
  python3 - "$repo" "$output" "$shape" <<'PY'
import pathlib, sys
repo, output = map(pathlib.Path, sys.argv[1:3])
shape = sys.argv[3]
(output / 'Cargo.toml').write_text(
    f'[package]\nname="task068-{shape}-codec"\nversion="0.1.0"\nedition="2024"\n'
    f'[dependencies]\nserde_json="1"\nams-gra-oms-runtime-rust={{path="{repo / "crates/runtime-rust"}"}}\n')
PY
  cargo +1.95.0 run --manifest-path "$output/Cargo.toml" \
    > "$scratch/evidence/reproduced-$shape-codec.txt" \
    2> "$scratch/logs/reproduced-$shape-codec.log"
done
model="$scratch/fixtures/synthetic/private"
rustc +1.95.0 --edition 2024 --crate-type lib "$model/model.rs" -o "$scratch/probes/private-model.rlib"
printf '#include "model.hpp"\nint main() {}\n' > "$model/probe.cpp"
g++ -std=c++17 -Wall -Wextra -Werror -fsyntax-only "$model/probe.cpp"
printf 'package Urn is\nend Urn;\n' > "$model/urn.ads"
(cd "$model" && gnatmake -c -gnatwa urn-audit.ads)
