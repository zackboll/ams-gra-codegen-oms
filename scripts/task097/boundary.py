"""Task097 production C boundary corpora and lifecycle, independent oracle."""
import argparse
import hashlib
import importlib.util
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[2]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--scratch", type=Path, required=True)
    parser.add_argument("--corpus", type=Path, required=True)
    args = parser.parse_args()
    scratch, corpus = args.scratch.resolve(), args.corpus.resolve()
    scratch.mkdir(parents=True, exist_ok=False)
    spec = importlib.util.spec_from_file_location("oracle", ROOT / "tests/ada-float64-probe/oracle.py")
    oracle = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(oracle)
    manifest = json.loads((ROOT / "tests/ada-float64-probe/corpus-manifest.json").read_text())
    for name, digest in manifest["hashes"].items():
        if hashlib.sha256((corpus / name).read_bytes()).hexdigest() != digest:
            raise RuntimeError("changed frozen corpus")

    def run(label, command):
        (scratch / f"{label}.command.json").write_text(json.dumps({"argv": list(map(str, command)), "cwd": str(ROOT)}, indent=2) + "\n")
        with (scratch / f"{label}.stdout").open("wb") as out, (scratch / f"{label}.stderr").open("wb") as err:
            result = subprocess.run(command, cwd=ROOT, stdout=out, stderr=err)
        (scratch / f"{label}.exit").write_text(str(result.returncode) + "\n")
        if result.returncode:
            raise RuntimeError(f"failed {label}")
        return (scratch / f"{label}.stdout").read_text()

    support = ROOT / "ada/oms-json-support/lossless"
    tests = ROOT / "tests/ada-float64-codec"
    cells = []
    for optimization in ["0", "2"]:
        base = ["gcc", "-std=c11", f"-O{optimization}", "-fno-builtin", "-Wall", "-Wextra", "-Werror", "-I", str(support), str(support / "ams_gra_float64.c")]
        binary = scratch / f"corpus-O{optimization}"
        run(f"build-corpus-O{optimization}", [*base, str(tests / "corpus.c"), "-lm", "-o", str(binary)])
        for locale in ["C", "de_DE.UTF-8"]:
            for mode, filename in [("convert", "conversions.tsv"), ("encode", "encoding.tsv")]:
                label = f"O{optimization}-{locale}-{mode}"
                text = run(label, [str(binary), mode, str(corpus / filename), locale])
                lines = text.splitlines()
                count = manifest["conversion_rows" if mode == "convert" else "encoding_rows"]
                if lines[-1] != f"TASK097_C_CORPUS_COMPLETE mode={mode} count={count} failures=0":
                    raise RuntimeError("incomplete production C corpus")
                if mode == "encode":
                    patterns = (corpus / filename).read_text().splitlines()
                    if len(lines) != len(patterns) + 1:
                        raise RuntimeError("encoding count")
                    for line, pattern in zip(lines[:-1], patterns, strict=True):
                        candidate, expected, token = line.split("\t")
                        if candidate != "production-format17" or expected != pattern or oracle.oracle(token) != expected:
                            raise RuntimeError("independent encoding mismatch")
                elif len(lines) != 1:
                    raise RuntimeError("unexpected C conversion output")
                cells.append({"optimization": optimization, "locale": locale, "mode": mode, "count": count})
        lifecycle = scratch / f"lifecycle-O{optimization}"
        run(f"build-lifecycle-O{optimization}", [*base, str(tests / "lifecycle.c"),
            "-Wl,--wrap=newlocale", "-Wl,--wrap=uselocale", "-Wl,--wrap=snprintf", "-Wl,--wrap=vsnprintf",
            "-pthread", "-lm", "-o", str(lifecycle)])
        text = run(f"lifecycle-O{optimization}", [str(lifecycle)])
        if text != "TASK097_C_LIFECYCLE_COMPLETE faults=6 concurrent_calls=8000 failstop=SIGABRT\n":
            raise RuntimeError("incomplete lifecycle")
        environment = scratch / f"environment-O{optimization}"
        harness = ROOT / "tests/ada-float32-probe"
        run(f"build-environment-O{optimization}", [*base, str(tests / "environment.c"),
            str(harness / "rounding_harness.c"), str(harness / "environment_harness.c"), "-lm", "-o", str(environment)])
        for x87 in range(4):
            for sse in range(4):
                for flushing in range(4):
                    text = run(f"environment-O{optimization}-{x87}-{sse}-{flushing}",
                        [str(environment), str(x87), str(sse), str(flushing)])
                    if text != f"TASK097_C_ENVIRONMENT_COMPLETE x87={x87} mxcsr={sse} flushing={flushing}\n":
                        raise RuntimeError("incomplete C environment")
    paths = [Path(__file__), support / "ams_gra_float64.c", support / "ams_gra_float64.h", tests / "corpus.c", tests / "lifecycle.c",
             tests / "environment.c", harness / "rounding_harness.c", harness / "environment_harness.c"]
    paths += list(scratch.glob("*-O[02]"))
    (scratch / "sources.json").write_text(json.dumps({str(p): hashlib.sha256(p.read_bytes()).hexdigest() for p in paths}, indent=2) + "\n")
    temporary = scratch / "result.json.tmp"
    temporary.write_text(json.dumps({"completed": True, "production_boundary": True, "cells": cells, "lifecycle_optimizations": [0, 2], "environment_processes": 128}, indent=2) + "\n")
    temporary.replace(scratch / "result.json")
    print("TASK097_PRODUCTION_BOUNDARY_COMPLETE cells=8 lifecycle=2", flush=True)


if __name__ == "__main__":
    main()