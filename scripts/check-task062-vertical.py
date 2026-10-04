#!/usr/bin/env python3
"""Production CLI + strict compiler/codec vertical for both pinned releases."""
import json
import os
from pathlib import Path
import subprocess
import sys

repo = Path(__file__).resolve().parent.parent
binary = Path(sys.argv[1]).resolve()
scratch = Path(os.environ["TMPDIR"]) / "task062-rdma-vertical"
scratch.mkdir(parents=True, exist_ok=True)


def run(args, cwd=repo):
    result = subprocess.run(list(map(str, args)), cwd=cwd, capture_output=True, text=True)
    print("COMMAND", args, "EXIT", result.returncode, flush=True)
    print(result.stdout + result.stderr, flush=True)
    if result.returncode:
        raise SystemExit(result.returncode)
    return result.stdout


for release in ["2.5", "2.6"]:
    root = os.environ[f"AMS_GRA_UCI_{release.replace('.', '_')}_ROOT"]
    contract = scratch / f"{release}.yaml"
    contract.write_text(f'''contract_version: "0.1"
service:
  name: task062
  version: "0.1.0"
  kind: service
standards:
  oms_version: "{release}"
  uci_schema_version: "{release}"
functions:
  - id: f
    name: F
    category: specific
    applicability: applicable
    exchanges:
      - id: e
        kind: oms_message
        direction: output
        mandate: mandatory
        message: RDMA_InitializeSetup
        topic: t
        timing:
          kind: asynchronous
''')
    for language in ["ada", "rust", "cpp"]:
        common = ["--schema", root, "--contract", contract, "--language", language,
                  "--world", "closed-schema"]
        result = run([binary, "service-check", *common])
        assert "READY" in result
        out = scratch / release / language
        run([binary, "service-generate", *common, "--output", out,
             *(["--with-codec"] if language == "rust" else [])])
        if language == "ada":
            run(["gnatmake", "-c", "-gnat2022", "-gnatwe", "-gnato", "service_api.ads"], out)
            for body in sorted(out.glob("*.adb")):
                run(["gnatmake", "-c", "-gnat2022", "-gnatwe", "-gnato", body.name], out)
        elif language == "cpp":
            (out / "probe.cpp").write_text('#include "service_api.hpp"\nint main(){return 0;}\n')
            run(["g++", "-std=c++17", "-Wall", "-Wextra", "-Werror", "-pedantic-errors",
                 "probe.cpp", "-o", "probe"], out)
            run([out / "probe"], out)
        else:
            body = json.loads((repo / "tests/fixtures/oms-json/task062-rdma.json").read_text())
            body["MessageHeader"]["SchemaVersion"] = f"002.{release[-1]}.0"
            (out / "valid.json").write_text(json.dumps(body))
            (out / "Cargo.toml").write_text(f'''[package]
name="task062-rdma"
version="0.1.0"
edition="2024"
[workspace]
[[bin]]
name="probe"
path="probe.rs"
[dependencies]
ams-gra-oms-runtime-api={{path={json.dumps(str(repo / "crates/runtime-api-rust"))}}}
ams-gra-oms-runtime-rust={{path={json.dumps(str(repo / "crates/runtime-rust"))}}}
serde_json="1"
''')
            (out / "probe.rs").write_text('''
#[path="service_api.rs"] pub mod generated;
use ams_gra_oms_runtime_rust::OmsJsonCodec;
use generated::{model::RDMAInitializeSetupMT,service_codec::ServiceCodec};
fn main() {
 let body:serde_json::Value=serde_json::from_str(include_str!("valid.json")).unwrap();
 let typed:RDMAInitializeSetupMT=ServiceCodec.decode_payload(&body).unwrap();
 let encoded=ServiceCodec.encode_payload(&typed).unwrap();
 assert_eq!(encoded["MessageData"]["Endpoint"],body["MessageData"]["Endpoint"]);
 assert_eq!(encoded["MessageData"]["Host"],body["MessageData"]["Host"]);
 let second:RDMAInitializeSetupMT=ServiceCodec.decode_payload(&encoded).unwrap();
 assert_eq!(ServiceCodec.encode_payload(&second).unwrap(),encoded);
 for bad in [serde_json::json!("::12345"),serde_json::json!("::256.0.2.128"),serde_json::json!("[::1]"),serde_json::json!(42)] {
  let mut changed=body.clone();changed["MessageData"]["Endpoint"][0]["IPv6_Endpoint"]["IPv6_Address"]=bad;
  let rejected:Result<RDMAInitializeSetupMT,_>=ServiceCodec.decode_payload(&changed);assert!(rejected.is_err());
 }
 println!("TASK062 REAL RDMA IPV6 CODEC: PASSED");
}
''')
            env = dict(os.environ, CARGO_TARGET_DIR=str(scratch / "cargo-target"), RUSTFLAGS="-Dwarnings")
            result = subprocess.run(["cargo", "run", "--offline", "--quiet"], cwd=out, env=env,
                                    capture_output=True, text=True)
            print(result.stdout + result.stderr, flush=True)
            if result.returncode:
                raise SystemExit(result.returncode)
            assert "TASK062 REAL RDMA IPV6 CODEC: PASSED" in result.stdout
        print(f"UCI {release} TASK062 RDMA {language} COMPILER VERTICAL: PASSED", flush=True)