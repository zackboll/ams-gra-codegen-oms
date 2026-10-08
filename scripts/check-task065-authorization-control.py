#!/usr/bin/env python3
"""Retain BEFORE/AFTER smallest-service compiler diagnostics; never hide failure."""
import json
import os
from pathlib import Path
import subprocess
import sys

before, after, scratch = map(lambda p: Path(p).resolve(), sys.argv[1:])
scratch.mkdir(parents=True, exist_ok=True)
rows = []
for release in ("2.5", "2.6"):
    root = os.environ[f"AMS_GRA_UCI_{release.replace('.', '_')}_ROOT"]
    contract = scratch / f"{release}.yaml"
    contract.write_text(f'''contract_version: "0.1"
service:
  name: task065
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
        message: Authorization
        topic: t
        timing:
          kind: asynchronous
''')
    for state, binary in (("before", before), ("after", after)):
        for language in ("ada", "rust", "cpp"):
            args = [str(binary), "service-check", "--schema", root, "--contract", str(contract),
                    "--language", language, "--world", "closed-schema"]
            result = subprocess.run(args, capture_output=True, text=True)
            (scratch / f"{release}-{state}-{language}-check.log").write_text(result.stdout + result.stderr)
            rows.append([release, state, language, "check", result.returncode, "status: READY" in result.stdout])
            print(rows[-1], flush=True)
            assert result.returncode == 0 and "status: READY" in result.stdout
            if language != "ada":
                continue
            out = scratch / release / state
            args[1] = "service-generate"
            result = subprocess.run(args + ["--output", str(out)], capture_output=True, text=True)
            (scratch / f"{release}-{state}-generate.log").write_text(result.stdout + result.stderr)
            rows.append([release, state, language, "generate", result.returncode])
            print(rows[-1], flush=True)
            assert result.returncode == 0
            result = subprocess.run(["gnatmake", "-c", "-q", "-gnat2022", "-gnatwe", "-gnato", "service_api.ads"],
                                    cwd=out, capture_output=True, text=True)
            (scratch / f"{release}-{state}-gnat.log").write_text(result.stdout + result.stderr)
            rows.append([release, state, language, "gnat", result.returncode, result.stderr])
            print(rows[-1], flush=True)
            (scratch / "results.json").write_text(json.dumps(rows, indent=2) + "\n")
(scratch / "results.json").write_text(json.dumps(rows, indent=2) + "\n")
failed = any(row[3] == "gnat" and row[4] for row in rows)
print("TASK065 AUTHORIZATION CONTROL: " + ("COMPILER BLOCKED" if failed else "PASSED"), flush=True)
raise SystemExit(1 if failed else 0)