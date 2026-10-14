#!/usr/bin/env python3
"""Mandatory support-only qualification; no missing-tool or test skip path."""
import argparse
import hashlib
import pathlib
import re
import shutil
import subprocess
import tomllib

CORPUS_HASH = "ba61ad1bf1cd12253655cb6495104874f5a61bf6f7f4b06fe8a84f154cfcf64e"
OUTPUT_HASH = "67a60d687e5fa1ad660a9b0322a4f9588f0107925e6abd4fe15353f8777c3036"
CORPUS_MARKER = b"GUARDED CORPUS PASSED cases= 199\n"
BOUNDARY_MARKER = b"TASK087 SUPPORT BOUNDARIES: PASSED\n"
PROVIDER_HASH = "sha512:cc5dd0a199115fee269ec1ef963b80abbb56e3029e817d5dbfe71e90fff09d0535ae5be86dedb6d3fbcc74cfa2f3e2e7885de800e5805f9f596507814dce6d4b"


def checked(command, cwd, log):
    result = subprocess.run(command, cwd=cwd, stdout=subprocess.PIPE,
                            stderr=subprocess.STDOUT, check=False)
    log.write_bytes(result.stdout)
    print(f"exit={result.returncode} log={log}", flush=True)
    if result.returncode:
        raise RuntimeError(f"command failed: {command}; see {log}")
    return result.stdout


def validate_output(output):
    if re.search(rb"skip|unexpected|raised |EXCEPTION", output, re.IGNORECASE):
        raise RuntimeError("skipped qualification or unexpected exception")
    if output.count(CORPUS_MARKER) != 1 or output.count(BOUNDARY_MARKER) != 1:
        raise RuntimeError("missing/incorrect corpus count or completion marker")
    prefix = output.split(CORPUS_MARKER)[0] + CORPUS_MARKER
    if hashlib.sha256(prefix).hexdigest() != CORPUS_HASH:
        raise RuntimeError("frozen 199-case evidence differs")
    if hashlib.sha256(output).hexdigest() != OUTPUT_HASH:
        raise RuntimeError("support-boundary evidence differs")


def run_probe(command, cwd, log):
    output = checked(command, cwd, log)
    validate_output(output)
    print(f"TASK087A PROBE: PASSED cases=199 boundaries=4 sha256={OUTPUT_HASH}")


