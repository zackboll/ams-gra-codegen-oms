#!/usr/bin/env python3
"""Task096 numerical measurement, not production or full-document certification.

First provision with scripts/task096/setup.py. Pass that fresh setup directory
as --setup. No concurrent jobs may write to the same setup consumer.
"""
import argparse
import collections
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
PROBES = ROOT / "tests/ada-float64-probe"


def load_oracle():
    spec = importlib.util.spec_from_file_location("task096_oracle", PROBES / "oracle.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def measure_conversion(path, expected_count):
    counts = collections.Counter()
    failures = collections.Counter()
    examples = {}
    for line in path.read_text().splitlines()[:-1]:
        candidate, expected, actual, kind, detail, token = line.split("\t")
        counts[candidate] += 1
        accepted = actual == expected or (expected == "OVERFLOW" and actual in ["REJECT", "EXCEPTION", "OVERFLOW"])
        if candidate == "strtod_l":
            status, consumed, error, result_bits = detail.split(":")
            if expected == "OVERFLOW":
                accepted = actual == "REJECT" and result_bits == "4045000000000000"
            else:
                accepted = accepted and int(status) == 0 and int(consumed) == len(token) and int(error) in [0, 34]
        if not accepted:
            failures[candidate] += 1
            examples.setdefault(candidate, {"expected": expected, "actual": actual,
                                           "token": token, "kind": kind, "detail": detail})
    if counts != {"strtod_l": expected_count, "IEEE_Value": expected_count, "DOM": expected_count}:
        raise RuntimeError(f"conversion accounting mismatch: {counts}")
    if path.read_text().splitlines()[-1] != f"TASK096_NUMERIC_COMPLETE mode=convert count= {expected_count}":
        raise RuntimeError("missing conversion completion marker")
    return {"counts": dict(counts), "failures": dict(failures), "first_failures": examples}


def measure_encoding(path, patterns, oracle):
    counts, failures = collections.Counter(), collections.Counter()
    examples = {}
    seen = collections.defaultdict(set)
    for line in path.read_text().splitlines()[:-1]:
        candidate, expected, token = line.split("\t")
        if expected not in patterns or expected in seen[candidate]:
            raise RuntimeError("unknown or duplicate encoding row")
        seen[candidate].add(expected)
        counts[candidate] += 1
        try:
            actual = oracle.oracle(token)
        except ValueError:
            actual = "INVALID_JSON_NUMBER"
        if actual != expected:
            failures[candidate] += 1
            examples.setdefault(candidate, {"expected": expected, "actual": actual, "token": token})
    if counts != {"writer": len(patterns), "format17": len(patterns)}:
        raise RuntimeError(f"encoding accounting mismatch: {counts}")
    if path.read_text().splitlines()[-1] != f"TASK096_NUMERIC_COMPLETE mode=encode count= {len(patterns)}":
        raise RuntimeError("missing encoding completion marker")
    return {"counts": dict(counts), "failures": dict(failures), "first_failures": examples}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--scratch", type=Path, required=True)
    parser.add_argument("--setup", type=Path, required=True)
    parser.add_argument("--corpus", type=Path, required=True)
    args = parser.parse_args()
    scratch, setup, corpus = args.scratch.resolve(), args.setup.resolve(), args.corpus.resolve()
    scratch.mkdir(parents=True, exist_ok=False)
    consumer = setup / "consumer"
    env = os.environ.copy()
    env.update(HOME=str(setup / "home"), XDG_CONFIG_HOME=str(setup / "home/.config"),
               XDG_CACHE_HOME=str(setup / "home/.cache"), XDG_DATA_HOME=str(setup / "home/.local/share"))
    alr = ["alr", "-s", str(setup / "settings"), "-n"]

    def checked(command, label):
        (scratch / f"{label}.command.json").write_text(json.dumps({"argv": command, "cwd": str(consumer)}, indent=2) + "\n")
        with (scratch / f"{label}.stdout").open("wb") as out, (scratch / f"{label}.stderr").open("wb") as err:
            result = subprocess.run(command, cwd=consumer, env=env, stdout=out, stderr=err)
        temporary = scratch / f"{label}.exit.tmp"
        temporary.write_text(str(result.returncode) + "\n")
        temporary.replace(scratch / f"{label}.exit")
        if result.returncode:
            raise RuntimeError(f"failed {label}")

    oracle = load_oracle()
    manifest = json.loads((corpus / "manifest.json").read_text())
    for name, expected in manifest["hashes"].items():
        if hashlib.sha256((corpus / name).read_bytes()).hexdigest() != expected:
            raise RuntimeError("changed frozen corpus")
    conversions = (corpus / "conversions.tsv").read_text().splitlines()
    if len(conversions) != manifest["conversion_rows"] or len({r.split("\t", 2)[2] for r in conversions}) != len(conversions):
        raise RuntimeError("conversion corpus count/uniqueness")
    for row in conversions:
        _, expected, token = row.split("\t", 2)
        if oracle.oracle(token) != expected:
            raise RuntimeError("oracle disagreement")
    patterns = set((corpus / "encoding.tsv").read_text().splitlines())
    if len(patterns) != manifest["encoding_rows"]:
        raise RuntimeError("encoding corpus count")
    for name in ["numeric_probe.adb", "boundary.c", "locale_controls.c"]:
        shutil.copyfile(PROBES / name, consumer / name)
    # Preserve the setup project as evidence before this isolated consumer changes.
    shutil.copyfile(consumer / "probe.gpr", scratch / "setup-probe.gpr")
    (consumer / "probe.gpr").write_text(
        'with "gnatcoll_core";\nproject Probe is\n'
        'for Languages use ("Ada", "C"); for Source_Dirs use (".");\n'
        'for Object_Dir use "obj"; for Exec_Dir use "bin"; for Main use ("numeric_probe.adb");\n'
        'package Compiler is for Default_Switches ("Ada") use ("-gnat2022", "-gnato", "-O0");\n'
        'for Default_Switches ("C") use ("-std=c11", "-O2", "-Wall", "-Wextra", "-Werror"); end Compiler;\n'
        'package Linker is for Default_Switches ("Ada") use ("-lm"); end Linker; end Probe;\n')
    # Minimal writer gate precedes the full encoding campaign.
    minimal = scratch / "minimal-encoding.tsv"
    minimal.write_text("3FF0000000000000\n3FF0000000000001\n3FF0000000000002\n")
    results = []
    for policy in ["Normal", "Check", "Ignore"]:
        command = alr + ["build", "--", "-f"]
        if policy != "Normal":
            adc = scratch / f"{policy}.adc"
            adc.write_text(f"pragma Assertion_Policy ({policy});\n")
            command += ["-cargs:Ada", f"-gnatec={adc}"]
        checked(command, f"{policy}-build")
        binary = str(consumer / "bin/numeric_probe")
        checked([binary, "encode", str(minimal)], f"{policy}-minimal")
        result = {"policy": policy}
        for mode, filename in [("convert", "conversions.tsv"), ("encode", "encoding.tsv")]:
            print("START", policy, mode, flush=True)
            label = f"{policy}-{mode}"
            checked([binary, mode, str(corpus / filename)], label)
            output = scratch / f"{label}.stdout"
            result[mode] = measure_conversion(output, len(conversions)) if mode == "convert" else measure_encoding(output, patterns, oracle)
            (scratch / f"{label}.result.json").write_text(json.dumps(result[mode], indent=2) + "\n")
            print("COMPLETE", policy, mode, result[mode]["failures"], flush=True)
            locale_label = label + "-german"
            checked([binary, mode, str(corpus / filename), "german"], locale_label)
            locale_output = scratch / f"{locale_label}.stdout"
            locale_result = measure_conversion(locale_output, len(conversions)) if mode == "convert" else measure_encoding(locale_output, patterns, oracle)
            result[mode + "_german"] = locale_result
            if locale_result != result[mode]:
                raise RuntimeError("ambient locale changed numerical results")
        results.append(result)
    sources = [*PROBES.glob("*.adb"), *PROBES.glob("*.c"), *PROBES.glob("*.py"), Path(__file__),
               consumer / "probe.gpr", consumer / "alire/alire.lock", consumer / "bin/numeric_probe"]
    (scratch / "sources.json").write_text(json.dumps({str(p): hashlib.sha256(p.read_bytes()).hexdigest() for p in sources}, indent=2) + "\n")
    temporary = scratch / "result.json.tmp"
    temporary.write_text(json.dumps({"completed_measurement": True, "full_boundary_qualified": False,
                                    "corpus": manifest, "cells": results}, indent=2) + "\n")
    temporary.replace(scratch / "result.json")
    print("TASK096_NUMERICAL_MEASUREMENT_COMPLETE policies=3", flush=True)


if __name__ == "__main__":
    main()