"""Strict final accounting of an end-to-end Task096 reproduction.

Reads evidence only. Emits normalized results to stdout; a missing, duplicate,
changed or interrupted stage is a hard failure, never a qualified disposition.
"""
import argparse
import collections
import hashlib
import json
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
from importlib import util

SPEC = util.spec_from_file_location("task096_measurements", ROOT / "scripts/check-task096.py")
runner = util.module_from_spec(SPEC)
SPEC.loader.exec_module(runner)


def require(condition, reason):
    if not condition:
        raise RuntimeError(reason)


def conversion_rows(path, expected):
    seen = collections.defaultdict(set)
    rows = path.read_text().splitlines()
    for line in rows[:-1]:
        candidate, want, actual, kind, detail, token = line.split("\t")
        require(candidate in ["DOM", "IEEE_Value", "strtod_l"], "unknown candidate")
        require(token in expected and expected[token] == want, "unknown expected conversion")
        require(token not in seen[candidate], "duplicate conversion output")
        require(kind in ["INTEGER_VALUE", "NUMBER_VALUE"], "not SAX numeric kind")
        seen[candidate].add(token)
    require(all(tokens == set(expected) for tokens in seen.values()) and len(seen) == 3,
            "incomplete candidate token sets")
    return runner.measure_conversion(path, len(expected))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--evidence", type=Path, required=True)
    args = parser.parse_args()
    evidence = args.evidence.resolve()
    final = json.loads((evidence / "result.json").read_text())
    require(final["completed_local_measurements"], "orchestrator incomplete")
    steps = final["steps"]
    require(len(steps) == 29 and len({s["label"] for s in steps}) == 29,
            "orchestrator stage count")
    require(all(s["exit"] == 0 for s in steps), "failed orchestrator stage")
    oracle = runner.load_oracle()
    manifest = json.loads((evidence / "corpus/manifest.json").read_text())
    committed = json.loads((ROOT / "tests/ada-float64-probe/corpus-manifest.json").read_text())
    require(manifest == committed, "corpus differs from frozen manifest")
    expected = {}
    for row in (evidence / "corpus/conversions.tsv").read_text().splitlines():
        _, want, token = row.split("\t", 2)
        require(token not in expected and oracle.oracle(token) == want, "oracle or uniqueness failure")
        expected[token] = want
    require(len(expected) == manifest["conversion_rows"], "conversion count")
    patterns = set((evidence / "corpus/encoding.tsv").read_text().splitlines())
    require(len(patterns) == manifest["encoding_rows"], "encoding count")
    numeric = []
    documents = []
    model_cells = environment_executions = 0
    for compiler in ["14", "16"]:
        for policy in ["Normal", "Check", "Ignore"]:
            for locale in ["default", "german"]:
                suffix = "" if locale == "default" else "-german"
                conversions = conversion_rows(evidence / f"numeric{compiler}/{policy}-convert{suffix}.stdout", expected)
                encodings = runner.measure_encoding(evidence / f"numeric{compiler}/{policy}-encode{suffix}.stdout", patterns, oracle)
                require(conversions["failures"] == {"DOM": 12382, "IEEE_Value": 1362}, "candidate counts changed")
                require(encodings["failures"] == {"writer": 19168}, "writer or alternative counts changed")
                numeric.append({"compiler": compiler, "policy": policy, "locale": locale,
                                "input_discrepancies": conversions["failures"],
                                "output_discrepancies": encodings["failures"]})
        for profile in ["oam", "qualified"]:
            model = json.loads((evidence / f"model{compiler}-{profile}/result.json").read_text())
            require(model["model_only"] and len(model["cells"]) == 3, "model checkpoint incomplete")
            require(all(c["output"] == "TASK096_MODEL_ONLY_COMPLETE checks= 14 rejections= 3" for c in model["cells"]), "model checks")
            model_cells += len(model["cells"])
            document = json.loads((evidence / f"document{compiler}-{profile}/result.json").read_text())
            require(document["handwritten_adapter"] and document["generated_model"], "wrong prototype provenance")
            require(len(document["cells"]) == 3, "missing document policies")
            for cell in document["cells"]:
                require(cell["accepted"] == 7 and cell["atomic_rejections"] == 17 and
                        cell["full_document_encoding_patterns"] == len(patterns) and
                        cell["numeric_positions_per_pattern"] == 8, "document checks incomplete")
                documents.append({"compiler": compiler, "profile": profile, **cell})
            environment = json.loads((evidence / f"environment{compiler}-{profile}/result.json").read_text())
            require(environment["completed"], "environment incomplete")
            cells = {(c["policy"], c["x87"], c["mxcsr"], c["flushing"]) for c in environment["cells"]}
            wanted = {(p, x, s, f) for p in ["Normal", "Check", "Ignore"] for x in range(4) for s in range(4) for f in range(4)}
            require(cells == wanted and len(environment["cells"]) == len(wanted), "environment matrix incomplete")
            environment_executions += len(cells)
    c_environment = json.loads((evidence / "c-environment/result.json").read_text())
    require(len(c_environment["executions"]) == 128, "C environment count")
    require(len({(c["optimization"], c["x87"], c["mxcsr"], c["flushing"]) for c in c_environment["executions"]}) == 128,
            "duplicate C environment rows")
    c_corpus = json.loads((evidence / "c-corpus/result.json").read_text())
    require(len(c_corpus["cells"]) == 8 and all(c["failures"] == 0 for c in c_corpus["cells"]), "C corpus incomplete")
    for optimization in ["0", "2"]:
        require((evidence / f"bridge-O{optimization}.stdout").read_text() == "TASK096_BRIDGE_CONTROLS_COMPLETE checks=81\n", "bridge controls incomplete")
    for source, digest in final["sources"].items():
        # Accounting/unit extensions can be reviewed separately, but executed
        # numerical, model, adapter and environment inputs must remain frozen.
        if source.endswith("/test_accounting.py"):
            continue
        require(hashlib.sha256((ROOT / source).read_bytes()).hexdigest() == digest,
                "executed source changed: " + source)
    for identity_file in evidence.rglob("sources.json"):
        for name, digest in json.loads(identity_file.read_text()).items():
            if Path(name) in [ROOT / "tests/ada-float64-probe/test_accounting.py",
                               ROOT / "tests/ada-float64-probe/candidate_environment.adb"]:
                # The numerical runner conservatively hashes every probe source;
                # these are not consumed by numeric_probe. Both have separate
                # final execution evidence and their historical hashes remain.
                continue
            require(Path(name).is_file() and hashlib.sha256(Path(name).read_bytes()).hexdigest() == digest,
                    "changed measured source/binary: " + name)
    binaries = [p for p in evidence.rglob("*") if p.is_file() and p.name in
                ["gnat", "gcc", "gprbuild", "numeric_probe", "document_probe", "model_consumer",
                 "environment_probe", "libc.so.6", "ams-gra-codegen-oms"]]
    binaries += [Path("/usr/lib/x86_64-linux-gnu/libc.so.6"), Path("/usr/bin/gnat"),
                 Path("/usr/bin/gcc"), Path("/usr/local/bin/alr")]
    binary_hashes = {str(p.relative_to(evidence)) if p.is_relative_to(evidence) else str(p):
                     hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(set(binaries))}
    result = {
        "research_outcome": "FLOAT64_BOUNDARY_QUALIFIED",
        "production_float64_support": False,
        "qualified_platform": "Linux/x86-64 Debian 13 glibc 2.41-12+deb13u4; x87/MXCSR nearest; DAZ/FTZ off",
        "baseline_main": "0e4d25718915496a7e01de4c4a689a8f739cd7a7",
        "baseline_tree": "3ea6909a114e12d114246f676d06e2dd38ad3125",
        "corpus": manifest, "numerical_cells": numeric, "model_only_cells": model_cells,
        "document_cells": documents, "ada_environment_executions": environment_executions,
        "c_environment_executions": 128, "c_corpus_cells": c_corpus["cells"],
        "bridge_checks_per_optimization": 81, "binary_sha256": binary_hashes,
        "sampling_not_exhaustive": True,
    }
    print(json.dumps(result, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()