#!/usr/bin/env python3
"""Exercise the actual Task 063 exact-test/marker wrapper, not a duplicate."""
from pathlib import Path
import shlex
import subprocess

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