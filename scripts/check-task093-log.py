#!/usr/bin/env python3
"""Compile and execute the full unchanged pinned UCI Log service, not an extract."""
import argparse
import hashlib
import importlib.util
import json
import pathlib
import shutil
import sys
import tomllib

ROOT = pathlib.Path(__file__).resolve().parents[1]
sys.dont_write_bytecode = True
spec = importlib.util.spec_from_file_location("binary", ROOT / "scripts/check-task093-real.py")
binary = importlib.util.module_from_spec(spec)
spec.loader.exec_module(binary)
checked = binary.checked


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--compiler", choices=["14", "16"], required=True)
    parser.add_argument("--version", choices=list(binary.PINS), required=True)
    parser.add_argument("--schema", type=pathlib.Path, required=True)
    parser.add_argument("--scratch", type=pathlib.Path, required=True)
    parser.add_argument("--generator", type=pathlib.Path, required=True)
    args = parser.parse_args()
    scratch = args.scratch.resolve()
    scratch.mkdir(parents=True, exist_ok=False)
    if hashlib.sha256(args.schema.read_bytes()).hexdigest() != binary.PINS[args.version]:
        raise RuntimeError("real UCI root hash differs")
    fixtures = ROOT / "tests/fixtures/service-generate"
    common = [str(args.generator.resolve()), "service-generate", "--schema", str(args.schema.resolve()),
              "--contract", str(fixtures / "real-duration-log.yaml"),
              "--language", "ada", "--world", "closed-schema"]
    model, consumer = scratch / "model", scratch / "consumer"
    checked([*common, "--output", str(model)], ROOT, scratch / "model-generate.log")
    checked([*common, "--with-codec", "--output", str(consumer)], ROOT, scratch / "codec-generate.log")
    parity = {}
    for path in model.iterdir():
        if path.read_bytes() != (consumer / path.name).read_bytes():
            raise RuntimeError(f"real Log model/API changed: {path.name}")
        parity[path.name] = hashlib.sha256(path.read_bytes()).hexdigest()
    (scratch / "model-byte-parity.json").write_text(json.dumps(parity, indent=2) + "\n")
    source = ROOT / "tests/ada-generated-codec/hexbinary_log_consumer.adb"
    text = source.read_text()
    if args.version == "2.6":
        text = text.replace("(Item.MessageData.Data.Value);", "(Item.MessageData.Data.Value.Value);")
    (consumer / source.name).write_text(text)
    shutil.copytree(ROOT / "ada/oms-json-support", scratch / "support")
    compiler = "gnat_external" if args.compiler == "14" else "gnat_native"
    version = "14.2.0" if args.compiler == "14" else "16.1.0"
    (consumer / "alire.toml").write_text(
        'name = "task093_log"\nversion = "0.1.0-dev"\n'
        'description = "Full real Log qualification"\nlicenses = "MIT"\n'
        'project-files = ["consumer.gpr"]\n[[depends-on]]\n'
        'ams_gra_oms_json = "=0.1.0-dev"\ngprbuild = "=26.0.1"\n'
        f'{compiler} = "={version}"\n[[pins]]\n'
        'ams_gra_oms_json = { path = "../support" }\n')
    (consumer / "consumer.gpr").write_text(
        'with "ams_gra_oms_json";\nproject Consumer is\n'
        'for Source_Dirs use ("."); for Object_Dir use "obj"; for Exec_Dir use "bin";\n'
        'for Main use ("hexbinary_log_consumer.adb");\n'
        'package Compiler is for Default_Switches ("Ada") use '
        '("-gnat2022", "-gnato", "-O0"); end Compiler;\nend Consumer;\n')
    if checked(["alr", "--version"], scratch, scratch / "alire.log").strip() != b"alr 2.1.1":
        raise RuntimeError("Alire identity differs")
    checked(["alr", "-n", "settings", "--set", "dependencies.shared", "false"], consumer,
            scratch / "isolation.log")
    checked(["alr", "-n", *(["--force"] if args.compiler == "14" else []),
             "toolchain", "--select", "--local", f"{compiler}={version}", "gprbuild=26.0.1"],
            consumer, scratch / "toolchain.log")
    golden = fixtures / f"task093-real-log{args.version.replace('.', '')}.json"
    for policy in ["normal", "Check", "Ignore"]:
        command = ["alr", "-n", "build", "--", "-f"]
        if policy != "normal":
            adc = scratch / f"{policy}.adc"
            adc.write_text(f"pragma Assertion_Policy ({policy});\n")
            command += ["-cargs:Ada", f"-gnatec={adc}"]
        checked(command, consumer, scratch / f"{policy}-build.log")
        lock = consumer / "alire/alire.lock"
        binary.binary.duration.guard.validate_solution(tomllib.loads(lock.read_text()), compiler)
        shutil.copy2(lock, scratch / f"{policy}-alire.lock")
        for tool, identity in [("gnat", f"GNAT {version}"), ("gprbuild", "26.0.0")]:
            if identity.encode() not in checked(["alr", "-n", "exec", "--", tool, "--version"],
                                                consumer, scratch / f"{policy}-{tool}.log"):
                raise RuntimeError(f"{tool} identity differs")
        output = checked([str(consumer / "bin/hexbinary_log_consumer"), str(golden)], consumer,
                         scratch / f"{policy}-run.log")
        if output != b"TASK093_LOG_COMPLETE checks= 29 rejections= 9\n":
            raise RuntimeError(f"real Log completion differs: {output!r}")
    paths = list(consumer.glob("*.ad?")) + [source, golden, args.schema.resolve(),
            args.generator.resolve(), pathlib.Path(__file__).resolve(), fixtures / "real-duration-log.yaml"]
    (scratch / "sources.json").write_text(json.dumps({str(p): hashlib.sha256(p.read_bytes()).hexdigest()
        for p in sorted(paths)}, indent=2) + "\n")
    (scratch / "qualification.json").write_text(json.dumps(dict(
        compiler=version, uci=args.version, policies=["normal", "Check", "Ignore"],
        checks_per_cell=29, rejections_per_cell=9, status="PASS", full_service_qualified=True,
        binary_length_profile="unconstrained octets: odd lexical lengths reject; empty and 3 octets accepted",
        constrained_binary_length_evidence="separate original and extracted Task 093 consumers"), indent=2) + "\n")
    print(f"TASK093_LOG_QUALIFICATION_PASS uci={args.version} compiler={version} policies=3")


if __name__ == "__main__":
    main()