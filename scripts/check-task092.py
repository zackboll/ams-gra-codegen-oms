#!/usr/bin/env python3
"""Production-generated repeated Choice qualification; no skipped compiler cells."""
import argparse
import hashlib
import importlib.util
import json
import pathlib
import re
import shutil
import subprocess
import sys
import tomllib

ROOT = pathlib.Path(__file__).resolve().parents[1]
sys.dont_write_bytecode = True
spec = importlib.util.spec_from_file_location("integration", ROOT / "scripts/check-task091r.py")
integration = importlib.util.module_from_spec(spec)
spec.loader.exec_module(integration)
checked = integration.checked


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--compiler", choices=["14", "16"], required=True)
    parser.add_argument("--scratch", type=pathlib.Path, required=True)
    parser.add_argument("--generator", type=pathlib.Path, required=True)
    parser.add_argument("--profile", choices=["oam", "qualified"], default="oam")
    args = parser.parse_args()
    scratch = args.scratch.resolve()
    scratch.mkdir(parents=True, exist_ok=False)
    fixtures = ROOT / "tests/fixtures/service-generate"
    suffix = "-qualified" if args.profile == "qualified" else ""
    schema = fixtures / f"codec-ada-repeated-choice{suffix}.xsd"
    golden = fixtures / ("task092-qualified.json" if suffix else "task092-populated.json")
    contract = fixtures / "codec-ada-repeated-choice.yaml"
    consumer = scratch / "consumer"
    common = ["--contract", str(contract), "--language", "ada", "--world",
              "closed-schema", "--schema", str(schema)]
    generator = str(args.generator.resolve())
    checked([generator, "service-check", *common, "--with-codec"], ROOT, scratch / "readiness.log")
    checked([generator, "service-generate", *common, "--with-codec", "--output", str(consumer)],
            ROOT, scratch / "generate.log")
    model = scratch / "model"
    checked([generator, "service-generate", *common, "--output", str(model)],
            ROOT, scratch / "model-generate.log")
    hashes = {}
    for path in sorted(model.rglob("*")):
        if path.is_file():
            relative = path.relative_to(model)
            if path.read_bytes() != (consumer / relative).read_bytes():
                raise RuntimeError(f"model/API changed: {relative}")
            hashes[str(relative)] = hashlib.sha256(path.read_bytes()).hexdigest()
    (scratch / "model-byte-parity.json").write_text(json.dumps(hashes, indent=2) + "\n")
    text = schema.read_text()
    controls = [
        ("repeated-alternative", text.replace('name="Enabled" type="xs:boolean"',
                                             'name="Enabled" type="xs:boolean" maxOccurs="2"')),
        ("optional-alternative", text.replace('name="Enabled" type="xs:boolean"',
                                             'name="Enabled" type="xs:boolean" minOccurs="0"')),
        ("nillable-alternative", text.replace('name="Enabled" type="xs:boolean"',
                                             'name="Enabled" type="xs:boolean" nillable="true"')),
        ("unqualified-alternative", text.replace('name="Enabled" type="xs:boolean"',
                                                'name="Enabled" type="xs:boolean" form="unqualified"')),
        ("abstract-choice", text.replace('name="Selection"><xs:choice>',
                                         'name="Selection" abstract="true"><xs:choice>')),
        ("temporal-alternative", text.replace('name="Enabled" type="xs:boolean"',
                                             'name="Enabled" type="xs:duration"')),
        ("missing-dependency", text.replace('name="Selection"><xs:choice>',
                                            'name="Missing"><xs:choice>')),
        ("cyclic-dependency", text.replace('name="Enabled" type="xs:boolean"',
                                           'name="Enabled" type="' + ("t:" if suffix else "uci:") + 'RepeatedPayload"')),
    ]
    for name, malformed in controls:
        path = scratch / (name + ".xsd")
        path.write_text(malformed)
        for action in ["service-check", "service-generate"]:
            output = scratch / (name + "-output")
            command = [generator, action, *common[:-2], "--schema", str(path),
                       "--with-codec"]
            if action == "service-generate":
                command += ["--output", str(output)]
            result = subprocess.run(command, cwd=ROOT, capture_output=True, check=False)
            log = scratch / f"{name}-{action}.log"
            log.write_bytes(result.stdout + result.stderr)
            log.with_suffix(".command.json").write_text(json.dumps(
                {"command": command, "cwd": str(ROOT), "exit": result.returncode}, indent=2) + "\n")
            if result.returncode != 1 or output.exists():
                raise RuntimeError(f"{name}: {action} did not reject atomically")
    source = (ROOT / "tests/ada-generated-codec/repeated_choice_consumer.adb").read_text()
    if suffix:
        source = source.replace("Programs.OAM", "Urn.Task092")
        keys = ["RepeatedNotice", "Bounded", "PositiveBounded", "Unbounded",
                "PositiveUnbounded", "Single", "Enabled", "State", "Nested",
                "Observed", "Elapsed", "Unknown"]
        source = re.sub('"(' + '|'.join(keys) + ')"',
                        lambda match: '"{urn:task092}' + match[1] + '"', source)
    (consumer / "repeated_choice_consumer.adb").write_text(source)
    shutil.copytree(ROOT / "ada/oms-json-support", scratch / "support")
    compiler = "gnat_external" if args.compiler == "14" else "gnat_native"
    version = "14.2.0" if args.compiler == "14" else "16.1.0"
    (consumer / "alire.toml").write_text(
        'name = "task092_consumer"\nversion = "0.1.0-dev"\n'
        'description = "Repeated Choice qualification"\nlicenses = "MIT"\n'
        'project-files = ["consumer.gpr"]\n[[depends-on]]\n'
        'ams_gra_oms_json = "=0.1.0-dev"\ngprbuild = "=26.0.1"\n'
        f'{compiler} = "={version}"\n[[pins]]\n'
        'ams_gra_oms_json = { path = "../support" }\n')
    (consumer / "consumer.gpr").write_text(
        'with "ams_gra_oms_json";\nproject Consumer is\n'
        'for Source_Dirs use ("."); for Object_Dir use "obj";\n'
        'for Exec_Dir use "bin"; for Main use ("repeated_choice_consumer.adb");\n'
        'package Compiler is for Default_Switches ("Ada") use '
        '("-gnat2022", "-gnato", "-O0"); end Compiler;\nend Consumer;\n')
    if checked(["alr", "--version"], scratch, scratch / "alire.log").strip() != b"alr 2.1.1":
        raise RuntimeError("Alire version differs")
    checked(["alr", "-n", "settings", "--set", "dependencies.shared", "false"],
            consumer, scratch / "isolation.log")
    options = ["--force"] if args.compiler == "14" else []
    checked(["alr", "-n", *options, "toolchain", "--select", "--local",
             f"{compiler}={version}", "gprbuild=26.0.1"], consumer, scratch / "toolchain.log")
    for policy in ["normal", "Check", "Ignore"]:
        command = ["alr", "-n", "build", "--", "-f"]
        if policy != "normal":
            adc = scratch / (policy + ".adc")
            adc.write_text(f"pragma Assertion_Policy ({policy});\n")
            command += ["-cargs:Ada", f"-gnatec={adc}"]
        checked(command, consumer, scratch / (policy + "-build.log"))
        lock = consumer / "alire/alire.lock"
        integration.datetime.guard.validate_solution(tomllib.loads(lock.read_text()), compiler)
        shutil.copy2(lock, scratch / (policy + "-alire.lock"))
        for tool, identity in [("gnat", f"GNAT {version}"), ("gprbuild", "26.0.0")]:
            if identity.encode() not in checked(["alr", "-n", "exec", "--", tool, "--version"],
                                               consumer, scratch / f"{policy}-{tool}.log"):
                raise RuntimeError(f"wrong {tool}")
        output = checked([str(consumer / "bin/repeated_choice_consumer"),
                          str(golden)], consumer,
                         scratch / (policy + "-run.log"))
        if output != b"TASK092_CODEC_PASS checks= 63 rejections= 17\n":
            raise RuntimeError(f"incorrect completion: {output!r}")
    paths = [p for p in scratch.rglob("*") if p.is_file() and p.suffix in [".ads", ".adb", ".lock"]]
    paths += [args.generator.resolve(), schema, contract, golden,
              ROOT / "tests/ada-generated-codec/repeated_choice_consumer.adb"]
    (scratch / "sources.sha256").write_text("".join(
        f"{hashlib.sha256(p.read_bytes()).hexdigest()}  {p}\n" for p in sorted(paths)))
    print(f"TASK092 QUALIFICATION PASSED compiler={version} policies=3")


if __name__ == "__main__":
    main()