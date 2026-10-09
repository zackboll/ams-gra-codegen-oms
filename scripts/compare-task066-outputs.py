#!/usr/bin/env python3
"""Task066 isolated immutable-parent/current production fixture comparison."""
import collections
import hashlib
import json
from pathlib import Path
import subprocess
import sys

repo = Path(__file__).resolve().parent.parent
before, after, scratch = [Path(p).resolve() for p in sys.argv[1:]]
scratch.mkdir(parents=True, exist_ok=True)
fixtures = sorted(set(repo.glob("crates/**/*.xsd")) | set(repo.glob("tests/**/*.xsd")) | set(repo.glob("examples/**/*.xsd")))
rows = []
counts = collections.Counter()
for index, fixture in enumerate(fixtures):
    for backend in ("ada", "rust", "cpp"):
        for world in ("closed-schema", "open-extensions"):
            results = []
            for phase, binary in (("before", before), ("after", after)):
                out = scratch / str(index) / backend / world / phase
                r = subprocess.run([str(binary), "generate", "--schema", str(fixture), "--language", backend, "--world", world, "--output", str(out)], capture_output=True, text=True)
                hashes = {str(p.relative_to(out)): hashlib.sha256(p.read_bytes()).hexdigest() for p in out.rglob("*") if p.is_file()} if r.returncode == 0 else {}
                results.append((r.returncode, hashes))
                if r.returncode:
                    (scratch / f"{index}-{backend}-{world}-{phase}.log").write_text(r.stdout + r.stderr)
            old, new = results
            if not old[0] and not new[0]:
                category = "identical success" if old[1] == new[1] else "changed success"
            elif old[0] and not new[0]:
                category = "new success"
            elif not old[0] and new[0]:
                category = "regression"
            else:
                category = "shared failure"
            counts[(backend, category)] += 1
            rows.append([str(fixture.relative_to(repo)), backend, world, category, results])
summary = {f"{b}: {c}": n for (b, c), n in sorted(counts.items())}
(scratch / "comparison.json").write_text(json.dumps({"summary": summary, "rows": rows}, indent=2) + "\n")
print(json.dumps(summary, indent=2), flush=True)
assert not any(category in ("regression", "changed success") for _, category in counts)
assert all("codec-unicode31" in row[0] for row in rows if row[3] == "new success")
print("TASK066 FIXTURE COMPARISON: PASSED", flush=True)