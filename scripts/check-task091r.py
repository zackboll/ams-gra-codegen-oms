#!/usr/bin/env python3
"""Mandatory production-generated Duration/DateTime/Choice/repetition integration checks."""
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
spec = importlib.util.spec_from_file_location("datetime", ROOT / "scripts/check-task090.py")
datetime = importlib.util.module_from_spec(spec)
spec.loader.exec_module(datetime)


def checked(command, cwd, log):
    result = subprocess.run(command, cwd=cwd, stdout=subprocess.PIPE,
                            stderr=subprocess.STDOUT, check=False)
    log.write_bytes(result.stdout)
    log.with_suffix(".command.json").write_text(json.dumps(
        {"command": command, "cwd": str(cwd), "exit": result.returncode}, indent=2) + "\n")
    print(f"exit={result.returncode} log={log}", flush=True)
    if result.returncode:
        raise RuntimeError(f"command failed: {command}; see {log}")
    return result.stdout


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--compiler", choices=["14", "16"], default="16")
    parser.add_argument("--scratch", type=pathlib.Path, required=True)
    parser.add_argument("--generator", type=pathlib.Path, required=True)
    args = parser.parse_args()
    scratch = args.scratch.resolve()
    scratch.mkdir(parents=True, exist_ok=False)
    fixtures = ROOT / "tests/fixtures/service-generate"
    schema = fixtures / "codec-ada-duration-combined.xsd"
    contract = fixtures / "codec-ada-duration-combined.yaml"
    consumer = scratch / "consumer"
    common = ["--contract", str(contract), "--language", "ada", "--world",
              "closed-schema", "--with-codec"]
    generator = str(args.generator.resolve())
    checked([generator, "service-check", *common, "--schema", str(schema)],
            ROOT, scratch / "readiness.log")
    checked([generator, "service-generate", *common, "--schema", str(schema),
             "--output", str(consumer)], ROOT, scratch / "generate.log")
    model = scratch / "model"
    checked([generator, "service-generate", *[x for x in common if x != "--with-codec"],
             "--schema", str(schema), "--output", str(model)], ROOT, scratch / "model-generate.log")
    model_hashes = {}
    for path in sorted(model.rglob("*")):
        if path.is_file():
            relative = path.relative_to(model)
            if path.read_bytes() != (consumer / relative).read_bytes():
                raise RuntimeError(f"model/API bytes changed: {relative}")
            model_hashes[str(relative)] = hashlib.sha256(path.read_bytes()).hexdigest()
    (scratch / "model-byte-parity.json").write_text(json.dumps(model_hashes, indent=2) + "\n")
    body = (consumer / "service_codec.adb").read_text()
    for helper in ["Scalar_Duration", "Scalar_Date_Time"]:
        if body.count("function " + helper + " (") != 1:
            raise RuntimeError(f"missing or duplicated helper: {helper}")
    text = schema.read_text()
    controls = [
        ("direct-choice", text.replace('name="Enabled" type="xs:boolean"',
                                       'name="Enabled" type="xs:dateTime"'),
         "direct DateTime Choice alternative is unsupported"),
        ("direct-bounded", text.replace('name="Values" type="xs:byte"',
                                        'name="Values" type="xs:dateTime"'),
         "direct DateTime repeated element is unsupported"),
        ("direct-unbounded", text.replace('name="Values" type="xs:byte" minOccurs="0" maxOccurs="3"',
                                          'name="Values" type="xs:dateTime" minOccurs="0" maxOccurs="unbounded"'),
         "direct DateTime repeated element is unsupported"),
        ("named-facets", text.replace('value=".+Z"', 'value=".*Z"'), "unsupported selected types:"),
        ("direct-facets", text.replace('<xs:element name="Direct" type="xs:dateTime"/>',
                                       '<xs:element name="Direct"><xs:simpleType><xs:restriction base="xs:dateTime"><xs:pattern value=".+Z"/></xs:restriction></xs:simpleType></xs:element>'),
         "unsupported XSD construct: xs:anonymous element type"),
        ("named-choice", text.replace('name="Enabled" type="xs:boolean"',
                                      'name="Enabled" type="uci:Instant"'),
         "named DateTime Choice alternative is not qualified"),
        ("named-repeated", text.replace('name="Values" type="xs:byte"',
                                        'name="Values" type="uci:Instant"'),
         "named DateTime repeated element is not qualified"),
    ]
    controls += [
        ("duration-direct-choice", text.replace('name="Enabled" type="xs:boolean"',
                                                'name="Enabled" type="xs:duration"'),
         "direct Duration Choice alternative is not qualified"),
        ("duration-named-choice", text.replace('name="Enabled" type="xs:boolean"',
                                               'name="Enabled" type="uci:Span"'),
         "named Duration Choice alternative is not qualified"),
        ("duration-named-facets", text.replace('<xs:restriction base="xs:duration"/>',
                                               '<xs:restriction base="xs:duration"><xs:pattern value="P.*"/></xs:restriction>'),
         "unsupported XSD construct: xs:pattern"),
    ]
    for kind, type_name in [("direct", "xs:duration"), ("named", "uci:Span")]:
        for bound in ["3", "unbounded"]:
            controls.append((f"duration-{kind}-repeated-{bound}",
                text.replace('name="Values" type="xs:byte" minOccurs="0" maxOccurs="3"',
                             f'name="Values" type="{type_name}" minOccurs="0" maxOccurs="{bound}"'),
                f"{kind} Duration repeated element is not qualified"))
    for name, bad, diagnostic in controls:
        if bad == text:
            raise RuntimeError(f"ineffective control: {name}")
        path = scratch / (name + ".xsd")
        path.write_text(bad)
        output = scratch / (name + "-output")
        for action in ["service-check", "service-generate"]:
            command = [generator, action, *common, "--schema", str(path)]
            if action == "service-generate":
                command += ["--output", str(output)]
            result = subprocess.run(command, cwd=ROOT, capture_output=True, check=False)
            stem = scratch / f"{name}-{action}"
            stem.with_suffix(".command.json").write_text(json.dumps(command) + "\n")
            stem.with_suffix(".log").write_bytes(result.stdout + result.stderr)
            stem.with_suffix(".exit").write_text(str(result.returncode) + "\n")
            stage = "frontend" if diagnostic.startswith("unsupported XSD construct:") else "capability"
            stem.with_suffix(".stage").write_text(stage + "\n")
            if result.returncode != 1 or output.exists() or diagnostic.lower().encode() not in (result.stdout + result.stderr).lower():
                raise RuntimeError(f"{name}: {action} did not reject atomically with {diagnostic}")
    shutil.copy2(ROOT / "tests/ada-generated-codec/duration_combined_consumer.adb", consumer)
    shutil.copytree(ROOT / "ada/oms-json-support", scratch / "support")
    compiler = "gnat_external" if args.compiler == "14" else "gnat_native"
    version = "14.2.0" if args.compiler == "14" else "16.1.0"
    (consumer / "alire.toml").write_text(
        'name = "task091r_consumer"\nversion = "0.1.0-dev"\n'
        'description = "DateTime composition qualification"\nlicenses = "MIT"\n'
        'project-files = ["consumer.gpr"]\n[[depends-on]]\n'
        'ams_gra_oms_json = "=0.1.0-dev"\ngprbuild = "=26.0.1"\n'
        f'{compiler} = "={version}"\n[[pins]]\n'
        'ams_gra_oms_json = { path = "../support" }\n')
    (consumer / "consumer.gpr").write_text(
        'with "ams_gra_oms_json";\nproject Consumer is\n'
        'for Source_Dirs use ("."); for Object_Dir use "obj";\n'
        'for Exec_Dir use "bin"; for Main use ("duration_combined_consumer.adb");\n'
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
        datetime.guard.validate_solution(tomllib.loads(lock.read_text()), compiler)
        shutil.copy2(lock, scratch / (policy + "-alire.lock"))
        for tool, identity in [("gnat", f"GNAT {version}"), ("gprbuild", "26.0.0")]:
            if identity.encode() not in checked(["alr", "-n", "exec", "--", tool, "--version"],
                                               consumer, scratch / f"{policy}-{tool}.log"):
                raise RuntimeError(f"wrong {tool}")
        output = checked([str(consumer / "bin/duration_combined_consumer"),
                          str(fixtures / "task091r-combined.json")], consumer,
                         scratch / (policy + "-run.log"))
        if output != b"TASK091R_COMBINED_PASS checks= 52 rejections= 20\n":
            raise RuntimeError("combined corpus did not complete")
    paths = [p for p in scratch.rglob("*") if p.is_file() and p.suffix in [".ads", ".adb", ".lock"]]
    paths += [args.generator.resolve(), schema, contract, fixtures / "task091r-combined.json",
              ROOT / "tests/ada-generated-codec/duration_combined_consumer.adb"]
    (scratch / "sources.sha256").write_text("".join(
        f"{hashlib.sha256(p.read_bytes()).hexdigest()}  {p}\n" for p in sorted(paths)))
    print(f"TASK091R QUALIFICATION PASSED compiler={version} policies=3 controls={len(controls)}")


if __name__ == "__main__":
    main()