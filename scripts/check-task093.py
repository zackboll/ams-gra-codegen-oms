#!/usr/bin/env python3
"""Compile and execute isolated production-generated single-valued hexBinary codecs."""
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
spec = importlib.util.spec_from_file_location("duration", ROOT / "scripts/check-task091.py")
duration = importlib.util.module_from_spec(spec)
spec.loader.exec_module(duration)
checked = duration.checked


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--compiler", choices=["14", "16"], required=True)
    parser.add_argument("--scratch", type=pathlib.Path, required=True)
    parser.add_argument("--generator", type=pathlib.Path, required=True)
    parser.add_argument("--qualified", action="store_true")
    args = parser.parse_args()
    scratch = args.scratch.resolve()
    scratch.mkdir(parents=True, exist_ok=False)
    fixtures = ROOT / "tests/fixtures/service-generate"
    schema = fixtures / "codec-ada-hexbinary-single.xsd"
    contract = fixtures / "codec-ada-hexbinary-single.yaml"
    golden = fixtures / "task093-hexbinary.json"
    namespace = "https://www.vdl.afrl.af.mil/programs/task093"
    if args.qualified:
        original = schema.read_text()
        schema = scratch / "qualified.xsd"
        schema.write_text(original.replace(
            "https://www.vdl.afrl.af.mil/programs/oam", namespace).replace(
            'elementFormDefault="qualified"',
            'xmlns:oam="https://www.vdl.afrl.af.mil/programs/oam" elementFormDefault="qualified"').replace(
            'uci:version=', 'oam:version='))
        def qualify(value):
            return {f"{{{namespace}}}{key}": qualify(child) for key, child in value.items()} if isinstance(value, dict) else value
        qualified_golden = scratch / "qualified.json"
        qualified_golden.write_text(json.dumps(qualify(json.loads(golden.read_text()))) + "\n")
        golden = qualified_golden
    common = [str(args.generator.resolve()), "service-generate", "--schema", str(schema),
              "--contract", str(contract), "--language", "ada", "--world", "closed-schema"]
    model = scratch / "model"
    consumer = scratch / "consumer"
    checked([*common, "--output", str(model)], ROOT, scratch / "model-generate.log")
    checked([*common, "--with-codec", "--output", str(consumer)], ROOT, scratch / "codec-generate.log")
    source = schema.read_text()
    controls = [
        ("direct-repeated", source.replace('name="Direct" type="xs:hexBinary"',
          'name="Direct" type="xs:hexBinary" maxOccurs="2"'), "codec-readiness", "direct Binary repeated element is not qualified"),
        ("named-repeated", source.replace('name="Named" type="uci:Bytes"',
          'name="Named" type="uci:Bytes" maxOccurs="2"'), "codec-readiness", "named Binary repeated element is not qualified"),
        ("base64", source.replace('base="xs:hexBinary"', 'base="xs:base64Binary"'), "frontend", "unsupported XSD construct: xs:base64Binary"),
        ("field-facets", source.replace('<xs:element name="Direct" type="xs:hexBinary"/>',
          '<xs:element name="Direct"><xs:simpleType><xs:restriction base="xs:hexBinary">'
          '<xs:length value="2"/></xs:restriction></xs:simpleType></xs:element>'), "frontend", "unsupported XSD construct: xs:anonymous element type"),
        ("direct-choice", source.replace('<xs:element name="Direct" type="xs:hexBinary"/>',
          '<xs:choice><xs:element name="Direct" type="xs:hexBinary"/></xs:choice>'), "frontend", "unsupported XSD construct: xs:choice"),
        ("named-choice", source.replace('<xs:element name="Named" type="uci:Bytes"/>',
          '<xs:choice><xs:element name="Named" type="uci:Bytes"/></xs:choice>'), "frontend", "unsupported XSD construct: xs:choice"),
        ("named-pattern", source.replace('<xs:restriction base="xs:hexBinary"/>',
          '<xs:restriction base="xs:hexBinary"><xs:pattern value="[0-9A-F]*"/></xs:restriction>'),
         "frontend", "unsupported XSD construct: xs:pattern"),
        ("invalid-cardinality", source.replace('name="Direct" type="xs:hexBinary"',
          'name="Direct" type="xs:hexBinary" minOccurs="2" maxOccurs="1"'),
         "frontend", "minOccurs exceeds maxOccurs on element Direct"),
    ]
    for label, type_name in [("direct", "xs:hexBinary"), ("named", "uci:Bytes")]:
        choice = ('<xs:complexType name="BinaryChoice"><xs:choice>'
                  f'<xs:element name="Data" type="{type_name}"/>'
                  '</xs:choice></xs:complexType>')
        text = source.replace('<xs:complexType name="BinaryBase">',
                              choice + '<xs:complexType name="BinaryBase">').replace(
                                  'name="Direct" type="xs:hexBinary"',
                                  'name="Direct" type="uci:BinaryChoice"')
        controls.append((f"{label}-choice-capability", text, "codec-readiness",
                         f"{label} Binary Choice alternative is not qualified"))
    for name, text, stage, diagnostic in controls:
        if text == source:
            raise RuntimeError(f"ineffective negative control: {name}")
        path = scratch / f"{name}.xsd"
        path.write_text(text)
        output = scratch / f"{name}-output"
        for action in ["service-check", "service-generate"]:
            command = [*common, "--with-codec"]
            command[1] = action
            command[command.index(str(schema))] = str(path)
            if action == "service-generate":
                command += ["--output", str(output)]
            result = subprocess.run(command, cwd=ROOT, capture_output=True, check=False)
            stem = scratch / f"{name}-{action}"
            stem.with_suffix(".command.json").write_text(json.dumps(
                dict(command=command, exit=result.returncode, cwd=str(ROOT),
                     stage=stage, diagnostic=diagnostic), indent=2) + "\n")
            stem.with_suffix(".stage").write_text(stage + "\n")
            stem.with_suffix(".log").write_bytes(result.stdout + result.stderr)
            if result.returncode != 1 or output.exists() or diagnostic.encode() not in (result.stdout + result.stderr):
                raise RuntimeError(f"negative control did not fail atomically at {stage}: {name} {action}")
    parity = {}
    for path in sorted(model.iterdir()):
        if path.read_bytes() != (consumer / path.name).read_bytes():
            raise RuntimeError(f"model/API changed: {path.name}")
        parity[path.name] = hashlib.sha256(path.read_bytes()).hexdigest()
    (scratch / "model-byte-parity.json").write_text(json.dumps(parity, indent=2) + "\n")
    shutil.copy2(ROOT / "tests/ada-generated-codec/hexbinary_consumer.adb", consumer)
    shutil.copy2(ROOT / "tests/ada-generated-codec/hexbinary_model_consumer.adb", consumer)
    if args.qualified:
        for name in ["hexbinary_consumer.adb", "hexbinary_model_consumer.adb"]:
            path = consumer / name
            text = path.read_text().replace("Programs.Oam", "Programs.Task093")
            for key in ["Direct", "OptionalDirect", "OptionalNamed", "ExactValue", "BoundedValue",
                        "Empty", "Unknown", "Nested", "Details", "LaterDetails"]:
                text = text.replace(f'"{key}"', f'"{{{namespace}}}{key}"')
            path.write_text(text)
    shutil.copytree(ROOT / "ada/oms-json-support", scratch / "support")
    compiler = "gnat_external" if args.compiler == "14" else "gnat_native"
    version = "14.2.0" if args.compiler == "14" else "16.1.0"
    (consumer / "alire.toml").write_text(
        'name = "task093_consumer"\nversion = "0.1.0-dev"\n'
        'description = "hexBinary qualification"\nlicenses = "MIT"\n'
        'project-files = ["consumer.gpr"]\n[[depends-on]]\n'
        'ams_gra_oms_json = "=0.1.0-dev"\ngprbuild = "=26.0.1"\n'
        f'{compiler} = "={version}"\n[[pins]]\n'
        'ams_gra_oms_json = { path = "../support" }\n')
    (consumer / "consumer.gpr").write_text(
        'with "ams_gra_oms_json";\nproject Consumer is\n'
        'for Source_Dirs use ("."); for Object_Dir use "obj";\n'
        'for Exec_Dir use "bin"; for Main use ("hexbinary_consumer.adb", "hexbinary_model_consumer.adb");\n'
        'package Compiler is for Default_Switches ("Ada") use '
        '("-gnat2022", "-gnato", "-O0"); end Compiler;\nend Consumer;\n')
    if checked(["alr", "--version"], scratch, scratch / "alire.log").strip() != b"alr 2.1.1":
        raise RuntimeError("Alire identity differs")
    checked(["alr", "-n", "settings", "--set", "dependencies.shared", "false"],
            consumer, scratch / "isolation.log")
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
        duration.guard.validate_solution(tomllib.loads(lock.read_text()), compiler)
        shutil.copy2(lock, scratch / f"{policy}-alire.lock")
        identity = checked(["alr", "-n", "exec", "--", "gnat", "--version"],
                           consumer, scratch / f"{policy}-gnat.log")
        if f"GNAT {version}".encode() not in identity:
            raise RuntimeError("GNAT identity differs")
        identity = checked(["alr", "-n", "exec", "--", "gprbuild", "--version"],
                           consumer, scratch / f"{policy}-gprbuild.log")
        if b"26.0.0" not in identity:
            raise RuntimeError("GPRBUILD identity differs")
        output = checked([str(consumer / "bin/hexbinary_consumer"),
                          str(golden)],
                         consumer, scratch / f"{policy}-run.log")
        if output != b"TASK093_CODEC_COMPLETE cases= 33\n":
            raise RuntimeError(f"incorrect completion marker: {output!r}")
        output = checked([str(consumer / "bin/hexbinary_model_consumer")],
                         consumer, scratch / f"{policy}-model-run.log")
        if output != b"TASK093_MODEL_COMPLETE cases= 6\n":
            raise RuntimeError(f"incorrect model completion marker: {output!r}")
    hashes = {path.name: hashlib.sha256(path.read_bytes()).hexdigest()
              for path in sorted(consumer.glob("*.ad?"))}
    (scratch / "sources.json").write_text(json.dumps(hashes, indent=2) + "\n")
    (scratch / "qualification.json").write_text(json.dumps(dict(
        compiler=version, qualified_non_oam=args.qualified,
        policies=["normal", "Check", "Ignore"], codec_checks_per_cell=33,
        model_checks_per_cell=6, generator_negative_cases=len(controls),
        generator_negative_executions=len(controls) * 2,
        decoder_stage="executable-Ada-decode", status="PASS"), indent=2) + "\n")
    print(f"TASK093 QUALIFICATION PASSED compiler={version} policies=3")


if __name__ == "__main__":
    main()