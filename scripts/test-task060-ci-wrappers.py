#!/usr/bin/env python3
"""Exercise the actual workflow shell, not a separately implemented guard."""
import pathlib
import subprocess
import tempfile
import textwrap

repo = pathlib.Path(__file__).resolve().parent.parent
fast = (repo / ".github/workflows/ci.yml").read_text()
start = fast.index("      - name: Generated Rust OMS JSON codec (must execute)")
section = fast[start:fast.index("      # Task 051:", start)]
helper = textwrap.dedent(section[section.index("          set -euo pipefail"):section.index("          for test")])
deep = (repo / ".github/workflows/deep-ci.yml").read_text()
start = deep.index("      - name: Real UCI constrained Binary inventory")
block = deep[start:deep.index("      # Task 054:", start)]
block = textwrap.dedent(block[block.index("          set -euo pipefail"):])

with tempfile.TemporaryDirectory(prefix="task060-wrapper-") as directory:
    root = pathlib.Path(directory)
    cases = [
        ("failed command", helper + "require_one_test probe bash -c 'echo DIAGNOSTIC; exit 23'", 23, "DIAGNOSTIC"),
        ("zero tests", helper + "require_one_test probe bash -c 'echo \"test result: ok. 0 passed\"'", 1, "0 passed"),
        ("missing marker", block.replace("cargo test", "fake_cargo test"), 1, "missing evidence"),
        ("deep failure", block.replace("cargo test", "failed_cargo test"), 37, "DEEP_DIAGNOSTIC"),
    ]
    for name, source, expected, diagnostic in cases:
        prefix = "fake_cargo() { echo 'missing evidence'; }; failed_cargo() { echo DEEP_DIAGNOSTIC; return 37; };\n"
        run = subprocess.run(["bash", "-c", prefix + source], cwd=repo, capture_output=True, text=True)
        assert run.returncode == expected, (name, run.returncode, run.stdout, run.stderr)
        assert diagnostic in run.stdout, (name, run.stdout)
        print(f"PASS: {name}: diagnostic visible, exit {expected}")
    # Save an executable copy for focused real exact-test validation.
    (root / "helper.sh").write_text(helper)
print("test-task060-ci-wrappers: PASSED (4 checks)")