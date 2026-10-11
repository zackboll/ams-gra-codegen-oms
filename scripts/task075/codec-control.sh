#!/usr/bin/env bash
# Research differential using production CLI and public generated codec interface.
set -euo pipefail
repo=$(cd "$(dirname "$0")/../.." && pwd)
: "${TASK075_SCRATCH:?Set an absolute task-specific scratch directory}"
[[ "$TASK075_SCRATCH" = /* ]] || exit 2
mkdir -p "$TASK075_SCRATCH/private-codec/src"
scratch="$TASK075_SCRATCH/private-codec"
export CARGO_TARGET_DIR="$TASK075_SCRATCH/target" CARGO_INCREMENTAL=0
cd "$repo"
df -h "$TASK075_SCRATCH"
cargo +1.95.0 run -p ams-gra-codegen-oms -- service-generate \
  --schema "$repo/tests/fixtures/open-world/task068-private/public.xsd" \
  --extension "private=$repo/tests/fixtures/open-world/task068-private/private.xsd" \
  --contract "$repo/tests/fixtures/open-world/task068-private/private.yaml" \
  --language rust --world closed-schema --output "$scratch" --with-codec
cat > "$scratch/Cargo.toml" <<EOF
[package]
name = "task075-private-codec-control"
version = "0.0.0"
edition = "2024"
[dependencies]
ams-gra-oms-runtime-rust = { path = "$repo/crates/runtime-rust" }
serde_json = "1"
EOF
cat > "$scratch/src/main.rs" <<'EOF'
#[path = "../audit.rs"]
mod model;
#[path = "../service_codec.rs"]
mod service_codec;
use ams_gra_oms_runtime_rust::OmsJsonCodec;
use serde_json::json;
fn main() {
    let body = json!({"{urn:audit}Value":{"$type":"{urn:audit}PrivateB","{urn:audit}Flag":true,"{urn:audit}Enabled":false}});
    let value: model::Holder = service_codec::ServiceCodec.decode_payload(&body).unwrap();
    assert_eq!(service_codec::ServiceCodec.encode_payload(&value).unwrap(), body);
    for ty in ["{urn:audit}PublicA", "{urn:audit}PrivateC"] {
        let mut bad = body.clone();
        bad["{urn:audit}Value"]["$type"] = json!(ty);
        assert!(<service_codec::ServiceCodec as OmsJsonCodec<model::Holder>>::decode_payload(&service_codec::ServiceCodec, &bad).is_err());
    }
    println!("TASK075_PRIVATE_CODEC_DIFFERENTIAL_PASSED {body}");
}
EOF
cargo +1.95.0 run --manifest-path "$scratch/Cargo.toml"