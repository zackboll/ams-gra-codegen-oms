#!/usr/bin/env python3
"""Adversarial tests of the actual Task066 exact-test wrapper helper."""
from pathlib import Path
import importlib.util
import shlex
import subprocess
import tempfile
from unittest.mock import patch

repo = Path(__file__).resolve().parent.parent
helper = (repo / "scripts/check-task066-fast.sh").read_text().split("export AMS_GRA_REQUIRE_GNAT=1", 1)[0]
cases = [
    ("pass", "probe: test", "test probe ... ok\ntest result: ok. 1 passed; 0 failed;\nMARKER", 0, 0),
    ("failure diagnostics", "probe: test", "preserved compiler diagnostic", 23, 23),
    ("zero tests", "probe: test", "test result: ok. 0 passed; 0 failed;", 0, 1),
    ("wrong name", "probe: test", "test other ... ok\ntest result: ok. 1 passed; 0 failed;\nMARKER", 0, 1),
    ("unregistered", "other: test", "unused", 0, 1),
    ("duplicate registration", "probe: test\nprobe: test", "unused", 0, 1),
    ("missing marker", "probe: test", "test probe ... ok\ntest result: ok. 1 passed; 0 failed;", 0, 1),
]
for label, listed, output, status, expected in cases:
    with tempfile.TemporaryDirectory(prefix="task066-wrapper-") as temporary:
        stub = Path(temporary) / "cargo"
        stub.write_text(f'''#!/usr/bin/env bash
if [[ "$*" == *--list* ]]; then printf '%s\\n' {shlex.quote(listed)}; exit 0; fi
printf '%s\\n' {shlex.quote(output)}
exit {status}
''')
        script = helper + f"cargo() {{ bash {shlex.quote(str(stub))} \"$@\"; }}\nrun_exact package target probe MARKER"
        r = subprocess.run(["bash", "-c", script], capture_output=True, text=True)
        assert r.returncode == expected, (label, r)
        assert listed in r.stdout
        if label not in ("unregistered", "duplicate registration"):
            assert output in r.stdout
        print(f"PASS {label}: exit {expected}; diagnostics visible")
print(f"TASK066 CI WRAPPER ADVERSARIAL: PASSED ({len(cases)} cases)")

# Import the real helpers without executing inventory, generation or compilers.
spec = importlib.util.spec_from_file_location("task066_vertical", repo / "scripts/check-task066-vertical.py")
vertical = importlib.util.module_from_spec(spec)
with patch.object(subprocess, "run", side_effect=AssertionError("vertical ran on import")):
    spec.loader.exec_module(vertical)

scratch_cases = [
    ({"TMPDIR": "/a", "RUNNER_TEMP": "/b"}, Path("/a")),
    ({"RUNNER_TEMP": "/b"}, Path("/b")),
    ({"TMPDIR": "", "RUNNER_TEMP": "/b"}, Path("/b")),
    ({}, Path("/controlled-system-temp")),
    ({"TMPDIR": "", "RUNNER_TEMP": ""}, Path("/controlled-system-temp")),
]
with patch.object(vertical.tempfile, "gettempdir", return_value="/controlled-system-temp"):
    for env, expected in scratch_cases:
        assert vertical.scratch_root(env) == expected, env
        print(f"PASS scratch environment {env}: {expected}")

target_cases = [
    ({"CARGO_TARGET_DIR": "/configured-target"}, Path("/configured-target")),
    ({"CARGO_TARGET_DIR": "relative-target"}, (repo / "relative-target").resolve()),
    ({}, (repo / "target").resolve()),
    ({"CARGO_TARGET_DIR": ""}, (repo / "target").resolve()),
]
with tempfile.TemporaryDirectory(prefix="task066-generated-cwd-") as generated:
    # A generated-service cwd must not reinterpret relative target storage.
    import os
    original_cwd = Path.cwd()
    try:
        os.chdir(generated)
        for env, expected in target_cases:
            resolved = vertical.cargo_target_root(env)
            assert resolved == expected and resolved.is_absolute(), env
            assert resolved / "task066-vertical" != resolved
            print(f"PASS Cargo environment {env}: {expected}")
    finally:
        os.chdir(original_cwd)
print(f"TASK066 ENVIRONMENT FALLBACKS: PASSED ({len(scratch_cases) + len(target_cases)} cases)")