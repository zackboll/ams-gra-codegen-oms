#!/usr/bin/env python3
"""Adversarial tests of the actual Task061 workflow helper's exact-test guard."""
from pathlib import Path
import subprocess

repo = Path(__file__).resolve().parent.parent
script = (repo / "scripts/check-task061-fast.sh").read_text()
helper = script[:script.index("for backend in")]
for label, command, status, diagnostic in [
    ("failure", "echo TASK061_DIAGNOSTIC; exit 23", 23, "TASK061_DIAGNOSTIC"),
    ("zero", "echo 'test result: ok. 0 passed'", 1, "0 passed"),
    ("wrong name", "echo 'test other ... ok'; echo 'test result: ok. 1 passed'", 1, "other"),
    ("pass", "echo 'test probe ... ok'; echo 'test result: ok. 1 passed'", 0, "1 passed"),
]:
    result = subprocess.run(["bash", "-c", helper + f"require_one_test probe bash -c {__import__('shlex').quote(command)}"], capture_output=True, text=True, check=False)
    assert result.returncode == status, (label, result)
    assert diagnostic in result.stdout, (label, result)
    print(f"PASS: {label}: diagnostic visible, exit {status}")
print("TASK061 CI WRAPPER ADVERSARIAL: PASSED (4 checks)")
assert "cargo fetch --locked" in (repo / "scripts/check-task061-pinned.sh").read_text()