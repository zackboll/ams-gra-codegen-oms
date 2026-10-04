#!/usr/bin/env python3
"""Exercise the actual Task 063 exact-test/marker wrapper, not a duplicate."""
from pathlib import Path
import os
import runpy
import shlex
import subprocess
import tempfile
from unittest import mock

repo = Path(__file__).resolve().parent.parent
script = (repo / "scripts/check-task063-fast.sh").read_text()
helper = script[:script.index("for entry in")]
for label, command, status, diagnostic in [
    ("failure", "echo TASK063_DIAGNOSTIC; exit 23", 23, "TASK063_DIAGNOSTIC"),
    ("zero", "echo 'test result: ok. 0 passed'", 1, "0 passed"),
    ("wrong name", "echo 'test other ... ok'; echo 'test result: ok. 1 passed'", 1, "other"),
    ("missing marker", "echo 'test probe ... ok'; echo 'test result: ok. 1 passed'", 1, "1 passed"),
    ("pass", "echo 'test probe ... ok'; echo 'test result: ok. 1 passed'; echo DONE", 0, "DONE"),
]:
    result = subprocess.run(["bash", "-c", helper + f"require_one probe DONE bash -c {shlex.quote(command)}"], capture_output=True, text=True, check=False)
    assert result.returncode == status, (label, result)
    assert diagnostic in result.stdout, (label, result)
    print(f"PASS: {label}: diagnostic visible, exit {status}")
print("TASK063 CI WRAPPER ADVERSARIAL: PASSED (5 checks)")

# Loading the helper and selecting scratch paths must not launch real UCI work.
with mock.patch("subprocess.run", side_effect=AssertionError("unexpected service-check")):
    service_impact = runpy.run_path(str(repo / "scripts/check-task063-service-impact.py"))
    with tempfile.TemporaryDirectory(prefix="task063-temp-root-test-") as directory:
        caller = Path(directory) / "caller"
        runner = Path(directory) / "runner"
        platform = Path(directory) / "platform"
        for label, environment, expected in [
            ("TMPDIR overrides RUNNER_TEMP", {"TMPDIR": str(caller), "RUNNER_TEMP": str(runner)}, caller),
            ("RUNNER_TEMP without TMPDIR", {"RUNNER_TEMP": str(runner)}, runner),
            ("platform temp without either", {}, platform),
        ]:
            with mock.patch.dict(os.environ, environment, clear=True), \
                    mock.patch("tempfile.gettempdir", return_value=str(platform)) as gettempdir:
                scratch = service_impact["scratch_root"]()
                assert scratch == expected / "task063-service-check", (label, scratch, expected)
                if expected == platform:
                    gettempdir.assert_called_once_with()
                else:
                    gettempdir.assert_not_called()
                print(f"PASS: {label}: {scratch}")
print("TASK063 TEMP ROOT REGRESSION: PASSED (3 checks; no service-check commands)")
