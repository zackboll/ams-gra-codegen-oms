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
checks = 0


perf_name = "Qualify batch transport and production equivalence controls"
perf_step = f"""      - name: {perf_name}
        run: |
          set -euo pipefail
          python3 scripts/test-task061-service-impact.py
          cargo test --release --locked -p ams-gra-codegen-oms --test service_check_batch
"""
inherited_targets = ["uci_binary_provenance", "uci_constrained_binary", "uci_member_names",
                     "uci_generated_support", "uci_generated_support", "uci_duration",
                     "uci_bounded_ascii_string",
                     "uci_structured_ascii", "uci_alternating_admission"]


def workflow_steps(workflow):
    # Ignore whole-line comments, never accept commented-out qualification.
    code = "\n".join(line for line in workflow.splitlines()
                     if not line.lstrip().startswith("#")) + "\n"
    jobs = re.findall(r"^  ([\w-]+):\n(.*?)(?=^  [\w-]+:|\Z)", code, re.M | re.S)
    steps = [(job, "      - " + step) for job, body in jobs
             for step in re.split(r"^      - ", body, flags=re.M)[1:]]
    perf = [(job, step) for job, step in steps
            if step.splitlines()[0] == "      - name: " + perf_name]
    assert len(perf) == 1, "PERF-001 qualification must exist exactly once"
    owner, step = perf[0]
    assert owner == "real-uci-regressions", "PERF-001 qualification owner"
    # Exact enabled step and executable body: no filters, conditions, failure
    # masking, alternate targets, or commands that merely echo the required text.
    assert step.strip() == perf_step.strip(), "PERF-001 qualification weakened"
    inherited = []
    for job, step in steps:
        if job not in ("real-uci", "real-uci-regressions") or (job, step) == perf[0]:
            continue
        # Task061/062 helper/CLI steps have dedicated gates, not inline captures.
        if "cargo test" not in step:
            continue
        match = re.search(r"^        run: \|\n((?:          .*\n|\n)+)", step, re.M)
        assert match, ("unknown Cargo test step", step)
        block = textwrap.dedent(match[1])
        assert "cargo test" in block, ("unknown Cargo test step", step)
        target = re.search(r"--test (\w+)", block)
        assert target, ("unknown Cargo test target", step)
        inherited.append((target[1], block))
    assert [target for target, _ in inherited] == inherited_targets, ("inherited Cargo inventory", inherited)
    return inherited, textwrap.dedent(perf[0][1].split("        run: |\n", 1)[1])


def check(name, source, expected, diagnostic, output="", status=0, fail_target="", fail_phase=""):
    global checks
    # Override only Cargo; the extracted workflow shell is executed unchanged.
    prefix = """cargo() {
      printf '%s\\n' "$WRAPPER_OUTPUT"
      printf '%s\\n' 'DEEP_DIAGNOSTIC' >&2
      if [[ -z "$WRAPPER_FAIL_TARGET" || " $* " == *" --test $WRAPPER_FAIL_TARGET "* ]]; then
        if [[ "$WRAPPER_FAIL_PHASE" == listing && " $* " != *" --list "* ]]; then
          return 0
        fi
        if [[ "$WRAPPER_FAIL_PHASE" == execution && " $* " == *" --list "* ]]; then
          return 0
        fi
        return "$WRAPPER_STATUS"
      fi
      return 0
    }
    """
    env = dict(os.environ, WRAPPER_OUTPUT=output, WRAPPER_STATUS=str(status),
               WRAPPER_FAIL_TARGET=fail_target, WRAPPER_FAIL_PHASE=fail_phase)
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
inherited, perf_block = workflow_steps(deep)
for target, block in inherited:
    markers = re.findall(r"grep -Fqx '([^']+)'", block)
    if target in category_markers:
        test, marker = category_markers[target]
        markers.append(f"test {test} ... {marker}")
    counts = re.findall(r"test result: ok\\\. (\d+) passed", block)
    assert counts, target
    summaries = [f"test result: ok. {count} passed; 0 failed" for count in counts]
    output = "\n".join(markers + summaries)
    registration = "task064_real_uci_generated_support_binding: test"
    task064 = "name=task064_real_uci_generated_support_binding" in block
    if task064:
        output = registration + "\n" + output
    check(f"{target}: failed command", block, 37, "DEEP_DIAGNOSTIC", "failure output", 37)
    check(f"{target}: failure with every success marker", block, 43, "DEEP_DIAGNOSTIC", output, 43)
    check(f"{target}: missing markers", block, 1, "missing evidence", "missing evidence")
    check(f"{target}: complete success", block, 0, "DEEP_DIAGNOSTIC", output)
    if task064:
        for phase in ["listing", "execution"]:
            check(f"Task064 {phase} failure with every success marker", block, 47,
                  "DEEP_DIAGNOSTIC", output, 47, fail_phase=phase)
        check("Task064 missing registered name", block, 1, "DEEP_DIAGNOSTIC",
              output.replace(registration, "some_other_test: test"))
        check("Task064 zero executed tests", block, 1, "DEEP_DIAGNOSTIC",
              output.replace("test result: ok. 1 passed", "test result: ok. 0 passed"))
        for marker in markers:
            check(f"Task064 missing {marker}", block, 1, "DEEP_DIAGNOSTIC",
                  output.replace(marker, ""))
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

