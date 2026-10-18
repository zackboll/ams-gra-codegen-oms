"""Production-generated numerical matrix, explicit number-kind/exact-value checks."""
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import shlex
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[2]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--scratch", type=Path, required=True)
    parser.add_argument("--setup", type=Path, required=True)
    parser.add_argument("--generator", type=Path, required=True)
    parser.add_argument("--corpus", type=Path, required=True)
    parser.add_argument("--namespace", choices=["oam", "qualified"], required=True)
    parser.add_argument("--optimization", choices=["0", "2"], default="0")
    args = parser.parse_args()
    scratch, setup, corpus = args.scratch.resolve(), args.setup.resolve(), args.corpus.resolve()
    scratch.mkdir(parents=True, exist_ok=False)
    env = os.environ.copy()
    env.update(HOME=str(setup / "home"), XDG_CONFIG_HOME=str(setup / "home/.config"),
               XDG_CACHE_HOME=str(setup / "home/.cache"), XDG_DATA_HOME=str(setup / "home/.local/share"))

    def run(label, command, cwd=ROOT):
        (scratch / f"{label}.command.json").write_text(json.dumps({"argv": list(map(str, command)), "cwd": str(cwd)}, indent=2) + "\n")
        with (scratch / f"{label}.stdout").open("wb") as out, (scratch / f"{label}.stderr").open("wb") as err:
            result = subprocess.run(command, cwd=cwd, env=env, stdout=out, stderr=err)
        (scratch / f"{label}.exit").write_text(str(result.returncode) + "\n")
        if result.returncode:
            raise RuntimeError(f"failed {label}")
        return (scratch / f"{label}.stdout").read_text()

    manifest = json.loads((ROOT / "tests/ada-float64-probe/corpus-manifest.json").read_text())
    for name, digest in manifest["hashes"].items():
        if hashlib.sha256((corpus / name).read_bytes()).hexdigest() != digest:
            raise RuntimeError("changed frozen corpus")
    spec = importlib.util.spec_from_file_location("oracle", ROOT / "tests/ada-float64-probe/oracle.py")
    oracle = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(oracle)
    schema = ROOT / "tests/fixtures/service-generate/codec-ada-float64.xsd"
    source = (ROOT / "tests/ada-float64-codec/generated_corpus.adb").read_text()
    if args.namespace == "qualified":
        text = schema.read_text().replace('xmlns:uci=', 'xmlns:version="https://www.vdl.afrl.af.mil/programs/oam" xmlns:uci=')
        text = text.replace('xmlns:uci="https://www.vdl.afrl.af.mil/programs/oam"', 'xmlns:uci="urn:task097"')
        text = text.replace('targetNamespace="https://www.vdl.afrl.af.mil/programs/oam"', 'targetNamespace="urn:task097"')
        text = text.replace('uci:version=', 'version:version=')
        schema = scratch / "qualified.xsd"
        schema.write_text(text)
        source = source.replace("Programs.Oam", "Urn.Task097").replace(
            'function Key (Local : String) return String is (Local);',
            'function Key (Local : String) return String is ("{urn:task097}" & Local);')
    consumer = scratch / "consumer"
    run("generation", [str(args.generator.resolve()), "service-generate", "--schema", str(schema),
        "--contract", str(ROOT / "tests/fixtures/service-generate/codec-ada-float64.yaml"),
        "--language", "ada", "--world", "closed-schema", "--with-codec",
        "--ada-codec-runtime", "linux-x86_64-glibc-2.41-lossless-v1", "--output", str(consumer)])
    (consumer / "generated_corpus.adb").write_text(source)
    shutil.copytree(ROOT / "ada/oms-json-support", scratch / "support")
    (consumer / "consumer.gpr").write_text(
        'with "../support/ams_gra_oms_json_lossless";\nproject Consumer is\n'
        'for Source_Dirs use ("."); for Object_Dir use "obj"; for Exec_Dir use "bin";\n'
        'for Main use ("generated_corpus.adb");\n'
        'package Compiler is for Default_Switches ("Ada") use ("-gnat2022", "-gnato", "-O' + args.optimization + '");\n'
        'end Compiler; end Consumer;\n')
    environment = run("printenv", ["alr", "-s", str(setup / "settings"), "-n", "printenv"], setup / "consumer")
    # Never share writable provider build directories among compiler/namespace workers.
    provider = setup / "consumer/alire/cache/dependencies"
    shutil.copytree(provider, scratch / "provider")
    environment = environment.replace(str(provider), str(scratch / "provider"))
    environment_file = scratch / "environment.sh"
    environment_file.write_text(environment)
    patterns = (corpus / "encoding.tsv").read_text().splitlines()

    class Number(str):
        pass

    def key(name):
        return name if args.namespace == "oam" else "{urn:task097}" + name

    cells = []
    for policy in ["Normal", "Check", "Ignore"]:
        command = ["gprbuild", "-P", "consumer.gpr", "-p", "-f"]
        if policy != "Normal":
            adc = scratch / f"{policy}.adc"
            adc.write_text(f"pragma Assertion_Policy ({policy});\n")
            command += ["-cargs:Ada", f"-gnatec={adc}"]
        run(f"{policy}-build", ["sh", "-ec", f". {shlex.quote(str(environment_file))}; exec " + shlex.join(command)], consumer)
        binary = str(consumer / "bin/generated_corpus")
        text = run(f"{policy}-convert", [binary, "convert", str(corpus / "conversions.tsv")], consumer)
        if text != f"TASK097_GENERATED_CORPUS_COMPLETE mode=convert count= {manifest['conversion_rows']}\n":
            raise RuntimeError("incomplete generated conversion corpus")
        output = run(f"{policy}-encode", [binary, "encode", str(corpus / "encoding.tsv")], consumer)
        lines = output.splitlines()
        if len(lines) != len(patterns) + 1 or lines[-1] != f"TASK097_GENERATED_CORPUS_COMPLETE mode=encode count= {manifest['encoding_rows']}":
            raise RuntimeError("incomplete generated encoding corpus")
        for line, expected in zip(lines[:-1], patterns, strict=True):
            pattern, document = line.split("\t", 1)
            if pattern != expected:
                raise RuntimeError("candidate attribution")
            tree = json.loads(document, parse_float=Number, parse_int=Number)
            if list(tree) != [key("Record"), key("Pick"), key("Picks")]:
                raise RuntimeError("document insertion order")
            records = [tree[key("Record")], tree[key("Pick")][key("Nested")], tree[key("Picks")][0][key("Nested")]]
            for record in records:
                if list(record) != [key(n) for n in ["Direct", "Optional", "Named", "Checked", "Data", "Single", "Samples", "NamedSamples"]]:
                    raise RuntimeError("record insertion order")
                tokens = [record[key("Direct")], record[key("Optional")], record[key("Named")], *record[key("Samples")]]
                for token in tokens:
                    if type(token) is not Number or oracle.oracle(token) != expected:
                        raise RuntimeError("finite value not JSON number or wrong exact bits")
        cells.append({"policy": policy, "conversions": manifest["conversion_rows"],
                      "encodings": manifest["encoding_rows"], "finite_positions": 15})
        print("COMPLETE", policy, args.namespace, flush=True)
    paths = [p for parent in [consumer, scratch / "support"] for p in parent.rglob("*")
             if p.is_file() and p.suffix in [".adb", ".ads", ".c", ".h", ".gpr"]]
    paths += [Path(__file__), args.generator.resolve(), schema]
    (scratch / "sources.json").write_text(json.dumps({str(p): hashlib.sha256(p.read_bytes()).hexdigest() for p in paths}, indent=2) + "\n")
    temporary = scratch / "result.json.tmp"
    temporary.write_text(json.dumps({"completed": True, "production_generated": True, "namespace": args.namespace,
                                    "optimization": args.optimization, "cells": cells}, indent=2) + "\n")
    temporary.replace(scratch / "result.json")
    print("TASK097_GENERATED_MATRIX_COMPLETE cells=3", flush=True)


if __name__ == "__main__":
    main()