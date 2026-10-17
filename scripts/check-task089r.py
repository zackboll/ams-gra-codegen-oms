#!/usr/bin/env python3
"""Focused production-generated Choice/repetition composition qualification."""
import argparse
import hashlib
import importlib.util
import json
import pathlib
import shutil
import subprocess
import sys
import tomllib

ROOT = pathlib.Path(__file__).resolve().parents[1]
sys.dont_write_bytecode = True
spec = importlib.util.spec_from_file_location("guard", ROOT / "scripts/check-task087a.py")
guard = importlib.util.module_from_spec(spec)
spec.loader.exec_module(guard)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--compiler", choices=["14", "16"], default="16")
    parser.add_argument("--scratch", type=pathlib.Path, required=True)
    parser.add_argument("--generator", type=pathlib.Path, required=True)
    args = parser.parse_args()
    scratch = args.scratch.resolve()
    scratch.mkdir(parents=True, exist_ok=False)
    fixtures = ROOT / "tests/fixtures/service-generate"
    schema = fixtures / "codec-ada-combined.xsd"
    consumer = scratch / "consumer"
    base = [str(args.generator.resolve()), "service-generate", "--contract",
            str(fixtures / "codec-ada-combined.yaml"), "--language", "ada",
            "--world", "closed-schema", "--with-codec"]
    guard.checked(base + ["--schema", str(schema), "--output", str(consumer)],
                  ROOT, scratch / "generate.log")
    hashes = {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in consumer.iterdir()}
    (scratch / "source-hashes.json").write_text(json.dumps(hashes, indent=2) + "\n")
    text = schema.read_text()
    for name, bad in [
        # Task 092 compiler-qualifies repeated Choice members. Keep this atomic
        # rejection control by combining that shape with a nillable member.
        ("nillable-repeated-choice-member", text.replace('name="Pick" type="uci:Selection"',
                                               'name="Pick" type="uci:Selection" maxOccurs="2" nillable="true"')),
        ("repeated-choice-alternative", text.replace('name="Nested" type="uci:Details"',
                                                    'name="Nested" type="uci:Details" maxOccurs="2"')),
        ("unsupported-nested", text.replace('type="xs:byte"', 'type="xs:double"')),
    ]:
        path = scratch / (name + ".xsd")
        path.write_text(bad)
        output = scratch / (name + "-output")
        result = subprocess.run(base + ["--schema", str(path), "--output", str(output)],
                                capture_output=True, check=False)
        (scratch / (name + ".log")).write_bytes(result.stdout + result.stderr)
        (scratch / (name + ".exit")).write_text(str(result.returncode) + "\n")
        if result.returncode != 1 or output.exists():
            raise RuntimeError(f"{name} must fail before publication")
    shutil.copy2(ROOT / "tests/ada-generated-codec/combined_consumer.adb", consumer)
    shutil.copytree(ROOT / "ada/oms-json-support", scratch / "support")
    compiler = "gnat_external" if args.compiler == "14" else "gnat_native"
    version = "14.2.0" if args.compiler == "14" else "16.1.0"
    (consumer / "alire.toml").write_text(
        'name = "task089r_consumer"\nversion = "0.1.0-dev"\n'
        'description = "Choice repetition integration"\nlicenses = "MIT"\n'
        'project-files = ["consumer.gpr"]\n[[depends-on]]\n'
        'ams_gra_oms_json = "=0.1.0-dev"\ngprbuild = "=26.0.1"\n'
        f'{compiler} = "={version}"\n[[pins]]\n'
        'ams_gra_oms_json = { path = "../support" }\n')
    (consumer / "consumer.gpr").write_text(
        'with "ams_gra_oms_json";\nproject Consumer is\n'
        'for Source_Dirs use ("."); for Object_Dir use "obj";\n'
        'for Exec_Dir use "bin"; for Main use ("combined_consumer.adb");\n'
        'package Compiler is for Default_Switches ("Ada") use '
        '("-gnat2022", "-gnato", "-O0"); end Compiler;\nend Consumer;\n')
    if guard.checked(["alr", "--version"], scratch, scratch / "alire.log").strip() != b"alr 2.1.1":
        raise RuntimeError("wrong Alire version")
    options = ["--force"] if args.compiler == "14" else []
    guard.checked(["alr", "-n", *options, "toolchain", "--select", "--local",
                   f"{compiler}={version}", "gprbuild=26.0.1"], consumer, scratch / "toolchain.log")
    for policy in ["normal", "Check", "Ignore"]:
        command = ["alr", "-n", "build", "--", "-f"]
        if policy != "normal":
            adc = scratch / (policy + ".adc")
            adc.write_text(f"pragma Assertion_Policy ({policy});\n")
            command += ["-cargs:Ada", f"-gnatec={adc}"]
        guard.checked(command, consumer, scratch / (policy + "-build.log"))
        lock = consumer / "alire/alire.lock"
        guard.validate_solution(tomllib.loads(lock.read_text()), compiler)
        shutil.copy2(lock, scratch / (policy + "-alire.lock"))
        for tool, expected in [("gnat", f"GNAT {version}"), ("gprbuild", "26.0.0")]:
            output = guard.checked(["alr", "-n", "exec", "--", tool, "--version"],
                                   consumer, scratch / (policy + "-" + tool + ".log"))
            if expected.encode() not in output:
                raise RuntimeError(f"wrong {tool}")
        output = guard.checked([str(consumer / "bin/combined_consumer")], consumer,
                               scratch / (policy + "-run.log"))
        if output != b"TASK089R_COMBINED_PASS\n":
            raise RuntimeError("combined corpus did not complete")
    print(f"TASK089R QUALIFICATION PASSED compiler={version} policies=3")


if __name__ == "__main__":
    main()