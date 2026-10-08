#!/usr/bin/env python3
"""Immutable-parent/current production CLI comparison; no shared build state."""
import collections
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys

repo = Path(__file__).resolve().parent.parent
before, after, scratch = map(lambda p: Path(p).resolve(), sys.argv[1:])
scratch.mkdir(parents=True, exist_ok=True)
counts = collections.Counter()
rows = []
files = sorted(set(repo.glob("crates/**/*.xsd")) | set(repo.glob("tests/**/*.xsd")) | set(repo.glob("examples/**/*.xsd")))
for index, source in enumerate(files):
    for language in ("ada", "rust", "cpp"):
        for world in ("closed-schema", "open-extensions"):
            snapshots = []
            statuses = []
            for label, binary in (("before", before), ("after", after)):
                out = scratch / str(index) / language / world / label
                result = subprocess.run([str(binary), "generate", "--schema", str(source),
                    "--language", language, "--world", world, "--output", str(out)],
                    cwd=repo, env=dict(os.environ), capture_output=True, text=True)
                statuses.append(result.returncode)
                snapshots.append({str(p.relative_to(out)): hashlib.sha256(p.read_bytes()).hexdigest()
                    for p in out.rglob("*") if p.is_file()} if result.returncode == 0 else {})
                if result.returncode:
                    (scratch / f"{index}-{language}-{world}-{label}.log").write_text(result.stdout + result.stderr)
            if statuses == [0, 0]:
                category = "byte-identical success" if snapshots[0] == snapshots[1] else "regression changed success"
            elif statuses[0] and not statuses[1]:
                category = "new success"
            elif not statuses[0] and statuses[1]:
                category = "regression lost success"
            else:
                category = "shared failure"
            counts[(language, category)] += 1
            rows.append([str(source.relative_to(repo)), language, world, category, statuses, snapshots])
summary = {f"{language}: {category}": count for (language, category), count in sorted(counts.items())}
(scratch / "comparison.json").write_text(json.dumps({"summary": summary, "rows": rows}, indent=2) + "\n")
print(json.dumps(summary, indent=2), flush=True)
assert not any("regression" in category for (_, category) in counts), "unexplained output churn"
assert all(language == "ada" for language, category in counts if category == "new success"), "peer change"
print("TASK065 OUTPUT COMPARISON: PASSED", flush=True)