#!/usr/bin/env python3
"""Separate pre-component-fix/final CLI stability ledger (never rewrites phase one)."""
import collections
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys

repo = Path(__file__).resolve().parent.parent
before, after, scratch = (Path(p).resolve() for p in sys.argv[1:])
scratch.mkdir(parents=True, exist_ok=True)
inputs = [(str(p.relative_to(repo)), p, None) for p in sorted(
    set(repo.glob("crates/**/*.xsd")) | set(repo.glob("tests/**/*.xsd")) | set(repo.glob("examples/**/*.xsd")))]
for release in ("2.5", "2.6"):
    root = os.environ[f"AMS_GRA_UCI_{release.replace('.', '_')}_ROOT"]
    contract = Path(f"/tmp/task065/probes/authorization-integrated-control/{release}.yaml")
    inputs.append((f"UCI-{release}-Authorization", Path(root), contract))
counts = collections.Counter()
rows = []
for index, (label, schema, contract) in enumerate(inputs):
    for language in ("ada", "rust", "cpp"):
        for world in ("closed-schema", "open-extensions"):
            snapshots, statuses = [], []
            for state, binary in (("before", before), ("after", after)):
                out = scratch / str(index) / language / world / state
                args = [str(binary), "service-generate" if contract else "generate",
                        "--schema", str(schema), "--language", language, "--world", world,
                        "--output", str(out)]
                if contract:
                    args += ["--contract", str(contract)]
                result = subprocess.run(args, capture_output=True, text=True)
                statuses.append(result.returncode)
                snapshots.append({str(p.relative_to(out)): p.read_bytes()
                                  for p in out.rglob("*") if p.is_file()} if not result.returncode else {})
                if result.returncode:
                    (scratch / f"{index}-{language}-{world}-{state}.log").write_text(result.stdout + result.stderr)
            changed = []
            if statuses == [0, 0]:
                assert snapshots[0].keys() == snapshots[1].keys(), (label, language, world)
                for name, old in snapshots[0].items():
                    new = snapshots[1][name]
                    if old == new:
                        continue
                    assert language == "ada", (label, language, name)
                    # Remove ONLY expanded subtype marks whose component has the
                    # same Ada identifier. Public identifiers and all other bytes
                    # must then be exactly the pre-fix production output.
                    pattern = rb"\b([A-Za-z][A-Za-z0-9_]*) : ([A-Za-z][A-Za-z0-9_]*(?:\.[A-Za-z][A-Za-z0-9_]*)+)\;"
                    def unqualify(match):
                        component, expanded = match.groups()
                        target = expanded.rsplit(b".", 1)[-1]
                        if component.lower() == target.lower():
                            return component + b" : " + target + b";"
                        return match.group(0)
                    assert re.sub(pattern, unqualify, new) == old, (label, language, name, "unexplained delta")
                    changed.append(name)
                category = "qualification-only success" if changed else "byte-identical success"
            elif statuses[0] == statuses[1]:
                category = "shared failure"
            else:
                raise AssertionError((label, language, world, statuses, "status churn"))
            counts[(language, category)] += 1
            hashes = [{n: hashlib.sha256(b).hexdigest() for n, b in s.items()} for s in snapshots]
            rows.append([label, language, world, category, changed, statuses, hashes])
summary = {f"{language}: {category}": count for (language, category), count in sorted(counts.items())}
(scratch / "comparison.json").write_text(json.dumps({"summary": summary, "rows": rows}, indent=2) + "\n")
print(json.dumps(summary, indent=2), flush=True)
print("TASK065 COMPONENT OUTPUT COMPARISON: PASSED", flush=True)