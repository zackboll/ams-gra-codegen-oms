#!/usr/bin/env python3
"""Confirm every unique newly-READY world tuple with production service-check."""
import concurrent.futures
import os
from pathlib import Path
import subprocess
import sys

repo = Path(__file__).resolve().parent.parent
binary = Path(sys.argv[1]).resolve()
scratch = Path(os.environ["TMPDIR"]) / "task061-service-check"
scratch.mkdir(parents=True, exist_ok=True)
roots = {v: os.environ[f"AMS_GRA_UCI_2_{v[-1]}_ROOT"] for v in ("2.5", "2.6")}
rows = [tuple(line.split("\t")) for line in
        (repo / "tests/fixtures/temporal/task061-newly-ready.tsv").read_text().splitlines()
        if not line.startswith("#")]
assert len(rows) == len(set(rows)) == 609

def check(row):
    version, world, backend, message = row
    stem = f"{version}-{world}-{backend}-{message}"
    contract = scratch / (stem + ".yaml")
    contract.write_text(f'''contract_version: "0.1"
service:
  name: task061
  version: "0.1.0"
  kind: service
standards:
  oms_version: "{version}"
  uci_schema_version: "{version}"
functions:
  - id: f
    name: F
    category: specific
    applicability: applicable
    exchanges:
      - id: e
        kind: oms_message
        direction: output
        mandate: mandatory
        message: {message}
        topic: t
        timing:
          kind: asynchronous
''')
    command = [str(binary), "service-check", "--schema", roots[version], "--contract",
               str(contract), "--language", backend.lower(), "--world",
               "closed-schema" if world == "ClosedSchemaSet" else "open-extensions"]
    result = subprocess.run(command, capture_output=True, check=False)
    (scratch / (stem + ".log")).write_bytes(result.stdout + result.stderr)
    if result.returncode:
        sys.stderr.buffer.write(result.stdout + result.stderr)
    return row, result.returncode

# Only evidence scheduling is parallelized; each invocation is production CLI.
with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:
    results = list(pool.map(check, rows))
for row, status in results:
    print("SERVICE-CHECK", *row, status, sep="\t")
assert all(status == 0 for _, status in results)
print("TASK061 UNIQUE NEWLY READY SERVICE-CHECK: PASSED (609 world tuples; 444 release/backend/message tuples)")