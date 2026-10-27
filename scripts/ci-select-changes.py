#!/usr/bin/env python3
"""Emit the complete CI selection manifest and one compact Actions output."""
import json
import os
from pathlib import Path
from ci_selection import from_environment

manifest = from_environment()
Path("ci-selection.json").write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n")
compact = json.dumps(manifest, sort_keys=True, separators=(",", ":"))
if os.environ.get("GITHUB_OUTPUT"):
    with open(os.environ["GITHUB_OUTPUT"], "a") as output:
        output.write("manifest=" + compact + "\n")
if os.environ.get("GITHUB_STEP_SUMMARY"):
    with open(os.environ["GITHUB_STEP_SUMMARY"], "a") as summary:
        summary.write(f"## CI selection: {manifest['mode']}\n\n"
                      f"Selected ordinary cells: {manifest['selected_cells']}; "
                      f"skipped: {manifest['skipped_cells']}; Deep required: {manifest['deep']}\n\n"
                      "```json\n" + json.dumps(manifest, indent=2, sort_keys=True) + "\n```\n")
print(compact)