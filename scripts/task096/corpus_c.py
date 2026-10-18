"""C corpus campaigns at O0/O2 under C and German thread-local locales."""
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
    manifest = json.loads((corpus / "manifest.json").read_text())
    for name, digest in manifest["hashes"].items():
        if hashlib.sha256((corpus / name).read_bytes()).hexdigest() != digest:
            raise RuntimeError("changed corpus")
    paths = [ROOT / "tests/ada-float64-probe" / name for name in ["boundary.c", "corpus.c"]]
    results = []
    for optimization in ["0", "2"]:
        binary = scratch / f"corpus-O{optimization}"
        command = ["gcc", "-std=c11", f"-O{optimization}", "-Wall", "-Wextra", "-Werror", *map(str, paths), "-lm", "-o", str(binary)]
        (scratch / f"build-O{optimization}.command.json").write_text(json.dumps(command) + "\n")
        subprocess.run(command, check=True)
        for locale in ["C", "de_DE.UTF-8"]:
            for mode, filename in [("convert", "conversions.tsv"), ("encode", "encoding.tsv")]:
                label = f"O{optimization}-{locale}-{mode}"
                command = [str(binary), mode, str(corpus / filename), locale]
                (scratch / f"{label}.command.json").write_text(json.dumps(command) + "\n")
                with (scratch / f"{label}.stdout").open("wb") as out, (scratch / f"{label}.stderr").open("wb") as err:
                    process = subprocess.run(command, stdout=out, stderr=err)
                (scratch / f"{label}.exit").write_text(str(process.returncode) + "\n")
                if process.returncode:
                    raise RuntimeError(f"failed {label}")
                lines = (scratch / f"{label}.stdout").read_text().splitlines()
                count = manifest["conversion_rows" if mode == "convert" else "encoding_rows"]
                if lines[-1] != f"TASK096_C_CORPUS_COMPLETE mode={mode} count={count} failures=0":
                    raise RuntimeError("incomplete C corpus")
                if mode == "encode":
                    patterns = (corpus / filename).read_text().splitlines()
                    if len(lines) != len(patterns) + 1:
                        raise RuntimeError("encoding count")
                    for line, pattern in zip(lines[:-1], patterns, strict=True):
                        _, expected, token = line.split("\t")
                        if expected != pattern or oracle.oracle(token) != expected:
                            raise RuntimeError("encoding mismatch")
                elif len(lines) != 1:
                    raise RuntimeError("unexpected C conversion output")
                results.append({"optimization": optimization, "locale": locale, "mode": mode, "count": count, "failures": 0})
                print("COMPLETE", label, flush=True)
    paths += [Path(__file__), *scratch.glob("corpus-O*")]
    (scratch / "sources.json").write_text(json.dumps({str(p): hashlib.sha256(p.read_bytes()).hexdigest() for p in paths}, indent=2) + "\n")
    temporary = scratch / "result.json.tmp"
    temporary.write_text(json.dumps({"completed": True, "cells": results}, indent=2) + "\n")
    temporary.replace(scratch / "result.json")
    print("TASK096_C_CORPUS_MATRIX_COMPLETE cells=8", flush=True)


if __name__ == "__main__":
    main()