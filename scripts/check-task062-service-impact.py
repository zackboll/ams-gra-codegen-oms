#!/usr/bin/env python3
"""Confirm every frozen newly READY release/world/backend/message CLI tuple."""
import os
from pathlib import Path
import subprocess
import sys

repo = Path(__file__).resolve().parent.parent
binary = Path(sys.argv[1]).resolve()
scratch = Path(os.environ["TMPDIR"]) / "task062-service-impact"
scratch.mkdir(parents=True, exist_ok=True)
rows = (repo / "tests/fixtures/string/task062-new-ready.tsv").read_text().splitlines()
assert len(rows) == len(set(rows)) == 54
assert len({(r.split('\t')[0], r.split('\t')[2], r.split('\t')[3]) for r in rows}) == 30
for row in rows:
    release, world, backend, message = row.split('\t')
    contract = scratch / "contract.yaml"
    contract.write_text(f'''contract_version: "0.1"
service:
  name: task062
  version: "0.1.0"
  kind: service
standards:
  oms_version: "{release}"
  uci_schema_version: "{release}"
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
    result = subprocess.run([str(binary), "service-check", "--schema",
        os.environ[f"AMS_GRA_UCI_{release.replace('.', '_')}_ROOT"], "--contract", str(contract),
        "--language", backend.lower(), "--world",
        "closed-schema" if world == "ClosedSchemaSet" else "open-extensions"], capture_output=True, text=True)
    print(row, "exit", result.returncode, result.stdout + result.stderr, flush=True)
    if result.returncode:
        raise SystemExit(result.returncode)
    assert "READY" in result.stdout
print("TASK062 ALL 54 NEWLY READY CLI TUPLES: PASSED", flush=True)