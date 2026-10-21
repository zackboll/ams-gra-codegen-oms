"""C3 production-generated consumer matrix; each attempt owns all writable trees."""
import argparse
import copy
import hashlib
import json
import os
from pathlib import Path
import shlex
import shutil
import struct
import subprocess

ROOT = Path(__file__).resolve().parents[2]


def atomic(path, data):
    temporary = path.with_suffix(path.suffix + ".tmp")
    temporary.write_text(json.dumps(data, indent=2, sort_keys=True) + "\n")
    temporary.replace(path)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--scratch", type=Path, required=True)
    parser.add_argument("--generator", type=Path, required=True)
    parser.add_argument("--compiler", type=int, choices=[14, 16], required=True)
    parser.add_argument("--namespace", choices=["oam", "qualified"], required=True)
    parser.add_argument("--policies", nargs="+", default=["Normal", "Check", "Ignore"])
    parser.add_argument("--optimizations", nargs="+", choices=["0", "2"], default=["0", "2"])
    parser.add_argument("--setup", type=Path, help="fresh scripts/task096/setup.py provisioning (production gate)")
    args = parser.parse_args()
    scratch = args.scratch.resolve()
    scratch.mkdir(parents=True, exist_ok=False)
    rows = []

    def run(label, command, cwd=scratch, expected=0):
        atomic(scratch / (label + ".command.json"), {"argv": list(map(str, command)), "cwd": str(cwd)})
        with (scratch / (label + ".stdout")).open("wb") as out, (scratch / (label + ".stderr")).open("wb") as err:
            result = subprocess.run(command, cwd=cwd, stdout=out, stderr=err)
        (scratch / (label + ".exit")).write_text(str(result.returncode) + "\n")
        rows.append({"label": label, "exit": result.returncode, "expected_exit": expected})
        if result.returncode != expected:
            raise RuntimeError(f"{label}: exit {result.returncode}")
        return (scratch / (label + ".stdout")).read_text()

    ignore = shutil.ignore_patterns("obj", "obj-*", "lib", "bin", "*.o", "*.ali", "*.a", "*.so", "*.lexch", "*.bexch")
    environment = scratch / "environment.sh"
    if args.setup:
        # Production qualification: exact current support sources plus a
        # private copy of the freshly provisioned task-owned provider.
        setup = args.setup.resolve()
        env = dict(os.environ, HOME=str(setup / "home"), XDG_CONFIG_HOME=str(setup / "home/.config"),
                   XDG_CACHE_HOME=str(setup / "home/.cache"), XDG_DATA_HOME=str(setup / "home/.local/share"))
        printed = subprocess.run(["alr", "-s", str(setup / "settings"), "-n", "printenv"], cwd=setup / "consumer",
                                 env=env, check=True, capture_output=True, text=True).stdout
        provider = setup / "consumer/alire/cache/dependencies"
        shutil.copytree(provider, scratch / "provider", ignore=ignore)
        shutil.copytree(ROOT / "ada/oms-json-support", scratch / "support", ignore=ignore)
        environment.write_text(printed.replace(str(provider), str(scratch / "provider")))
    else:
        # Historical C3 reproduction only (pristine C1 provider/support copies).
        c1 = Path(f"/home/zboll/task097-scratch/c1/pristine{args.compiler}-01")
        for directory in ["support", "provider"]:
            shutil.copytree(c1 / directory, scratch / directory, ignore=ignore)
        environment.write_text((c1 / "environment.sh").read_text().replace(str(c1 / "provider"), str(scratch / "provider")))
    shell = lambda command: ["sh", "-ec", f". {shlex.quote(str(environment))}; exec " + shlex.join(command)]
    run("compiler", shell(["gnatls", "-v"]))
    namespace = "https://www.vdl.afrl.af.mil/programs/oam"
    package = "Programs.Oam"
    schema = (ROOT / "tests/fixtures/service-generate/codec-ada-copy-views.xsd").read_text()
    if args.namespace == "qualified":
        schema = schema.replace('xmlns:uci=', 'xmlns:version="' + namespace + '" xmlns:uci=')
        schema = schema.replace(namespace, "urn:copyproof").replace('xmlns:version="urn:copyproof"', 'xmlns:version="' + namespace + '"').replace('uci:version=', 'version:version=')
        namespace, package = "urn:copyproof", "Urn.Copyproof"
    schema_path = scratch / "schema.xsd"
    schema_path.write_text(schema)
    generated = scratch / "generated"
    run("generate", [str(args.generator.resolve()), "service-generate", "--schema", str(schema_path),
        "--contract", str(ROOT / "tests/fixtures/service-generate/codec-ada-float64.yaml"), "--language", "ada",
        "--world", "closed-schema", "--with-codec", "--ada-codec-runtime", "linux-x86_64-glibc-2.41-lossless-v1", "--output", str(generated)])
    for name in ["c3_consumer.adb", "c3_views.adb", "c1_malloc_fault.c"]:
        text = (ROOT / "tests/ada-float64-codec" / name).read_text().replace("Programs.Oam", package)
        (generated / name).write_text(text)
    for ext in ["ads", "adb"]:
        shutil.copy2(ROOT / f"tests/ada-float64-codec/c1_null_allocation_faults.{ext}", generated / f"allocation_faults.{ext}")

    def qualified(value):
        if isinstance(value, dict):
            return {(k if args.namespace == "oam" else "{" + namespace + "}" + k): qualified(v) for k, v in value.items()}
        if isinstance(value, list):
            return [qualified(v) for v in value]
        return value

    leaf = {"Data": "80FF", "Digest": "80FF", "OptionalDigest": "01", "Digests": ["80FF", "01"], "Exact": "01FE", "Empty": ""}
    reading = {"Inherited": -0.0, "Direct": 1.0000000000000002, "Single": 1.25,
               "Text": "C3 inline/heap " + "é 🛰 " * 12, "Nested": leaf,
               "Dates": ["2026-10-18T00:00:00Z", "2024-02-29T00:00:00Z"], "Spans": ["PT1.25S", "-P2D"]}
    second = copy.deepcopy(reading)
    second.update(Direct=5e-324, Text="tiny")
    payload = {"Record": reading, "Pick": {"Nested": second}, "Picks": [{"Nested": reading}, {"Enabled": True}],
               "UB0": [reading, second], "UB1": [second, reading], "B0": [second, reading], "B1": [reading, second]}
    valid = scratch / "valid.json"
    valid.write_text(json.dumps(qualified(payload), ensure_ascii=False) + "\n")
    minimum = copy.deepcopy(payload)
    minimum.pop("UB0"); minimum.pop("B0")
    minimum["UB1"] = minimum["UB1"][:1]; minimum["B1"] = minimum["B1"][:1]; minimum["Picks"] = minimum["Picks"][:1]
    for r in [minimum["Record"], minimum["Pick"]["Nested"], minimum["Picks"][0]["Nested"], *minimum["UB1"], *minimum["B1"]]:
        r["Nested"].pop("OptionalDigest", None); r["Nested"].pop("Digests", None)
    (scratch / "minimum.json").write_text(json.dumps(qualified(minimum)) + "\n")
    invalid = []
    for name, change in [
        ("bad-digest", lambda v: v["Record"]["Nested"].update(Digest="000102")),
        ("bad-empty", lambda v: v["Record"]["Nested"].update(Empty="01")),
        ("bad-exact", lambda v: v["Record"]["Nested"].update(Exact="00")),
        ("bad-binary", lambda v: v["Record"]["Nested"].update(Data="GG")),
        ("bad-repeated", lambda v: v["Record"]["Nested"].update(Digests=["01", ""])),
        ("unknown", lambda v: v["Record"].update(Unknown=True)),
        ("bad-number", lambda v: v["Record"].update(Direct="1.0")),
        ("missing-required", lambda v: v.update(UB1=[])),
    ]:
        value = copy.deepcopy(payload); change(value)
        (scratch / (name + ".json")).write_text(json.dumps(qualified(value)) + "\n"); invalid.append(name)
    (scratch / "syntax.json").write_text('{"broken": [}\n'); invalid.append("syntax")
    # Duplicate leaf retains last value, including qualified wire keys.
    duplicate = valid.read_text()
    key = json.dumps("Digest" if args.namespace == "oam" else "{" + namespace + "}Digest")
    duplicate = duplicate.replace(key + ': "80FF"', key + ': "01", ' + key + ': "80FF"', 1)
    (scratch / "duplicate.json").write_text(duplicate)

    def exact(actual, expected):
        if isinstance(expected, dict):
            return actual.keys() == expected.keys() and all(exact(actual[k], v) for k, v in expected.items())
        if isinstance(expected, list):
            return len(actual) == len(expected) and all(exact(a, b) for a, b in zip(actual, expected, strict=True))
        if isinstance(expected, bool):
            return actual is expected
        if isinstance(expected, (int, float)):
            return struct.pack("!d", float(actual)) == struct.pack("!d", float(expected))
        return actual == expected

    for policy in args.policies:
        for optimization in args.optimizations:
            label = f"{policy}-O{optimization}"
            consumer = scratch / label
            shutil.copytree(generated, consumer)
            adc = consumer / "policy.adc"
            adc.write_text(f"pragma Assertion_Policy ({policy});\n" if policy != "Normal" else "-- Normal\n")
            # Explicit source lists demonstrate that both child units are included.
            sources = sorted(p.name for p in consumer.iterdir() if p.suffix in [".ads", ".adb", ".c"])
            (consumer / "consumer.gpr").write_text('with "../support/ams_gra_oms_json_lossless";\nproject Consumer is\n'
                'for Languages use ("Ada", "C"); for Source_Dirs use ("."); for Source_Files use (' + ', '.join('"' + n + '"' for n in sources) + ');\n'
                'for Object_Dir use "obj"; for Exec_Dir use "bin"; for Main use ("c3_consumer.adb", "c3_views.adb");\n'
                'package Compiler is for Default_Switches ("Ada") use ("-gnat2022", "-gnato", "-g", "-O' + optimization + '"); for Default_Switches ("C") use ("-g", "-O' + optimization + '"); end Compiler;\n'
                'package Linker is for Default_Switches ("Ada") use ("-Wl,--wrap=__gnat_malloc"); end Linker;\nend Consumer;\n')
            run(label + "-build", shell(["gprbuild", "-P", "consumer.gpr", "-p", "-j2", "-cargs:Ada", f"-gnatec={adc}", "-g", f"-O{optimization}"]), consumer)
            output = run(label + "-views", [str(consumer / "bin/c3_views")], consumer)
            if not output.startswith("C3_BINARY_VIEWS_COMPLETE checks="): raise RuntimeError("view marker")
            binary = str(consumer / "bin/c3_consumer")
            for case in ["valid", "minimum", "duplicate"]:
                output = run(label + "-" + case, [binary, "accept", str(scratch / (case + ".json"))], consumer)
                lines = output.splitlines()
                expected = qualified(minimum if case == "minimum" else payload)
                if len(lines) != 2 or lines[1] != "C3_TYPED_ROUNDTRIP_COMPLETE" or not exact(json.loads(lines[0], parse_int=float), expected):
                    raise RuntimeError("typed exact/order mismatch " + label + " " + case)
            for case in invalid:
                output = run(label + "-" + case, [binary, "reject", str(valid), str(scratch / (case + ".json"))], consumer)
                if output != "C3_REJECT_ATOMIC_COMPLETE\n": raise RuntimeError("atomic marker")
            output = run(label + "-encode-baseline", [binary, "encode", str(valid), "1000000"], consumer)
            if "C3_ENCODE_SUCCESS" not in output or "failures= 0" not in output: raise RuntimeError("baseline incomplete")
            output = run(label + "-encode-first", [binary, "encode", str(valid), "0"], consumer)
            if "EXCEPTION STORAGE_ERROR" not in output or "failures= 1" not in output or "RECOVERY_COMPLETE" not in output: raise RuntimeError("injected result")
            print("C3_COMPOSITION_CELL_COMPLETE", args.compiler, args.namespace, label, flush=True)
    files = [p for p in scratch.rglob("*") if p.is_file() and p.suffix in [".ads", ".adb", ".c", ".gpr", ".adc", ".xsd", ".json", ".sh"] and "obj" not in p.parts]
    files += [Path(__file__), args.generator.resolve()]
    atomic(scratch / "sources.json", {str(p): hashlib.sha256(p.read_bytes()).hexdigest() for p in files})
    atomic(scratch / "result.json", {"completed": True, "compiler": args.compiler, "namespace": args.namespace, "rows": rows})
    print("C3_COMPOSITION_MATRIX_COMPLETE", flush=True)


if __name__ == "__main__":
    main()