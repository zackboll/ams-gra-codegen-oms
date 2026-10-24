"""Generated composition/atomicity/facet/locale controls beyond the numerical corpus.

Ordinary resources only. Forced allocator exhaustion is not part of the
qualified production scope; historical C1-C7 allocation evidence is retained
separately and is not rerun or relabeled here.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shlex
import shutil
import struct
import subprocess

ROOT = Path(__file__).resolve().parents[2]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--scratch", type=Path, required=True)
    source = parser.add_mutually_exclusive_group(required=True)
    source.add_argument("--generated", type=Path, help="reuse a completed generated.py tree's provider copy")
    source.add_argument("--setup", type=Path, help="fresh scripts/task096/setup.py provisioning (first smoke gate)")
    parser.add_argument("--namespace", choices=["oam", "qualified"], help="required with --setup")
    parser.add_argument("--generator", type=Path, required=True)
    parser.add_argument("--optimization", choices=["0", "2"], default="0")
    args = parser.parse_args()
    scratch = args.scratch.resolve()
    if args.generated:
        generated = args.generated.resolve()
        prior = json.loads((generated / "result.json").read_text())
        if not prior.get("completed"):
            raise RuntimeError("numerical generated matrix not complete")
        namespace = prior["namespace"]
    else:
        if not args.namespace:
            parser.error("--setup requires --namespace")
        namespace = args.namespace
    scratch.mkdir(parents=True, exist_ok=False)
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

    def run(label, command, cwd=ROOT, env=None):
        prefix = scratch / label
        prefix.with_suffix(".command.json").write_text(json.dumps({"argv": command, "cwd": str(cwd)}, indent=2) + "\n")
        with prefix.with_suffix(".stdout").open("wb") as out, prefix.with_suffix(".stderr").open("wb") as err:
            process = subprocess.run(command, cwd=cwd, env=env, stdout=out, stderr=err)
        prefix.with_suffix(".exit").write_text(str(process.returncode) + "\n")
        if process.returncode:
            raise RuntimeError(f"failed {label}")
        return prefix.with_suffix(".stdout").read_text()

    run("generate", [str(args.generator.resolve()), "service-generate", "--schema", str(schema_path),
        "--contract", str(ROOT / "tests/fixtures/service-generate/codec-ada-float64.yaml"), "--language", "ada",
        "--world", "closed-schema", "--with-codec", "--ada-codec-runtime", "linux-x86_64-glibc-2.41-lossless-v1", "--output", str(consumer)])
    source = (ROOT / "tests/ada-float64-codec/conformance.adb").read_text()
    if namespace == "qualified":
        source = source.replace("Programs.Oam", "Urn.Task097")
    (consumer / "conformance.adb").write_text(source)
    # Test-only per-thread German numeric locale controls (never setlocale).
    shutil.copy2(ROOT / "tests/ada-float64-probe/locale_controls.c", consumer)
    environment = scratch / "environment.sh"
    if args.generated:
        for directory in ["support", "provider"]:
            shutil.copytree(generated / directory, scratch / directory)
        environment.write_text((generated / "environment.sh").read_text().replace(str(generated / "provider"), str(scratch / "provider")))
    else:
        # Exact current support sources and a private provider build tree.
        setup = args.setup.resolve()
        env = dict(os.environ, HOME=str(setup / "home"), XDG_CONFIG_HOME=str(setup / "home/.config"),
                   XDG_CACHE_HOME=str(setup / "home/.cache"), XDG_DATA_HOME=str(setup / "home/.local/share"))
        printed = run("printenv", ["alr", "-s", str(setup / "settings"), "-n", "printenv"], setup / "consumer", env)
        provider = setup / "consumer/alire/cache/dependencies"
        shutil.copytree(provider, scratch / "provider")
        shutil.copytree(ROOT / "ada/oms-json-support", scratch / "support")
        environment.write_text(printed.replace(str(provider), str(scratch / "provider")))
    (consumer / "consumer.gpr").write_text(
        'with "../support/ams_gra_oms_json_lossless";\nproject Consumer is\n'
        'for Languages use ("Ada", "C");\n'
        'for Source_Dirs use ("."); for Object_Dir use "obj"; for Exec_Dir use "bin";\n'
        'for Main use ("conformance.adb");\npackage Compiler is\n'
        'for Default_Switches ("Ada") use ("-gnat2022", "-gnato", "-O' + args.optimization + '");\n'
        'for Default_Switches ("C") use ("-std=c11", "-O' + args.optimization + '"); end Compiler;\nend Consumer;\n')
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

    class Number(str):
        """Explicit JSON-number kind tag; a quoted numeral stays a plain str."""

    def equivalent(actual, expected, width="!d"):
        if isinstance(expected, dict):
            return isinstance(actual, dict) and list(actual) == list(expected) and all(
                equivalent(actual[k], v, "!f" if k == key("Single") else "!d") for k, v in expected.items())
        if isinstance(expected, list):
            return isinstance(actual, list) and len(actual) == len(expected) and all(
                equivalent(a, b, width) for a, b in zip(actual, expected, strict=True))
        if isinstance(expected, bool) or expected is None:
            return actual is expected
        if isinstance(expected, (int, float)):
            # Kind, signed zero and exact width-specific bits, not Python ==.
            return type(actual) is Number and struct.pack(width, float(actual)) == struct.pack(width, float(expected))
        return type(actual) is str and actual == expected
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
            tree = json.loads(lines[0], parse_float=Number, parse_int=Number)
            if name in ["base", "duplicate-leaf", "subtree-replacement"] and not equivalent(tree, original):
                raise RuntimeError("duplicate/Unicode/typed representation mismatch")
            if name in ["nan", "infinity", "negative-infinity"]:
                special = json.loads(accepted[name])[key("Record")][key("Direct")]
                if type(tree[key("Record")][key("Direct")]) is not str or tree[key("Record")][key("Direct")] != special:
                    raise RuntimeError("special mapping")
            if name == "absent" and any(key(n) in tree[key("Record")] for n in ["Optional", "Unbounded", "NamedBounded"]):
                raise RuntimeError("absent optional/repeated emitted")
            # Established repeated-codec convention: an empty minOccurs=0
            # sequence is omitted, never emitted as [] or as a scalar.
            if name == "empty" and any(key(n) in tree[key("Record")] for n in ["Samples", "Unbounded", "NamedBounded"]):
                raise RuntimeError("empty repeated representation")
        for name in rejected:
            output = run(f"{policy}-{name}", [binary, "reject", str(valid), str(paths[name])], consumer)
            if not output.startswith("TASK097_REJECT_ATOMIC "):
                raise RuntimeError("missing rejection/atomicity")
            if name == "later-index" and "Samples item 2" not in output:
                raise RuntimeError("wrong later-element index")
        output = run(f"{policy}-encode-invalid", [binary, "encode-invalid", str(valid)], consumer)
        if output != "TASK097_ENCODE_REJECT_ATOMIC\n":
            raise RuntimeError("invalid output not atomic")
        output = run(f"{policy}-locale", [binary, "locale", str(valid)], consumer)
        if not output.startswith("TASK097_LOCALE_COMPLETE "):
            raise RuntimeError("missing German thread-locale evidence")
        cells.append({"policy": policy, "optimization": args.optimization, "accepted": len(accepted),
                      "rejected": len(rejected), "german_thread_locale": True})
        print("COMPLETE", policy, namespace, flush=True)
    files = [p for parent in [consumer, scratch / "support"] for p in parent.rglob("*")
             if p.is_file() and p.suffix in [".ads", ".adb", ".c", ".h", ".gpr"]]
    files += [Path(__file__), schema_path, args.generator.resolve()]
    (scratch / "sources.json").write_text(json.dumps({str(p): hashlib.sha256(p.read_bytes()).hexdigest() for p in files}, indent=2) + "\n")
    temporary = scratch / "result.json.tmp"
    temporary.write_text(json.dumps({"completed": True, "production_generated": True, "namespace": namespace,
                                     "optimization": args.optimization, "cells": cells}, indent=2) + "\n")
    temporary.replace(scratch / "result.json")
    print("TASK097_CONFORMANCE_MATRIX_COMPLETE cells=3", flush=True)


if __name__ == "__main__":
    main()