def validate_solution(data, compiler):
    required = {"gnatcoll": "26.0.0", "gnatcoll_minimal": "26.0.0",
                "gprbuild": "26.0.1", compiler: "14.2.0" if compiler == "gnat_external" else "16.1.0"}
    releases = [state.get("release", {}) for state in data["solution"]["state"]]
    for name, version in required.items():
        matches = [r for r in releases if r.get("name") == name]
        if not matches or any(r.get("version") != version for r in matches):
            raise RuntimeError(f"wrong/missing dependency: {name}={version}")
        if name.startswith("gnatcoll"):
            if any(PROVIDER_HASH not in r.get("origin", {}).get("hashes", []) for r in matches):
                raise RuntimeError("provider archive hash differs")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--compiler", choices=["14", "16"], default="16")
    parser.add_argument("--scratch", type=pathlib.Path, required=True,
                        help="new task-owned directory (must not exist)")
    args = parser.parse_args()
    root = pathlib.Path(__file__).resolve().parents[1]
    scratch = args.scratch.resolve()
    scratch.mkdir(parents=True, exist_ok=False)
    version = checked(["alr", "--version"], scratch, scratch / "alire-version.log")
    if version.strip() != b"alr 2.1.1":
        raise RuntimeError("Alire 2.1.1 required")
    support = scratch / "ada" / "oms-json-support"
    shutil.copytree(root / "ada" / "oms-json-support", support)
    probe = scratch / "tests" / "ada-json-probe"
    shutil.copytree(root / "tests" / "ada-json-probe", probe)
    compiler = "gnat_external" if args.compiler == "14" else "gnat_native"
    compiler_version = "14.2.0" if args.compiler == "14" else "16.1.0"
    manifest = probe / "alire.toml"
    text = manifest.read_text().replace('gnat_external = "=14.2.0"',
                                       f'{compiler} = "={compiler_version}"\ngprbuild = "=26.0.1"')
    manifest.write_text(text)
    # Alire's external-compiler compatibility heuristic rejects this locally
    # qualified pair. Force selection only, never suppress build/run failures.
    options = ["--force"] if args.compiler == "14" else []
    checked(["alr", "-n", *options, "toolchain", "--select", "--local",
             f"{compiler}={compiler_version}", "gprbuild=26.0.1"], probe,
            scratch / "toolchain.log")
    checked(["alr", "-n", "build", "--", "-f"], probe, scratch / "initial-build.log")
    solution = tomllib.loads((probe / "alire" / "alire.lock").read_text())
    validate_solution(solution, compiler)
    shutil.copy2(probe / "alire" / "alire.lock", scratch / "qualified-alire.lock")
    for tool, expected in [("gnat", f"GNAT {compiler_version}"), ("gprbuild", "26.0.0")]:
        output = checked(["alr", "-n", "exec", "--", tool, "--version"],
                         probe, scratch / f"{tool}-version.log").decode()
        if expected not in output:
            raise RuntimeError(f"wrong selected {tool} version")
    # Independent project consumes the support package, not copied Ada sources.
    consumer = scratch / "consumer"
    consumer.mkdir()
    (consumer / "alire.toml").write_text(
        'name = "task087a_consumer"\nversion = "0.1.0-dev"\n'
        'description = "Clean support consumer"\nlicenses = "MIT"\n'
        'project-files = ["consumer.gpr"]\n[[depends-on]]\n'
        'ams_gra_oms_json = "=0.1.0-dev"\ngprbuild = "=26.0.1"\n'
        f'{compiler} = "={compiler_version}"\n[[pins]]\n'
        'ams_gra_oms_json = { path = "../ada/oms-json-support" }\n')
    (consumer / "consumer.gpr").write_text(
        'with "ams_gra_oms_json";\nproject Consumer is\n'
        'for Source_Dirs use ("."); for Object_Dir use "obj";\n'
        'for Exec_Dir use "bin"; for Main use ("consumer.adb");\nend Consumer;\n')
    (consumer / "consumer.adb").write_text(
        'with AMS_GRA_OMS_JSON_Guard; with Ada.Text_IO;\n'
        'procedure Consumer is\nbegin\n'
        'if AMS_GRA_OMS_JSON_Guard.Write (AMS_GRA_OMS_JSON_Guard.Read ("{}")) /= "{}" then\n'
        'raise Program_Error; end if;\n'
        'Ada.Text_IO.Put_Line ("TASK087A CLEAN CONSUMER: PASSED");\nend Consumer;\n')
    checked(["alr", "-n", "build"], consumer, scratch / "consumer-build.log")
    output = checked([str(consumer / "bin" / "consumer")], consumer,
                     scratch / "consumer-run.log")
    if output != b"TASK087A CLEAN CONSUMER: PASSED\n":
        raise RuntimeError("clean consumer did not execute")
    for policy in ["normal", "check", "ignore"]:
        command = ["alr", "-n", "build", "--", "-f"]
        if policy != "normal":
            adc = scratch / f"{policy}.adc"
            adc.write_text(f"pragma Assertion_Policy ({policy.title()});\n")
            command += ["-cargs:Ada", f"-gnatec={adc}"]
        checked(command, probe, scratch / f"{policy}-build.log")
        run_probe([str(probe / "bin" / "json_probe")], probe, scratch / f"{policy}-run.log")
    print(f"TASK087A QUALIFICATION: PASSED compiler={compiler_version} policies=3")


if __name__ == "__main__":
    main()