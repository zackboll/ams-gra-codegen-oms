#!/usr/bin/env python3
"""Actual smallest newly READY service; production CLI and strict compilers."""
import json
import os
from pathlib import Path
import subprocess
import sys

repo = Path(__file__).resolve().parent.parent
binary = Path(sys.argv[1]).resolve()
scratch = Path(os.environ["TMPDIR"]) / "task066-vertical"
scratch.mkdir(parents=True, exist_ok=True)

def run(args, cwd=repo):
    result = subprocess.run(list(map(str, args)), cwd=cwd, capture_output=True, text=True)
    print("COMMAND", list(map(str, args)), "EXIT", result.returncode, flush=True)
    print(result.stdout + result.stderr, flush=True)
    if result.returncode:
        raise SystemExit(result.returncode)
    return result.stdout

for release in ("2.5", "2.6"):
    contract = scratch / f"{release}.yaml"
    contract.write_text(f'''contract_version: "0.1"
service:
  name: task066
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
        message: AO_CapabilityStatus
        topic: t
        timing:
          kind: asynchronous
''')
    root = os.environ[f"AMS_GRA_UCI_{release.replace('.', '_')}_ROOT"]
    for language in ("ada", "rust", "cpp"):
        common = ["--schema", root, "--contract", contract, "--language", language, "--world", "closed-schema"]
        assert "status: READY" in run([binary, "service-check", *common])
        out = scratch / release / language
        run([binary, "service-generate", *common, "--output", out, *(["--with-codec"] if language == "rust" else [])])
        sources = list(out.glob("*.rs")) + list(out.glob("*.ads")) + list(out.glob("*.adb")) + list(out.glob("*.hpp"))
        print("SIZE", release, language, sum(p.stat().st_size for p in sources), sum(len(p.read_text().splitlines()) for p in sources), flush=True)
        if language == "ada":
            run(["gnatmake", "-c", "-gnat2022", "-gnatwe", "-gnato", "service_api.ads"], out)
            for body in sorted(out.glob("*.adb")):
                run(["gnatmake", "-c", "-gnat2022", "-gnatwe", "-gnato", body.name], out)
        elif language == "cpp":
            (out / "probe.cpp").write_text('#include "service_api.hpp"\nint main(){return 0;}\n')
            run(["g++", "-std=c++17", "-Wall", "-Wextra", "-Werror", "-pedantic-errors", "probe.cpp", "-o", "probe"], out)
            run([out / "probe"], out)
        else:
            # Support-only profiles are reached through NITF_PackingPlanPET.
            # Round-trip the actual selected message with that concrete sum
            # alternative, not a standalone support type codec.
            message = json.loads((repo / "tests/fixtures/oms-json/task062-rdma.json").read_text())
            message["MessageHeader"]["SchemaVersion"] = f"002.{release[-1]}.0"
            message["MessageData"] = {
                "CapabilityStatus": [{"CapabilityID": {"UUID": "550e8400-e29b-41d4-a716-446655440000"}, "Availability": "AVAILABLE"}],
                "DefaultPackingPlan": {"$type": "NITF_PackingPlanType", "FileHeader": {"FileSecuritySourceDate": "2١000101"}, "ACFTB": {"AC_TO": "2١0001010000"}},
            }
            (out / "valid.json").write_text(json.dumps(message))
            (out / "Cargo.toml").write_text(f'''[package]
name="task066-real"
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
use generated::{model as m,service_codec::ServiceCodec};
fn main() {
 let json:serde_json::Value=serde_json::from_str(include_str!("valid.json")).unwrap();
 let value:m::AOCapabilityStatusMT=ServiceCodec.decode_payload(&json).unwrap();
 let encoded=ServiceCodec.encode_payload(&value).unwrap();
 assert_eq!(encoded["MessageData"]["DefaultPackingPlan"]["FileHeader"]["FileSecuritySourceDate"],serde_json::json!("2١000101"));
 assert_eq!(encoded["MessageData"]["DefaultPackingPlan"]["ACFTB"]["AC_TO"],serde_json::json!("2١0001010000"));
 let restored:m::AOCapabilityStatusMT=ServiceCodec.decode_payload(&encoded).unwrap();
 assert_eq!(ServiceCodec.encode_payload(&restored).unwrap(),encoded);
 for bad in [serde_json::json!("2A000101"),serde_json::json!("١0000101"),serde_json::json!(42)] {
  let mut changed=json.clone();changed["MessageData"]["DefaultPackingPlan"]["FileHeader"]["FileSecuritySourceDate"]=bad;
  let result:Result<m::AOCapabilityStatusMT,_>=ServiceCodec.decode_payload(&changed);assert!(result.is_err());
 }
 println!("TASK066 REAL UNICODE JSON ROUNDTRIP: PASSED");
}
''')
            env = dict(os.environ, CARGO_TARGET_DIR=str(Path(os.environ["CARGO_TARGET_DIR"]) / "task066-vertical"), RUSTFLAGS="-Dwarnings", CARGO_PROFILE_DEV_DEBUG="0", CARGO_BUILD_JOBS="2")
            result = subprocess.run(["cargo", "run", "--offline", "--quiet"], cwd=out, env=env, capture_output=True, text=True)
            print(result.stdout + result.stderr, flush=True)
            if result.returncode:
                raise SystemExit(result.returncode)
            assert "TASK066 REAL UNICODE JSON ROUNDTRIP: PASSED" in result.stdout
        print(f"UCI {release} TASK066 AO_CAPABILITY {language} VERTICAL: PASSED", flush=True)