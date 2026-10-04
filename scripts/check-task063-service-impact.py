#!/usr/bin/env python3
"""Confirm frozen Task 063 newly-READY tuples through production service-check."""
import concurrent.futures
import os
from pathlib import Path
import subprocess
import sys

repo = Path(__file__).resolve().parent.parent
binary = Path(sys.argv[1]).resolve()
scratch = Path(os.environ["TMPDIR"]) / "task063-service-check"
scratch.mkdir(parents=True, exist_ok=True)
rows = [tuple(line.split("\t")) for line in
        (repo / "tests/fixtures/integral/task063-newly-ready.tsv").read_text().splitlines()
        if line and not line.startswith("#")]
assert len(rows) == len(set(rows)) == 9
assert len({(v, b, m) for v, _, b, m in rows}) == 6

def check(row):
    release, world, backend, message = row
    stem = "-".join(row)
    contract = scratch / (stem + ".yaml")
    template = (repo / "tests/fixtures/integral/patterned.yaml").read_text()
    contract.write_text(template.replace("message: Message", f"message: {message}"))
    root = os.environ[f"AMS_GRA_UCI_2_{release[-1]}_ROOT"]
    result = subprocess.run([str(binary), "service-check", "--schema", root,
                             "--contract", str(contract), "--language", backend,
                             "--world", world], capture_output=True, check=False)
    (scratch / (stem + ".log")).write_bytes(result.stdout + result.stderr)
    if result.returncode:
        sys.stderr.buffer.write(result.stdout + result.stderr)
    return row, result.returncode

with concurrent.futures.ThreadPoolExecutor(max_workers=3) as pool:
    results = list(pool.map(check, rows))
for row, status in results:
    print("SERVICE-CHECK", *row, status, sep="\t")
assert all(status == 0 for _, status in results)
print("TASK063 NEWLY READY SERVICE-CHECK: PASSED (9 world tuples; 6 release/backend/message tuples)")