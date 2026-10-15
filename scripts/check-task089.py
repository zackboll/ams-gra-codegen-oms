#!/usr/bin/env python3
"""Mandatory production-generated repeated Ada codec execution; no skip path."""
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


def checked(command, cwd, log):
    result = subprocess.run(command, cwd=cwd, stdout=subprocess.PIPE,
                            stderr=subprocess.STDOUT, check=False)
    log.write_bytes(result.stdout)
    log.with_suffix(".exit").write_text(str(result.returncode) + "\n")
    if result.returncode:
        raise RuntimeError(f"exit={result.returncode}: {command}; see {log}")
    return result.stdout


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--compiler", choices=["14", "16"], default="16")
    parser.add_argument("--scratch", type=pathlib.Path, required=True)
    parser.add_argument("--generator", type=pathlib.Path, required=True)
    args = parser.parse_args()
    scratch = args.scratch.resolve()
    scratch.mkdir(parents=True, exist_ok=False)
    consumer = scratch / "consumer"
    fixtures = ROOT / "tests/fixtures/service-generate"
    base = [str(args.generator.resolve()), "service-generate", "--schema",
            str(fixtures / "codec-ada-repeated.xsd"), "--contract",
            str(fixtures / "codec-ada-repeated.yaml"), "--language", "ada",
            "--world", "closed-schema"]
    checked(base + ["--with-codec", "--output", str(consumer)], ROOT, scratch / "generate.log")
    plain = scratch / "plain"
    checked(base + ["--output", str(plain)], ROOT, scratch / "plain-generate.log")
    for path in plain.iterdir():
        if path.read_bytes() != (consumer / path.name).read_bytes():
            raise RuntimeError(f"codec changed model/API: {path.name}")
    codec_spec = (consumer / "service_codec.ads").read_text()
    for operation in ["Encode_RepeatedPayload", "Decode_RepeatedPayload"]:
        if codec_spec.count("function " + operation) != 1:
            raise RuntimeError("payload codec not deduplicated")
    hashes = {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in consumer.iterdir()}
    (scratch / "generated-source-hashes.json").write_text(json.dumps(hashes, indent=2) + "\n")
    for name in ["repeated_codec_consumer.adb", "repeated_model_consumer.adb"]:
        shutil.copy2(ROOT / "tests/ada-generated-codec" / name, consumer)
    shutil.copytree(ROOT / "ada/oms-json-support", scratch / "support")
    compiler = "gnat_external" if args.compiler == "14" else "gnat_native"
    version = "14.2.0" if args.compiler == "14" else "16.1.0"
    (consumer / "alire.toml").write_text(
        'name = "task089_consumer"\nversion = "0.1.0-dev"\n'
        'description = "Repeated codec qualification"\nlicenses = "MIT"\n'
        'project-files = ["consumer.gpr"]\n[[depends-on]]\n'
        'ams_gra_oms_json = "=0.1.0-dev"\ngprbuild = "=26.0.1"\n'
        f'{compiler} = "={version}"\n[[pins]]\n'
        'ams_gra_oms_json = { path = "../support" }\n')
    (consumer / "consumer.gpr").write_text(
        'with "ams_gra_oms_json";\nproject Consumer is\n'
        'for Source_Dirs use ("."); for Object_Dir use "obj";\n'
        'for Exec_Dir use "bin"; for Main use '
        '("repeated_codec_consumer.adb", "repeated_model_consumer.adb");\n'
        'package Compiler is for Default_Switches ("Ada") use '
        '("-gnat2022", "-gnato", "-O0"); end Compiler;\nend Consumer;\n')
    if checked(["alr", "--version"], scratch, scratch / "alire.log").strip() != b"alr 2.1.1":
        raise RuntimeError("Alire version differs")
    options = ["--force"] if args.compiler == "14" else []
    checked(["alr", "-n", *options, "toolchain", "--select", "--local",
             f"{compiler}={version}", "gprbuild=26.0.1"], consumer, scratch / "toolchain.log")
    for policy in ["normal", "Check", "Ignore"]:
        command = ["alr", "-n", "build", "--", "-f"]
        if policy != "normal":
            adc = scratch / f"{policy}.adc"
            adc.write_text(f"pragma Assertion_Policy ({policy});\n")
            command += ["-cargs:Ada", f"-gnatec={adc}"]
        checked(command, consumer, scratch / f"{policy}-build.log")
        lock = consumer / "alire/alire.lock"
        guard.validate_solution(tomllib.loads(lock.read_text()), compiler)
        shutil.copy2(lock, scratch / f"{policy}-alire.lock")
        for tool, expected in [("gnat", f"GNAT {version}"), ("gprbuild", "26.0.0")]:
            output = checked(["alr", "-n", "exec", "--", tool, "--version"],
                             consumer, scratch / f"{policy}-{tool}.log")
            if expected.encode() not in output:
                raise RuntimeError(f"wrong {tool} version")
        output = checked([str(consumer / "bin/repeated_model_consumer")], consumer,
                         scratch / f"{policy}-model-run.log")
        if output != b"TASK089_MODEL_BASELINE_PASS\n":
            raise RuntimeError("model corpus did not execute")
        output = checked([str(consumer / "bin/repeated_codec_consumer"),
                          str(fixtures / "codec-ada-repeated.json"),
                          str(fixtures / "codec-ada-repeated-zero.json")], consumer,
                         scratch / f"{policy}-codec-run.log")
        if output != b"TASK089_CODEC_COMPLETE cases= 121\n":
            raise RuntimeError(f"codec corpus did not complete: {output!r}")
    print(f"TASK089 QUALIFICATION PASSED compiler={version} policies=3")


if __name__ == "__main__":
    main()