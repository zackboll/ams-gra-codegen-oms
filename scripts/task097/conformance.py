"""Generated composition/atomicity/resource/facet controls beyond the numerical corpus."""
import argparse
import hashlib
import json
from pathlib import Path
import shlex
import shutil
import struct
import subprocess

ROOT = Path(__file__).resolve().parents[2]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--scratch", type=Path, required=True)
    parser.add_argument("--generated", type=Path, required=True)
    parser.add_argument("--generator", type=Path, required=True)
    args = parser.parse_args()
    scratch, generated = args.scratch.resolve(), args.generated.resolve()
    prior = json.loads((generated / "result.json").read_text())
    if not prior.get("completed"):
        raise RuntimeError("numerical generated matrix not complete")
    scratch.mkdir(parents=True, exist_ok=False)
    namespace = prior["namespace"]
    schema = (ROOT / "tests/fixtures/service-generate/codec-ada-float64.xsd").read_text()
    schema = schema.replace(' <xs:complexType name="PathLength"><xs:sequence>', '''
 <xs:simpleType name="ExclusiveNumber"><xs:restriction base="xs:double"><xs:minExclusive value="0"/><xs:maxInclusive value="1"/></xs:restriction></xs:simpleType>
 <xs:simpleType name="Digest"><xs:restriction base="xs:hexBinary"><xs:minLength value="1"/><xs:maxLength value="2"/></xs:restriction></xs:simpleType>
 <xs:simpleType name="Mode"><xs:restriction base="xs:string"><xs:enumeration value="A"/><xs:enumeration value="B"/></xs:restriction></xs:simpleType>
 <xs:complexType name="Emitter_Base"><xs:sequence><xs:element name="Inherited" type="xs:double"/></xs:sequence></xs:complexType>
 <xs:complexType name="PathLength"><xs:complexContent><xs:extension base="uci:Emitter_Base"><xs:sequence>''')
    schema = schema.replace(' </xs:sequence></xs:complexType>\n <xs:complexType name="ArraySample">', '''
  <xs:element name="Unbounded" type="xs:double" minOccurs="0" maxOccurs="unbounded"/>
  <xs:element name="NamedBounded" type="uci:TargetGet" minOccurs="0" maxOccurs="3"/>
  <xs:element name="Positive" type="uci:ExclusiveNumber" minOccurs="0"/>
  <xs:element name="Text" type="xs:string"/>
  <xs:element name="Digest" type="uci:Digest" minOccurs="0"/>
  <xs:element name="Mode" type="uci:Mode" minOccurs="0"/>
  <xs:element name="Date" type="xs:dateTime" minOccurs="0"/>
  <xs:element name="Span" type="xs:duration" minOccurs="0"/>
  <xs:element name="Count" type="xs:int" minOccurs="0"/>
 </xs:sequence></xs:extension></xs:complexContent></xs:complexType>
 <xs:complexType name="ArraySample">''')
    if namespace == "qualified":
        schema = schema.replace('xmlns:uci=', 'xmlns:version="https://www.vdl.afrl.af.mil/programs/oam" xmlns:uci=')
        schema = schema.replace('xmlns:uci="https://www.vdl.afrl.af.mil/programs/oam"', 'xmlns:uci="urn:task097"')
        schema = schema.replace('targetNamespace="https://www.vdl.afrl.af.mil/programs/oam"', 'targetNamespace="urn:task097"').replace('uci:version=', 'version:version=')
    schema_path = scratch / "conformance.xsd"
    schema_path.write_text(schema)
    consumer = scratch / "consumer"

    def run(label, command, cwd=ROOT):
        prefix = scratch / label
        prefix.with_suffix(".command.json").write_text(json.dumps({"argv": command, "cwd": str(cwd)}, indent=2) + "\n")
        with prefix.with_suffix(".stdout").open("wb") as out, prefix.with_suffix(".stderr").open("wb") as err:
            process = subprocess.run(command, cwd=cwd, stdout=out, stderr=err)
        prefix.with_suffix(".exit").write_text(str(process.returncode) + "\n")
        if process.returncode:
            raise RuntimeError(f"failed {label}")
        return prefix.with_suffix(".stdout").read_text()

    run("generate", [str(args.generator.resolve()), "service-generate", "--schema", str(schema_path),
        "--contract", str(ROOT / "tests/fixtures/service-generate/codec-ada-float64.yaml"), "--language", "ada",
        "--world", "closed-schema", "--with-codec", "--ada-codec-runtime", "linux-x86_64-glibc-2.41-lossless-v1", "--output", str(consumer)])
    for name in ["conformance.adb", "allocation_faults.ads", "allocation_faults.adb"]:
        source = (ROOT / "tests/ada-float64-codec" / name).read_text()
        if namespace == "qualified":
            source = source.replace("Programs.Oam", "Urn.Task097")
        (consumer / name).write_text(source)
    for directory in ["support", "provider"]:
        shutil.copytree(generated / directory, scratch / directory)
    environment = scratch / "environment.sh"
    environment.write_text((generated / "environment.sh").read_text().replace(str(generated / "provider"), str(scratch / "provider")))
    (consumer / "consumer.gpr").write_text(
        'with "../support/ams_gra_oms_json_lossless";\nproject Consumer is\n'
        'for Source_Dirs use ("."); for Object_Dir use "obj"; for Exec_Dir use "bin";\n'
        'for Main use ("conformance.adb");\npackage Compiler is\n'
        'for Default_Switches ("Ada") use ("-gnat2022", "-gnato", "-O0"); end Compiler;\n'
        'package Linker is for Default_Switches ("Ada") use ("-Wl,--wrap=__gnat_malloc"); end Linker;\nend Consumer;\n')
    prefix = "" if namespace == "oam" else "{urn:task097}"

    def key(name):
        return prefix + name

    record = {key("Inherited"): -0.0, key("Direct"): 1.0000000000000002,
        key("Optional"): -0.0, key("Named"): 5e-324, key("Checked"): -1,
        key("Data"): "80FF", key("Single"): 1.0000001192092896, key("Samples"): [0, 5e-324],
        key("NamedSamples"): [-1, 0.5], key("Unbounded"): [-0.0, 5e-324],
        key("NamedBounded"): [-1, 0.5], key("Positive"): 1,
        key("Text"): 'Unicode 🛰️ é " \\ \x00', key("Digest"): "80FF", key("Mode"): "B",
        key("Date"): "2026-01-02T03:04:05Z", key("Span"): "PT1.25S", key("Count"): 42}
    original = {key("Record"): record, key("Pick"): {key("Nested"): record}, key("Picks"): [{key("Nested"): record}]}
    valid = scratch / "valid.json"
    valid.write_text(json.dumps(original, ensure_ascii=False) + "\n")
    accepted = {"base": valid.read_text()}
    for name, special in [("nan", "NaN"), ("infinity", "Infinity"), ("negative-infinity", "-Infinity")]:
        obj = json.loads(valid.read_text())
        obj[key("Record")][key("Direct")] = special
        accepted[name] = json.dumps(obj)
    absent = json.loads(valid.read_text())
    for name in ["Optional", "Unbounded", "NamedBounded"]:
        del absent[key("Record")][key(name)]
    accepted["absent"] = json.dumps(absent)
    empty = json.loads(valid.read_text())
    for name in ["Samples", "Unbounded", "NamedBounded"]:
        empty[key("Record")][key(name)] = []
    accepted["empty"] = json.dumps(empty)
    escaped_key = json.dumps(key("Record")).replace("R", "\\u0052", 1)
    accepted["subtree-replacement"] = (
        '{' + json.dumps(key("Record")) + ':{"obsolete":0},' + escaped_key + ':' + json.dumps(record) + ',' +
        json.dumps(key("Pick")) + ':' + json.dumps(original[key("Pick")]) + ',' + json.dumps(key("Picks")) + ':' + json.dumps(original[key("Picks")]) + '}')
    accepted["duplicate-leaf"] = valid.read_text().replace('"' + key("Direct") + '": 1.0000000000000002',
        '"' + key("Direct") + '": "invalid but replaced", "' + key("Direct") + '": 1.0000000000000002', 1)
    rejected = {}
    for name, field, value in [
        ("quoted-finite", "Direct", "1.5"), ("bad-special", "Direct", "INF"),
        ("case-special", "Direct", "nan"), ("wrong-kind", "Direct", True),
        ("inclusive-lower", "Checked", -1.0000000000000002), ("exclusive-upper", "Checked", 1),
        ("exclusive-lower", "Positive", 0), ("exclusive-negative-zero", "Positive", -0.0),
        ("inclusive-upper", "Positive", 1.0000000000000002), ("bounded-cardinality", "Samples", [0, 1, 2, 3]),
        ("required-unbounded", "NamedSamples", []), ("later-index", "Samples", [0, "1.5"]),
        ("named-later-index", "NamedBounded", [0, 1]), ("unicode", "Text", "\ud800"),
    ]:
        obj = json.loads(valid.read_text())
        obj[key("Record")][key(field)] = value
        rejected[name] = json.dumps(obj)
    obj = json.loads(valid.read_text()); obj["Unknown"] = 0; rejected["unknown-root"] = json.dumps(obj)
    obj = json.loads(valid.read_text()); obj[key("Record")]["Unknown"] = 0; rejected["unknown-record"] = json.dumps(obj)
    obj = json.loads(valid.read_text()); obj[key("Pick")] = {"Unknown": record}; rejected["unknown-choice"] = json.dumps(obj)
    rejected["overflow"] = valid.read_text().replace('1.0000000000000002', '1e9999', 1)
    rejected["invalid-replaced-number"] = accepted["duplicate-leaf"].replace('"invalid but replaced"', '01', 1)
    rejected["invalid-replaced-subtree"] = accepted["subtree-replacement"].replace('{"obsolete":0}', '{"obsolete":01}', 1)
    rejected["byte-limit"] = " " * 1_048_577
    rejected["depth-limit"] = '[' * 129 + '0' + ']' * 129
    rejected["node-limit"] = '[' + ','.join('0' for _ in range(100_001)) + ']'
    paths = {}
    for name, text in (accepted | rejected).items():
        path = scratch / f"{name}.json"
        path.write_text(text + "\n")
        paths[name] = path
    cells = []
    def equivalent(actual, expected):
        if isinstance(expected, dict):
            return actual.keys() == expected.keys() and all(
                struct.pack("!f", actual[k]) == struct.pack("!f", v)
                if k == key("Single") else equivalent(actual[k], v)
                for k, v in expected.items())
        if isinstance(expected, list):
            return len(actual) == len(expected) and all(equivalent(a, b) for a, b in zip(actual, expected, strict=True))
        return actual == expected
    for policy in ["Normal", "Check", "Ignore"]:
        command = ["gprbuild", "-P", "consumer.gpr", "-p", "-f"]
        if policy != "Normal":
            adc = scratch / f"{policy}.adc"; adc.write_text(f"pragma Assertion_Policy ({policy});\n")
            command += ["-cargs:Ada", f"-gnatec={adc}"]
        run(f"{policy}-build", ["sh", "-ec", f". {shlex.quote(str(environment))}; exec " + shlex.join(command)], consumer)
        binary = str(consumer / "bin/conformance")
        for name in accepted:
            output = run(f"{policy}-{name}", [binary, "accept", str(valid), str(paths[name])], consumer)
            lines = output.splitlines()
            if len(lines) != 2 or lines[-1] != "TASK097_CONFORMANCE_ACCEPT_COMPLETE":
                raise RuntimeError("incomplete accepted case")
            tree = json.loads(lines[0])
            if name in ["base", "duplicate-leaf", "subtree-replacement"] and not equivalent(tree, original):
                raise RuntimeError("duplicate/Unicode/typed representation mismatch")
            if name in ["nan", "infinity", "negative-infinity"] and tree[key("Record")][key("Direct")] != json.loads(accepted[name])[key("Record")][key("Direct")]:
                raise RuntimeError("special mapping")
        for name in rejected:
            output = run(f"{policy}-{name}", [binary, "reject", str(valid), str(paths[name])], consumer)
            if not output.startswith("TASK097_REJECT_ATOMIC "):
                raise RuntimeError("missing rejection/atomicity")
            if name == "later-index" and "Samples item 2" not in output:
                raise RuntimeError("wrong later-element index")
        output = run(f"{policy}-encode-invalid", [binary, "encode-invalid", str(valid)], consumer)
        if output != "TASK097_ENCODE_REJECT_ATOMIC\n":
            raise RuntimeError("invalid output not atomic")
        output = run(f"{policy}-allocation", [binary, "allocation", str(valid)], consumer)
        if not output.startswith("TASK097_ALLOCATION_ATOMIC_COMPLETE "):
            raise RuntimeError("missing allocation fault evidence")
        cells.append({"policy": policy, "accepted": len(accepted), "rejected": len(rejected), "allocation_budgets": 101})
        print("COMPLETE", policy, namespace, flush=True)
    files = [p for parent in [consumer, scratch / "support"] for p in parent.rglob("*")
             if p.is_file() and p.suffix in [".ads", ".adb", ".c", ".h", ".gpr"]]
    files += [Path(__file__), schema_path, args.generator.resolve()]
    (scratch / "sources.json").write_text(json.dumps({str(p): hashlib.sha256(p.read_bytes()).hexdigest() for p in files}, indent=2) + "\n")
    temporary = scratch / "result.json.tmp"
    temporary.write_text(json.dumps({"completed": True, "production_generated": True, "namespace": namespace, "cells": cells}, indent=2) + "\n")
    temporary.replace(scratch / "result.json")
    print("TASK097_CONFORMANCE_MATRIX_COMPLETE cells=3", flush=True)


if __name__ == "__main__":
    main()