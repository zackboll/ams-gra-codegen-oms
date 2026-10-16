#!/usr/bin/env python3
"""Qualify extracted, unchanged pinned UCI Binary definitions, not full Log services."""
import argparse
import hashlib
import importlib.util
import json
import pathlib
import re
import shutil
import sys
import tomllib

ROOT = pathlib.Path(__file__).resolve().parents[1]
sys.dont_write_bytecode = True
spec = importlib.util.spec_from_file_location("binary", ROOT / "scripts/check-task093.py")
binary = importlib.util.module_from_spec(spec)
spec.loader.exec_module(binary)
checked = binary.checked
PINS = {
    "2.5": "ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27",
    "2.6": "af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b",
}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--compiler", choices=["14", "16"], required=True)
    parser.add_argument("--version", choices=list(PINS), required=True)
    parser.add_argument("--schema", type=pathlib.Path, required=True)
    parser.add_argument("--scratch", type=pathlib.Path, required=True)
    parser.add_argument("--generator", type=pathlib.Path, required=True)
    args = parser.parse_args()
    scratch = args.scratch.resolve()
    scratch.mkdir(parents=True, exist_ok=False)
    raw = args.schema.read_bytes()
    if hashlib.sha256(raw).hexdigest() != PINS[args.version]:
        raise RuntimeError("real UCI root hash differs")
    text = raw.decode()
    extracted = {}

    def declaration(kind, name):
        match = re.search(rf'<xs:{kind}\b[^>]*name="{name}"[^>]*>.*?</xs:{kind}>', text, re.S)
        if not match:
            raise RuntimeError(f"missing real declaration: {name}")
        extracted[name] = match.group()
        return match.group()

    def member(owner, name):
        parent = declaration("complexType", owner)
        match = re.search(rf'<xs:element\b[^>]*name="{name}"[^>]*>.*?</xs:element>', parent, re.S)
        if not match:
            raise RuntimeError(f"missing real member: {owner}.{name}")
        extracted[f"{owner}.{name}"] = match.group()
        return match.group()

    data = member("LogMDT", "Data")
    hash_member = member("FileMetadataMDT", "SHA_2_Hash")
    # Inspect the other priority fields without substituting them for Log.
    member("SubsystemStreamMDT", "SubsystemStreamBinary")
    member("ProductMetadataMDT", "SHA_2_Hash")
    declarations = declaration("simpleType", "SHA_2_256_HashType")
    if args.version == "2.6":
        declarations += declaration("simpleType", "HexBinaryType")
    schema = scratch / "real-binary.xsd"
    schema.write_text(
        '<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" '
        'xmlns:uci="https://www.vdl.afrl.af.mil/programs/oam" '
        'targetNamespace="https://www.vdl.afrl.af.mil/programs/oam" elementFormDefault="qualified">'
        + declarations + '<xs:complexType name="BinaryProbe"><xs:sequence>' + data + hash_member +
        '</xs:sequence></xs:complexType>'
        '<xs:element name="BinaryNotice" type="uci:BinaryProbe" uci:version="001.000.000.000"/>'
        '</xs:schema>')
    (scratch / "provenance.json").write_text(json.dumps(dict(
        root=str(args.schema.resolve()), root_sha256=PINS[args.version],
        scope="extracted unchanged real Binary declarations/members; synthetic envelope, NOT full Log",
        fragments={name: dict(text=value, sha256=hashlib.sha256(value.encode()).hexdigest())
                   for name, value in extracted.items()}), indent=2) + "\n")
    consumer = scratch / "consumer"
    common = [str(args.generator.resolve()), "service-generate", "--schema", str(schema),
              "--contract", str(ROOT / "tests/fixtures/service-generate/codec-ada-hexbinary-single.yaml"),
              "--language", "ada", "--world", "closed-schema"]
    checked([*common, "--with-codec", "--output", str(consumer)], ROOT, scratch / "generate.log")
    plain = scratch / "model"
    checked([*common, "--output", str(plain)], ROOT, scratch / "model-generate.log")
    for path in plain.iterdir():
        if path.read_bytes() != (consumer / path.name).read_bytes():
            raise RuntimeError(f"real model/API differs: {path.name}")
    source = (ROOT / "tests/ada-generated-codec/hexbinary_real_consumer.adb").read_text()
    if args.version == "2.6":
        source = source.replace(":= Item.Data.Value;", ":= Item.Data.Value.Value;")
    (consumer / "hexbinary_real_consumer.adb").write_text(source)
    shutil.copytree(ROOT / "ada/oms-json-support", scratch / "support")
    compiler = "gnat_external" if args.compiler == "14" else "gnat_native"
    version = "14.2.0" if args.compiler == "14" else "16.1.0"
    (consumer / "alire.toml").write_text(
        'name = "task093_real"\nversion = "0.1.0-dev"\n'
        'description = "Real Binary qualification"\nlicenses = "MIT"\n'
        'project-files = ["consumer.gpr"]\n[[depends-on]]\n'
        'ams_gra_oms_json = "=0.1.0-dev"\ngprbuild = "=26.0.1"\n'
        f'{compiler} = "={version}"\n[[pins]]\n'
        'ams_gra_oms_json = { path = "../support" }\n')
    (consumer / "consumer.gpr").write_text(
        'with "ams_gra_oms_json";\nproject Consumer is\n'
        'for Source_Dirs use ("."); for Object_Dir use "obj"; for Exec_Dir use "bin";\n'
        'for Main use ("hexbinary_real_consumer.adb");\n'
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
        binary.duration.guard.validate_solution(tomllib.loads(lock.read_text()), compiler)
        shutil.copy2(lock, scratch / f"{policy}-alire.lock")
        for tool, expected in [("gnat", f"GNAT {version}"), ("gprbuild", "26.0.0")]:
            identity = checked(["alr", "-n", "exec", "--", tool, "--version"], consumer,
                               scratch / f"{policy}-{tool}.log")
            if expected.encode() not in identity:
                raise RuntimeError(f"{tool} identity differs")
        output = checked([str(consumer / "bin/hexbinary_real_consumer")], consumer,
                         scratch / f"{policy}-run.log")
        if output != b"TASK093_REAL_COMPLETE cases= 14\n":
            raise RuntimeError(f"real completion differs: {output!r}")
    (scratch / "sources.json").write_text(json.dumps({path.name: hashlib.sha256(path.read_bytes()).hexdigest()
        for path in sorted(consumer.glob("*.ad?"))}, indent=2) + "\n")
    (scratch / "qualification.json").write_text(json.dumps(dict(
        compiler=version, uci=args.version, policies=["normal", "Check", "Ignore"],
        checks_per_cell=14, status="PASS", full_service_qualified=False), indent=2) + "\n")
    print(f"TASK093 REAL BINARY PASSED uci={args.version} compiler={version} policies=3")


if __name__ == "__main__":
    main()