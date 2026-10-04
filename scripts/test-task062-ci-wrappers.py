#!/usr/bin/env python3
"""Test the actual Fast helper against failure, zero-test and name mutations."""
from pathlib import Path
import shlex
import subprocess
import tempfile

repo = Path(__file__).resolve().parent.parent
script = (repo / "scripts/check-task062-fast.sh").read_text()
helper = script[:script.index("one ipv6_address::")]
checks = 0
for label, listed, output, status, expected in [
    ("pass", "probe: test", "test probe ... ok\ntest result: ok. 1 passed; 0 failed;", 0, 0),
    ("execution failure", "probe: test", "TASK062_DIAGNOSTIC", 23, 23),
    ("zero tests", "probe: test", "test result: ok. 0 passed; 0 failed;", 0, 1),
    ("wrong name", "probe: test", "test other ... ok\ntest result: ok. 1 passed; 0 failed;", 0, 1),
    ("unregistered", "other: test", "unused", 0, 1),
    ("duplicate registration", "probe: test\nprobe: test", "unused", 0, 1),
]:
    with tempfile.TemporaryDirectory(prefix="task062-wrapper-") as tmp:
        command = Path(tmp) / "stub.sh"
        command.write_text(f'''#!/usr/bin/env bash
if [[ "$*" == *--list* ]]; then printf '%s\\n' {shlex.quote(listed)}; exit 0; fi
printf '%s\\n' {shlex.quote(output)}
exit {status}
''')
        result = subprocess.run(["bash", "-c", helper + f"one probe bash {shlex.quote(str(command))}"], capture_output=True, text=True)
        assert result.returncode == expected, (label, result)
        assert listed in result.stdout, (label, result)
        if label not in ["unregistered", "duplicate registration"]:
            assert output in result.stdout, (label, result)
        checks += 1
        print(f"PASS {label}: exit {expected}, diagnostics visible")
print(f"TASK062 CI WRAPPER ADVERSARIAL: PASSED ({checks} checks)")