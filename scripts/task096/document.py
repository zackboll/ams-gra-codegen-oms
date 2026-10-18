"""Build/measure the handwritten shape-specific full-document research adapter."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shlex
import shutil
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
from importlib import util


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--scratch", type=Path, required=True)
    parser.add_argument("--setup", type=Path, required=True)
    parser.add_argument("--model", type=Path, required=True)
    parser.add_argument("--profile", choices=["oam", "qualified"], required=True)
    parser.add_argument("--corpus", type=Path, required=True)
    args = parser.parse_args()
    scratch, setup = args.scratch.resolve(), args.setup.resolve()
    scratch.mkdir(parents=True, exist_ok=False)
    consumer = scratch / "consumer"
    consumer.mkdir()
    probes = ROOT / "tests/ada-float64-probe"
    spec = util.spec_from_file_location("oracle", probes / "oracle.py")
    oracle = util.module_from_spec(spec)
    spec.loader.exec_module(oracle)
    for source in (args.model.resolve() / "consumer").glob("*.ad?"):
        if source.name != "model_consumer.adb":
            shutil.copyfile(source, consumer / source.name)
    for name in ["document_probe.adb", "task096_adapter.ads", "task096_adapter.adb",
                 "task096_boundary.ads", "task096_boundary.adb", "boundary.c", "locale_controls.c",
                 "ams_gra_oms_json_occurrences-research.ads", "ams_gra_oms_json_occurrences-research.adb"]:
        text = (probes / name).read_text()
        if args.profile == "qualified":
            text = text.replace("Programs.Oam", "Urn.Task096")
            text = text.replace("function Key (Local : String) return String is (Local);",
                                'function Key (Local : String) return String is ("{urn:task096}" & Local);')
        (consumer / name).write_text(text)
    support = scratch / "support"
    shutil.copytree(ROOT / "ada/oms-json-support", support)
    # The build uses the task-owned, already verified provider installation.
    # No writable support or consumer build directory is shared with a sibling.
    (consumer / "consumer.gpr").write_text(
        'with "../support/ams_gra_oms_json_float32";\nproject Consumer is\n'
        'for Languages use ("Ada", "C"); for Source_Dirs use (".");\n'
        'for Object_Dir use "obj"; for Exec_Dir use "bin"; for Main use ("document_probe.adb");\n'
        'package Compiler is for Default_Switches ("Ada") use ("-gnat2022", "-gnato", "-O0");\n'
        'for Default_Switches ("C") use ("-std=c11", "-O2", "-Wall", "-Wextra", "-Werror"); end Compiler;\n'
        'package Linker is for Default_Switches ("Ada") use ("-lm"); end Linker; end Consumer;\n')
    env = os.environ.copy()
    env.update(HOME=str(setup / "home"), XDG_CONFIG_HOME=str(setup / "home/.config"),
               XDG_CACHE_HOME=str(setup / "home/.cache"), XDG_DATA_HOME=str(setup / "home/.local/share"))
    alr = ["alr", "-s", str(setup / "settings"), "-n"]

    def run(command, cwd, label, extra_env=None):
        (scratch / f"{label}.command.json").write_text(json.dumps({"argv": command, "cwd": str(cwd)}, indent=2) + "\n")
        with (scratch / f"{label}.stdout").open("wb") as out, (scratch / f"{label}.stderr").open("wb") as err:
            result = subprocess.run(command, cwd=cwd, env=env | (extra_env or {}), stdout=out, stderr=err)
        (scratch / f"{label}.exit").write_text(str(result.returncode) + "\n")
        if result.returncode:
            raise RuntimeError(f"failed {label}")
        return (scratch / f"{label}.stdout").read_text()

    environment = scratch / "environment.sh"
    provider = scratch / "provider"
    shutil.copytree(setup / "consumer/alire", provider / "alire")
    environment.write_text(run(alr + ["printenv"], setup / "consumer", "printenv").replace(
        str(setup / "consumer"), str(provider)))
    key = (lambda name: "{urn:task096}" + name) if args.profile == "qualified" else (lambda name: name)
    # JSON numeric placeholders exist only in fixture construction, never output.
    # Use exact values via parse_float=str/parse_int=str to avoid a host oracle.
    base = {
        key("Required"): "__required__", key("Optional"): "__zero__", key("Named"): "__max__",
        key("Closed"): -1, key("Open"): 1,
        key("Bounded"): ["__adjacent__", "__subnormal__", "__zero__"],
        key("Unbounded"): [1, 2, "__minnormal__"], key("NamedRepeated"): [-1, 0, 1],
        key("Nested"): {key("Value"): "__decimal__"},
        key("Choice"): {key("Record"): {key("Value"): "__adjacent__"}},
        key("Small"): 1.5, key("Count"): 42, key("Enabled"): True,
        key("Text"): 'quoted " \\ é 😀 literal __required__',
    }
    replacements = {"__required__": "1.0000000000000002", "__zero__": "-0",
                    "__max__": oracle.decimal(oracle.exact(oracle.MAX_FINITE)),
                    "__adjacent__": "1.0000000000000004", "__subnormal__": oracle.decimal(oracle.exact(1)),
                    "__minnormal__": oracle.decimal(oracle.exact(1 << 52)), "__decimal__": "1.2345678901234567"}
    original = json.dumps(base, ensure_ascii=True, separators=(",", ":"))
    for name, value in replacements.items():
        original = original.replace(json.dumps(name), value)
    fixtures = {"base": original}
    # Reverse member order without ever numerically parsing the JSON.
    # Construct reversed original top-level members from known original fragments.
    fragments = []
    for name in reversed(list(base)):
        value = json.dumps(base[name], ensure_ascii=True, separators=(",", ":"))
        for marker, number in replacements.items():
            value = value.replace(json.dumps(marker), number)
        fragments.append(json.dumps(name) + ":" + value)
    fixtures["reversed"] = "{" + ",".join(fragments) + "}"
    required = json.dumps(key("Required"))
    escaped_required = required.replace("Required", "Requ\\u0069red")
    fixtures["escaped"] = original.replace(required, escaped_required)
    fixtures["duplicate"] = "{" + required + ":0," + original[1:].replace(required, escaped_required, 1)
    nested = json.dumps(key("Nested"))
    fixtures["subtree-replacement"] = "{" + nested + ':{"obsolete":5},' + original[1:]
    fixtures["optional-absent"] = original.replace(json.dumps(key("Optional")) + ":-0,", "")
    fixtures["optional-string-special"] = original.replace(json.dumps(key("Optional")) + ":-0", json.dumps(key("Optional")) + ':"Infinity"')
    rejected = {
        "overflow": original.replace("1.0000000000000002", "1e9999", 1),
        "quoted-number": original.replace("1.0000000000000002", '"1.0"', 1),
        "hex": original.replace("1.0000000000000002", "0x1p0", 1),
        "leading-plus": original.replace("1.0000000000000002", "+1", 1),
        "malformed": original[:-1],
        "range": original.replace(json.dumps(key("Open")) + ":1", json.dumps(key("Open")) + ":0"),
        "bounded-index": original.replace("1.0000000000000004", '"bad"', 1),
        "bad-unicode": original.replace('quoted', '\\uD800'),
        "wrong-boolean": original.replace("true", "1", 1),
        "missing-required": original.replace(required + ":1.0000000000000002,", "", 1),
        "leading-zero": original.replace("1.0000000000000002", "01", 1),
        "trailing-value": original + " true",
        "special-case": original.replace("1.0000000000000002", '"infinity"', 1),
        "subtree-loss": "{" + original[1:-1] + "," + nested + ':{}' + "}",
        "array-index2": original.replace(replacements["__subnormal__"], '"bad"', 1),
        "depth-limit": '[' * 129 + '0' + ']' * 129,
        "size-limit": ' ' * 1_048_577,
    }
    paths = {}
    for name, text in (fixtures | rejected).items():
        path = scratch / f"{name}.json"
        path.write_text(text)
        paths[name] = path

    def verify(source, output):
        want = json.loads(source, parse_int=str, parse_float=str)
        got = json.loads(output, parse_int=str, parse_float=str)

        def walk(a, b, path=()):
            if isinstance(a, dict):
                if a.keys() != b.keys():
                    raise RuntimeError(f"keys differ: {path}")
                for k in a:
                    walk(a[k], b[k], (*path, k))
            elif isinstance(a, list):
                if len(a) != len(b):
                    raise RuntimeError("array length differs")
                for i, (x, y) in enumerate(zip(a, b, strict=True)):
                    walk(x, y, (*path, i))
            elif path[-1] in [key("Text"), key("Enabled")] or a in ["NaN", "Infinity", "-Infinity"]:
                if a != b:
                    raise RuntimeError("nonfloat changed")
            elif oracle.oracle(a) != oracle.oracle(b):
                raise RuntimeError(f"numeric bits differ at {path}")
        walk(want, got)
    cells = []
    for policy in ["Normal", "Check", "Ignore"]:
        command = ["gprbuild", "-P", "consumer.gpr", "-p", "-f"]
        if policy != "Normal":
            adc = scratch / f"{policy}.adc"
            adc.write_text(f"pragma Assertion_Policy ({policy});\n")
            command += ["-cargs:Ada", f"-gnatec={adc}"]
        run(["sh", "-ec", f". {shlex.quote(str(environment))}; exec " + shlex.join(command)], consumer, f"{policy}-build")
        binary = str(consumer / "bin/document_probe")
        for name, source in fixtures.items():
            text = run([binary, "accept", str(paths[name])], consumer, f"{policy}-{name}")
            lines = text.splitlines()
            if len(lines) != 2 or lines[-1] != "TASK096_DOCUMENT_COMPLETE":
                raise RuntimeError("missing document marker")
            verify(source, lines[0])
        for name in rejected:
            text = run([binary, "reject", str(paths[name])], consumer, f"{policy}-{name}")
            if not text.startswith("TASK096_REJECT_ATOMIC"):
                raise RuntimeError("missing atomic rejection")
            if name == "bounded-index" and "Bounded item 1" not in text:
                raise RuntimeError("incorrect array index attribution")
            if name == "array-index2" and "Bounded item 2" not in text:
                raise RuntimeError("incorrect second array index attribution")
        text = run([binary, "encode-invalid", "unused"], consumer, f"{policy}-encode-invalid")
        if text != "TASK096_ENCODE_REJECT_ATOMIC\n":
            raise RuntimeError("missing encode atomic rejection")
        text = run([binary, "specials", "unused"], consumer, f"{policy}-specials")
        for line, special in zip(text.splitlines()[:3], ["NaN", "Infinity", "-Infinity"], strict=True):
            if json.loads(line)[key("Required")] != special:
                raise RuntimeError("special output semantics")
        if text.splitlines()[-1] != "TASK096_SPECIAL_DOCUMENTS_COMPLETE":
            raise RuntimeError("special document marker")
        corpus = args.corpus.resolve()
        manifest = json.loads((corpus / "manifest.json").read_text())
        encoding = corpus / "encoding.tsv"
        if hashlib.sha256(encoding.read_bytes()).hexdigest() != manifest["hashes"]["encoding.tsv"]:
            raise RuntimeError("changed encoding corpus")
        text = run([binary, "encode-corpus", str(encoding)], consumer, f"{policy}-encode-corpus")
        lines = text.splitlines()
        patterns = encoding.read_text().splitlines()
        if len(lines) != len(patterns) + 1 or lines[-1] != "TASK096_DOCUMENT_ENCODING_COMPLETE":
            raise RuntimeError("incomplete full-document encoding corpus")
        for line, expected in zip(lines[:-1], patterns, strict=True):
            pattern, document = line.split("\t", 1)
            if pattern != expected:
                raise RuntimeError("encoding attribution")
            tree = json.loads(document, parse_int=str, parse_float=str)
            numbers = [tree[key("Required")], tree[key("Optional")], tree[key("Named")],
                       tree[key("Nested")][key("Value")], tree[key("Choice")][key("Record")][key("Value")],
                       tree[key("Bounded")][0], *tree[key("Unbounded")]]
            if any(oracle.oracle(token) != expected for token in numbers):
                raise RuntimeError("full output path lost binary64 bits")
        locale_output = run([binary, "locale", str(paths["base"])], consumer, f"{policy}-locale")
        if locale_output.splitlines()[-1] != "TASK096_DOCUMENT_COMPLETE":
            raise RuntimeError("locale document incomplete")
        verify(original, locale_output.splitlines()[0])
        cells.append({"policy": policy, "accepted": len(fixtures), "atomic_rejections": len(rejected),
                      "full_document_encoding_patterns": len(patterns), "numeric_positions_per_pattern": 8})
        print("COMPLETE", args.profile, policy, flush=True)
    sources = [p for p in consumer.iterdir() if p.is_file()]
    sources += [Path(__file__), *support.rglob("*.adb"), *support.rglob("*.ads"), *support.rglob("*.c")]
    (scratch / "sources.json").write_text(json.dumps({str(p): hashlib.sha256(p.read_bytes()).hexdigest() for p in sources}, indent=2) + "\n")
    (scratch / "result.json").write_text(json.dumps({"handwritten_adapter": True, "generated_model": True,
                                                   "profile": args.profile, "cells": cells}, indent=2) + "\n")
    print("TASK096_DOCUMENT_MEASUREMENT_COMPLETE", flush=True)


if __name__ == "__main__":
    main()