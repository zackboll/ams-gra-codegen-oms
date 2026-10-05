#!/usr/bin/env python3
"""Adversarial tests of the actual Task066 exact-test wrapper helper."""
from pathlib import Path
import shlex
import subprocess
import tempfile

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