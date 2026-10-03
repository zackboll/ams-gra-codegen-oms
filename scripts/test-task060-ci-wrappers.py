#!/usr/bin/env python3
"""Exercise the actual workflow shell, not a separately implemented guard."""
import os
import pathlib
import re
import subprocess
import textwrap

repo = pathlib.Path(__file__).resolve().parent.parent
fast = (repo / ".github/workflows/ci.yml").read_text()
start = fast.index("      - name: Generated Rust OMS JSON codec (must execute)")
section = fast[start:fast.index("      # Task 051:", start)]
helper = textwrap.dedent(section[section.index("          set -euo pipefail"):section.index("          for test")])
deep = (repo / ".github/workflows/deep-ci.yml").read_text()
real_uci = deep.split("  real-uci:\n", 1)[1].split("\n  msrv-real-uci:", 1)[0]
checks = 0


def check(name, source, expected, diagnostic, output="", status=0, fail_target=""):
    global checks
    # Override only Cargo; the extracted workflow shell is executed unchanged.
    prefix = """cargo() {
      printf '%s\\n' "$WRAPPER_OUTPUT"
      printf '%s\\n' 'DEEP_DIAGNOSTIC' >&2
      if [[ -z "$WRAPPER_FAIL_TARGET" || " $* " == *" --test $WRAPPER_FAIL_TARGET "* ]]; then
        return "$WRAPPER_STATUS"
      fi
      return 0
    }
    """
    env = dict(os.environ, WRAPPER_OUTPUT=output, WRAPPER_STATUS=str(status),
               WRAPPER_FAIL_TARGET=fail_target)
    run = subprocess.run(["bash", "-c", prefix + source], cwd=repo,
                         env=env, capture_output=True, text=True)
    assert run.returncode == expected, (name, run.returncode, run.stdout, run.stderr)
    assert diagnostic in run.stdout, (name, run.stdout, run.stderr)
    checks += 1
    print(f"PASS: {name}: diagnostic visible, exit {expected}")


check("failed command", helper + "require_one_test probe bash -c 'echo DIAGNOSTIC; exit 23'",
      23, "DIAGNOSTIC")
check("zero tests", helper + "require_one_test probe bash -c 'echo \"test result: ok. 0 passed\"'",
      1, "0 passed")
qualified = "task060_probes::generated_group_and_carrier_composes_with_production_service_codec"
assert fast.count(qualified) == 2, "AND expectation and filter must both be qualified"
for matched, expected in [(qualified.split("::")[1], 1), (qualified, 0)]:
    output = f"test {matched} ... ok\ntest result: ok. 1 passed"
    check(f"qualified-name guard: {matched}", helper + f"require_one_test {qualified} cargo test",
          expected, matched, output)

category_markers = {
    "uci_member_names": ("task054_real_uci_category_a_selection_generates_and_compiles",
                         "UCI 2.5 REAL CATEGORY-A MEMBER REMAPPING: PASSED"),
    "uci_duration": ("task057_real_uci_category_a_log_generates_and_compiles",
                     "UCI 2.5 REAL CATEGORY-A DURATION SERVICE: PASSED"),
    "uci_bounded_ascii_string": ("task058_real_uci_newly_ready_service_generates_and_compiles",
                                 "UCI 2.5 REAL NEWLY-READY BOUNDED ASCII SERVICE: PASSED"),
}
targets = []
for step in real_uci.split("      - name: ")[1:]:
    match = re.search(r"^        run: \|\n((?:          .*\n|\n)+)", step, re.MULTILINE)
    assert match, step
    block = textwrap.dedent(match[1])
    if "cargo test" not in block:
        continue
    target = re.search(r"--test (\w+)", block)[1]
    targets.append(target)
    markers = re.findall(r"grep -Fqx '([^']+)'", block)
    if target in category_markers:
        test, marker = category_markers[target]
        markers.append(f"test {test} ... {marker}")
    counts = re.findall(r"test result: ok\\\. (\d+) passed", block)
    assert counts, target
    summaries = [f"test result: ok. {count} passed; 0 failed" for count in counts]
    output = "\n".join(markers + summaries)
    check(f"{target}: failed command", block, 37, "DEEP_DIAGNOSTIC", "failure output", 37)
    check(f"{target}: failure with every success marker", block, 43, "DEEP_DIAGNOSTIC", output, 43)
    check(f"{target}: missing markers", block, 1, "missing evidence", "missing evidence")
    check(f"{target}: complete success", block, 0, "DEEP_DIAGNOSTIC", output)
    if target == "uci_alternating_admission":
        check("Task060 second command failure with every success marker", block, 101,
              "DEEP_DIAGNOSTIC", output, 101, "uci_alternating_after")
        check("Task060 second command missing marker", block, 1, "DEEP_DIAGNOSTIC",
              output.replace("UCI 2.6 TASK060 PROJECTED MESSAGE IMPACT: PASSED", ""))
    if target == "uci_duration":
        # Reject each missing requirement, not merely an entirely empty output.
        for requirement in markers + summaries:
            incomplete = "\n".join(line for line in markers + summaries if line != requirement)
            check(f"Task057 missing {requirement}", block, 1, "DEEP_DIAGNOSTIC", incomplete)
        test, marker = category_markers[target]
        check("Task057 standalone Log marker", block, 0, marker,
              output.replace(f"test {test} ... ", ""))
        check("Task057 foreign test prefix rejected", block, 1, marker,
              output.replace(test, "some_other_test"))
        check("Task057 unanchored substring rejected", block, 1, marker,
              output.replace(f"test {test} ... {marker}", f"noise {marker} noise"))

assert targets == ["uci_binary_provenance", "uci_constrained_binary", "uci_member_names",
                   "uci_generated_support", "uci_duration", "uci_bounded_ascii_string",
                   "uci_structured_ascii", "uci_alternating_admission"], targets
print(f"test-task060-ci-wrappers: PASSED ({checks} checks)")