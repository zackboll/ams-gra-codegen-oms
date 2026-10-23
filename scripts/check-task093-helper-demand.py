#!/usr/bin/env python3
"""C4 production CLI/GNAT regression for semantic Binary helper demand."""
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import time
import tomllib

ROOT = Path(__file__).resolve().parents[1]
sys.dont_write_bytecode = True
spec = importlib.util.spec_from_file_location("temporal", ROOT / "scripts/check-task094.py")
temporal = importlib.util.module_from_spec(spec)
spec.loader.exec_module(temporal)
NAMES = ["ArraySample", "Array_Encode_Hex_Binary", "Encode_Hex_Binary",
         "Array_encode_hex_binary", "Array_Decode_Hex_Binary"]
QUALIFIED = "https://www.vdl.afrl.af.mil/programs/task093"
RUNTIME = "linux-x86_64-glibc-2.41"


def helper_definitions(body, identifier):
    """Match the complete Ada identifier, not a suffix in a model operation."""
    return re.findall(r"(?im)^\s*function\s+" + re.escape(identifier) + r"\s*\([^;]*?\)\s*return\s+[^;\n]+\s+is\s*$", body)


def checked(command, cwd, stem, expected=0):
    started = time.time()
    with stem.with_suffix(".log").open("xb") as output:
        result = subprocess.run(command, cwd=cwd, stdin=subprocess.DEVNULL, stdout=output, stderr=subprocess.STDOUT)
    stem.with_suffix(".command.json").write_text(json.dumps(dict(command=command, cwd=str(cwd), exit=result.returncode, expected=expected, started=started, elapsed_seconds=time.time() - started), indent=2) + "\n")
    text = stem.with_suffix(".log").read_bytes()
    if expected is not None and result.returncode != expected:
        # Preserve the complete subprocess log and expose its diagnostic in CI.
        print(text.decode("utf-8", errors="replace"), file=sys.stderr, flush=True)
        raise RuntimeError(f"exit {result.returncode}, expected {expected}: {stem}")
    return result.returncode, text


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--compiler", choices=["14", "16"], required=True)
    parser.add_argument("--family", choices=["portable", "float32"], required=True)
    parser.add_argument("--generator", type=Path, required=True)
    parser.add_argument("--scratch", type=Path, required=True)
    parser.add_argument("--profiles", nargs="+", choices=["oam", "qualified"], default=["oam", "qualified"])
    parser.add_argument("--policies", nargs="+", choices=["normal", "Check", "Ignore"], default=["normal", "Check", "Ignore"])
    parser.add_argument("--red", action="store_true", help="record intentional reviewed-head compile failures for the trigger")
    args = parser.parse_args()
    build_jobs = int(os.environ.get("OMS_CI_ADA_BUILD_JOBS", "1"))
    if not 1 <= build_jobs <= 4:
        parser.error("OMS_CI_ADA_BUILD_JOBS must be between 1 and 4")
    scratch = args.scratch.resolve()
    scratch.mkdir(parents=True, exist_ok=False)
    compiler = "gnat_external" if args.compiler == "14" else "gnat_native"
    version = "14.2.0" if args.compiler == "14" else "16.1.0"
    floating = args.family == "float32"
    if floating and (subprocess.check_output(["getconf", "GNU_LIBC_VERSION"]).strip() != b"glibc 2.41" or subprocess.check_output(["uname", "-m"]).strip() != b"x86_64"):
        raise RuntimeError("unqualified Float32 runtime")
    records = []

    def run(command, cwd, label, expected=0):
        return checked(command, cwd, scratch / label, expected)

    run(["sh", "-ec", "uname -m; getconf GNU_LIBC_VERSION; dpkg-query -W libc6; sha256sum /usr/lib/x86_64-linux-gnu/libc.so.6; alr --version"], ROOT, "runtime")
    # Fail before any long naming campaign if this environment cannot supply
    # the requested release. This task-local probe never changes global setup.
    probe = scratch / "toolchain-probe"
    probe.mkdir()
    (probe / "alire.toml").write_text(
        'name = "task093_probe"\nversion = "0.1.0-dev"\ndescription = "Compiler availability"\nlicenses = "MIT"\n[[depends-on]]\n'
        f'{compiler} = "={version}"\ngprbuild = "=26.0.1"\n')
    run(["alr", "-n", "--force", "toolchain", "--select", "--local", f"{compiler}={version}", "gprbuild=26.0.1"], probe, "toolchain-preflight")
    _, identity = run(["alr", "-n", "exec", "--", "sh", "-ec", "command -v gnat; gnat --version"], probe, "compiler-preflight")
    if f"GNAT {version}".encode() not in identity:
        raise RuntimeError("compiler preflight identity mismatch")
    probe_lock = probe / "alire/alire.lock"
    releases = [state.get("release", {}) for state in tomllib.loads(probe_lock.read_text())["solution"]["state"]]
    for name, required in [(compiler, version), ("gprbuild", "26.0.1")]:
        if not any(release.get("name") == name and release.get("version") == required for release in releases):
            raise RuntimeError(f"compiler preflight solution mismatch: {name}={required}")
    shutil.copy2(probe_lock, scratch / "toolchain-preflight.lock")
    fixtures = ROOT / "tests/fixtures/service-generate"
    suffix = "-float32" if floating else ""
    original = (fixtures / f"codec-ada-helper-demand{suffix}.xsd").read_text()
    golden = (fixtures / f"task093-helper-demand{suffix}.json").read_text()
    consumer_source = (ROOT / "tests/ada-generated-codec/helper_demand_consumer.adb").read_text()
    if floating:
        consumer_source = consumer_source.replace("with Ada.Command_Line;", "with Ada.Command_Line;\nwith Ada.Unchecked_Conversion;\nwith Interfaces; use type Interfaces.Unsigned_32;")
        consumer_source = consumer_source.replace("   File :", "   use type Interfaces.IEEE_Float_32;\n   function Bits is new Ada.Unchecked_Conversion (Interfaces.IEEE_Float_32, Interfaces.Unsigned_32);\n   File :")
        consumer_source = consumer_source.replace("Item.Number = 42", "Bits (Item.Number) = 16#3F80_0001#")
    for profile in args.profiles:
        for name in (["Array_Encode_Hex_Binary"] if args.red else NAMES):
            label = f"{profile}-{name}"
            source = original.replace("ArraySample", name)
            text = golden
            consumer = consumer_source.replace("ArraySample", name)
            if profile == "qualified":
                temporal.QUALIFIED = QUALIFIED
                source = temporal.qualified_schema(source, scratch / f"{label}-namespace.json")
                text = text.replace('"Number":', f'"{{{QUALIFIED}}}Number":').replace('"Text":', f'"{{{QUALIFIED}}}Text":')
                consumer = consumer.replace("Programs.Oam", "Programs.Task093")
            schema = scratch / f"{label}.xsd"
            schema.write_text(source)
            input_path = scratch / f"{label}.json"
            input_path.write_text(text)
            common = [str(args.generator.resolve()), "service-check", "--schema", str(schema), "--contract", str(fixtures / "codec-ada-helper-demand.yaml"), "--language", "ada", "--world", "closed-schema"]
            run(common, ROOT, f"{label}-model-check")
            plain = scratch / f"{label}-plain"
            generate = [*common]
            generate[1] = "service-generate"
            run([*generate, "--output", str(plain)], ROOT, f"{label}-model-generate")
            runtime = ["--ada-codec-runtime", RUNTIME] if floating else []
            _, report = run([*common, "--with-codec", *runtime], ROOT, f"{label}-codec-check")
            if b"codec status: READY" not in report:
                raise RuntimeError("missing readiness result")
            if floating:
                run([*common, "--with-codec"], ROOT, f"{label}-portable-reject", expected=1)
                rejected = scratch / f"{label}-rejected"
                run([*generate, "--with-codec", "--output", str(rejected)], ROOT, f"{label}-portable-generate-reject", expected=1)
                if rejected.exists():
                    raise RuntimeError("non-atomic Float32 rejection")
            directory = scratch / label
            run([*generate, "--with-codec", *runtime, "--output", str(directory)], ROOT, f"{label}-codec-generate")
            for path in plain.iterdir():
                if path.read_bytes() != (directory / path.name).read_bytes():
                    raise RuntimeError("model/API bytes changed")
            body = (directory / "service_codec.adb").read_text()
            model = "".join(p.read_text() for p in plain.glob("*.ad?"))
            if "Binary_Vectors" in model or "UnselectedBinary" in model:
                raise RuntimeError("unselected Binary leaked through projection")
            for operation in ["To_JSON_", "From_JSON_", "Encode_", "Decode_"]:
                if f"function {operation}{name} (" not in body or f"end {operation}{name};" not in body:
                    raise RuntimeError("schema-derived identifier corrupted")
            actual = [len(helper_definitions(body, h)) for h in ["Encode_Hex_Binary", "Decode_Hex_Binary"]]
            if args.red:
                if actual != [1, 1] or "Model.Binary_Vectors.Vector" not in body:
                    raise RuntimeError("intended red helper/model mismatch not reproduced")
            elif actual != [0, 0] or "Model.Binary_Vectors" in body or "with Ada.Containers;" in body:
                raise RuntimeError("unwanted Binary helper demand")
            if ("Require_Platform" in body) != floating or ("with AMS_GRA_OMS_JSON_Occurrences;" in body) != floating:
                raise RuntimeError("runtime representation leakage")
            if not floating:
                opted = scratch / f"{label}-opted"
                run([*generate, "--with-codec", "--ada-codec-runtime", RUNTIME, "--output", str(opted)], ROOT, f"{label}-opted-generate")
                if any(p.read_bytes() != (opted / p.name).read_bytes() for p in directory.iterdir()):
                    raise RuntimeError("runtime option changes portable output")
            (directory / "helper_demand_consumer.adb").write_text(consumer)
            # Each independent naming case owns its support/provider build area.
            support = scratch / f"{label}-support"
            shutil.copytree(ROOT / "ada/oms-json-support", support)
            (directory / "alire.toml").write_text(
                'name = "task093_c4"\nversion = "0.1.0-dev"\ndescription = "Semantic helper demand"\nlicenses = "MIT"\nproject-files = ["consumer.gpr"]\n[[depends-on]]\n'
                f'{compiler} = "={version}"\ngprbuild = "=26.0.1"\nams_gra_oms_json = "=0.1.0-dev"\n[[pins]]\nams_gra_oms_json = {{ path = "../{support.name}" }}\n')
            project = "ams_gra_oms_json_float32" if floating else "ams_gra_oms_json"
            (directory / "consumer.gpr").write_text(
                f'with "../{support.name}/{project}";\nproject Consumer is\nfor Source_Dirs use ("."); for Object_Dir use "obj"; for Exec_Dir use "bin";\nfor Main use ("helper_demand_consumer.adb");\npackage Compiler is for Default_Switches ("Ada") use ("-gnat2022", "-gnato", "-O0"); end Compiler;\nend Consumer;\n')
            run(["alr", "-n", "settings", "--set", "dependencies.shared", "false"], directory, f"{label}-isolation")
            run(["alr", "-n", "--force", "toolchain", "--select", "--local", f"{compiler}={version}", "gprbuild=26.0.1"], directory, f"{label}-toolchain")
            for policy in args.policies:
                cell = f"{label}-{policy}"
                command = ["alr", "-n", "build", "--", "-f", f"-j{build_jobs}"]
                if policy != "normal":
                    adc = scratch / f"{cell}.adc"
                    adc.write_text(f"pragma Assertion_Policy ({policy});\n")
                    command += ["-cargs:Ada", f"-gnatec={adc}"]
                exit_code, diagnostic = run(command, directory, f"{cell}-build", expected=None if args.red else 0)
                lock = directory / "alire/alire.lock"
                temporal.guard.validate_solution(tomllib.loads(lock.read_text()), compiler)
                shutil.copy2(lock, scratch / f"{cell}-alire.lock")
                _, identity = run(["alr", "-n", "exec", "--", "sh", "-ec", "command -v gnat; gnat --version"], directory, f"{cell}-gnat")
                if f"GNAT {version}".encode() not in identity:
                    raise RuntimeError("compiler identity mismatch")
                if args.red:
                    if exit_code == 0 or b'"Binary_Vectors" not declared in "Oam"' not in diagnostic:
                        raise RuntimeError("expected actual Binary_Vectors compiler diagnostic missing")
                else:
                    _, output = run([str(directory / "bin/helper_demand_consumer"), str(input_path)], directory, f"{cell}-execute")
                    if output != b"TASK093_C4_HELPER_DEMAND_COMPLETE checks= 7\n":
                        raise RuntimeError("incomplete naming execution")
                records.append(dict(compiler=version, family=args.family, profile=profile, name=name, policy=policy, intentional_red=args.red, compiler_exit=exit_code))
                print("COMPLETE", cell, flush=True)
    paths = [p for p in scratch.rglob("*") if p.is_file() and p.suffix in [".ads", ".adb", ".c", ".gpr", ".lock", ".xsd", ".json"]]
    paths += [args.generator.resolve(), Path(__file__).resolve(), ROOT / "tests/ada-generated-codec/helper_demand_consumer.adb", fixtures / "codec-ada-helper-demand.yaml", fixtures / f"codec-ada-helper-demand{suffix}.xsd", fixtures / f"task093-helper-demand{suffix}.json"]
    (scratch / "sources.sha256").write_text("".join(f"{hashlib.sha256(p.read_bytes()).hexdigest()}  {p}\n" for p in sorted(paths)))
    temporary = scratch / "result.json.tmp"
    temporary.write_text(json.dumps(dict(completed=True, red=args.red, records=records), indent=2) + "\n")
    temporary.replace(scratch / "result.json")
    print(f"TASK093_C4_HELPER_QUALIFICATION_COMPLETE family={args.family} compiler={version} naming_executions={len(records)} red={args.red}")


if __name__ == "__main__":
    main()