assert [target for target, _ in inherited] == inherited_targets
print(f"Task052–060 inherited wrappers: PASSED ({checks} checks; {len(inherited)} gates)")


def reject_workflow(name, mutated, diagnostic):
    global checks
    try:
        workflow_steps(mutated)
    except AssertionError as error:
        assert diagnostic in str(error), (name, error)
    else:
        raise AssertionError(f"accepted workflow mutation: {name}")
    checks += 1
    print(f"PASS: workflow rejects {name}")


assert deep.count(perf_step) == 1
for name, replacement, diagnostic in [
    ("missing PERF-001", "", "exactly once"),
    ("duplicated PERF-001", perf_step * 2, "exactly once"),
    ("renamed PERF-001", perf_step.replace(perf_name, perf_name + " renamed"), "exactly once"),
    ("commented-out PERF-001", "".join("#" + line for line in perf_step.splitlines(True)), "exactly once"),
]:
    reject_workflow(name, deep.replace(perf_step, replacement), diagnostic)

for command in perf_block.splitlines():
    for action, replacement in [("missing", ""), ("commented", "# " + command),
                                ("masked", command + " || true")]:
        reject_workflow(f"PERF-001 {action}: {command}",
                        deep.replace(perf_step, perf_step.replace(command, replacement)),
                        "weakened")
for name, old, new in [
    ("disabled step", "        run:", "        if: false\n        run:"),
    ("allowed failure", "        run:", "        continue-on-error: true\n        run:"),
    ("missing pipefail", "set -euo pipefail", "set -eu"),
    ("missing release", "--release ", ""),
    ("missing lock", "--locked ", ""),
    ("listing only", "--test service_check_batch", "--test service_check_batch -- --list"),
    ("filtered Rust tests", "--test service_check_batch", "--test service_check_batch nonexistent"),
    ("wrong Rust target", "service_check_batch", "some_other_test"),
    ("echoed Python", "python3 scripts/", "echo python3 scripts/"),
    ("echoed Cargo", "cargo test", "echo cargo test"),
]:
    reject_workflow("PERF-001 " + name,
                    deep.replace(perf_step, perf_step.replace(old, new)), "weakened")
reject_workflow("moved PERF-001", deep.replace(perf_step, "").replace(
    "  real-uci-regressions:\n", perf_step + "  real-uci-regressions:\n"), "owner")
for name, extra in [
    ("unknown Cargo target", perf_step.replace(perf_name, "Unknown qualification").replace(
        "service_check_batch", "some_other_test")),
    ("unknown duplicate batch gate", perf_step.replace(perf_name, "Unknown batch qualification")),
    ("inline Cargo step", "      - run: cargo test --test some_other_test\n"),
]:
    reject_workflow(name, deep.replace(perf_step, perf_step + extra),
                    "unknown Cargo" if name == "inline Cargo step" else "inherited Cargo inventory")

# Removing any inherited gate must still fail the exact inventory check.
gate_steps = [step for step in re.findall(
    r"^      - name: .*?(?=^      - |^  [\w-]+:|\Z)", deep, re.M | re.S)
    if "cargo test" in step and "--test uci_" in step]
assert len(gate_steps) == len(inherited_targets)
for index, step in enumerate(gate_steps):
    reject_workflow(f"missing inherited gate {index + 1}: {inherited_targets[index]}",
                    deep.replace(step, "", 1), "inherited Cargo inventory")

# Execute the exact PERF-001 shell with both command entry points overridden.
# This verifies order and that either failure is visible and cannot be masked.
for python_status, cargo_status, expected, diagnostic in [
    (0, 0, 0, "BATCH_EXECUTED"),
    (29, 0, 29, "TRANSPORT_EXECUTED"),
    (0, 31, 31, "BATCH_EXECUTED"),
]:
    source = f"""python3() {{ echo TRANSPORT_EXECUTED; return {python_status}; }}
cargo() {{ echo BATCH_EXECUTED; return {cargo_status}; }}
""" + perf_block
    check(f"PERF-001 command statuses {python_status}/{cargo_status}", source, expected, diagnostic)
    run = subprocess.run(["bash", "-c", source], cwd=repo, capture_output=True, text=True)
    assert run.stdout.splitlines() == (["TRANSPORT_EXECUTED"] if python_status else
                                     ["TRANSPORT_EXECUTED", "BATCH_EXECUTED"]), run

print(f"test-task060-ci-wrappers: PASSED ({checks} checks)")
