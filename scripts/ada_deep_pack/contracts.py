"""Exact workflow placement plus frozen original campaign contract checks."""
from pathlib import Path
import subprocess

from . import run
from workflow_text import blocks


def check(test, suite):
    pack = next(p for p, names in run.INVENTORY["packs"].items() if suite in names)
    header, jobs = blocks((run.ROOT / ".github/workflows/deep-ci.yml").read_text())
    job = jobs["ada-deep-pack-" + pack]
    spec = run.INVENTORY["suites"][suite]
    original = subprocess.check_output(["git", "show", run.INVENTORY["origin_commit"] +
                                       ":.github/workflows/deep-ci.yml"], cwd=run.ROOT, text=True)
    original_job = blocks(original)[1][suite]
    test.assertNotIn(suite, jobs)
    test.assertEqual(spec["main_only"], "    if: github.ref == 'refs/heads/main'\n" in original_job)
    if pack == "b":
        test.assertIn("    if: github.ref == 'refs/heads/main'\n", job)
    else:
        test.assertFalse(any(line.startswith("    if:") for line in job.splitlines()))
    test.assertIn("compiler: ['14', '16']", job)
    for compiler, slot in zip(("14", "16"), (4, 9) if pack == "a" else (10, 5)):
        test.assertIn(f"compiler: '{compiler}'\n            slot: {slot}", job)
    for required in ("group: ${{ format('oms-main-slot-{0}', matrix.slot) }}", "queue: max",
                     "timeout-minutes: 240", "scripts/ada_deep_pack/run.py",
                     "scripts/ada_deep_pack/verify.py", "--expected-sha '${{ github.sha }}'",
                     "if-no-files-found: error", "include-hidden-files: true",
                     "--workers " + ("4" if pack == "a" else "3"), spec["artifact"]["name"]):
        test.assertIn(required, job)
    for forbidden in ("continue-on-error", "inputs.selection", "pull_request", "oms-pr-"):
        test.assertNotIn(forbidden, header + job)
    for step in spec["steps"]:
        for line in step["run"].splitlines():
            test.assertIn(line, original_job)
    commands = "\n".join(step["run"] for step in spec["steps"])
    test.assertIn(f"scripts/task{spec['task']}/qualify.py", commands)
    if spec["task"] != 100:
        test.assertIn(f"from task{spec['task']}_evidence import verify_bundle", commands)
        test.assertIn(f"TASK{spec['task']}_HOSTED_VERIFIED", commands)
    test.assertIn("uses: ./.github/workflows/deep-ci.yml",
                  (run.ROOT / ".github/workflows/ci.yml").read_text())