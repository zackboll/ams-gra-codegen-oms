#!/usr/bin/env python3
"""Narrow C5 compiler-placement and subprocess-diagnostic regressions."""
import contextlib
import importlib.util
import io
import json
from pathlib import Path
import re
import shlex
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
sys.dont_write_bytecode = True
spec = importlib.util.spec_from_file_location("helper_demand", ROOT / "scripts/check-task093-helper-demand.py")
runner = importlib.util.module_from_spec(spec)
spec.loader.exec_module(runner)


def job(text, name):
    match = re.search(r"^  " + re.escape(name) + r":\n.*?(?=^  [\w-]+:|\Z)", text, re.M | re.S)
    if not match:
        raise ValueError(f"missing job: {name}")
    return match[0]


def placement(text):
    # Reuse this bounded workflow reader for PERF-005; comments and unrelated
    # jobs are not qualification inputs.
    text = "\n".join(line for line in text.splitlines() if not line.lstrip().startswith("#"))
    float32_qualification(job(text, "ada-float32-codec"))
    actual = []
    for name in ["ada-hexbinary-codec", "ada-float32-codec"]:
        body = job(text, name)
        if "runs-on: ubuntu-24.04" not in body:
            raise ValueError("unexpected runner")
        if name == "ada-hexbinary-codec" and "container:" in body:
            raise ValueError("portable compiler 16 must remain on Ubuntu")
        if name == "ada-float32-codec":
            for required in [
                "debian@sha256:bcf83fd8af414eda0c0a99d25a4e61dedd88e7aac7f7bc519d04555ff0c40ec5",
                "gnat gnat-14 locales", "gnat --version | grep -E '^GNAT 14\\.2\\.0([[:space:]]|$)'",
                "/tmp/task095-evidence/task093-c5-portable-14/**",
            ]:
                if required not in body:
                    raise ValueError("missing qualified setup/evidence: " + required)
        for step in re.split(r"(?m)^      - ", body)[1:]:
            if "Float32-only helper-demand naming regression" in step:
                required = ("python3 scripts/perf005-ci.py --compilers 14 16", "--alr /opt/alire/bin/alr",
                            "--runtime-root /var/tmp/perf005-float32", "--evidence /tmp/task095-evidence/perf005-ci",
                            "test -s /tmp/task095-evidence/perf005-ci/summary.json",
                            "sha256sum --check --quiet /tmp/task095-evidence/perf005-ci/evidence.sha256")
                if (name != "ada-float32-codec" or any(s not in step for s in required)
                        or any(s in step for s in ("continue-on-error", "--profiles", "--policies", "--red",
                                                   "scripts/check-task093-helper-demand.py"))):
                    raise ValueError("qualified optimized Float32 CI selection missing/weakened")
                actual.extend((name, "float32", c) for c in ("14", "16"))
                continue
            if "scripts/check-task093-helper-demand.py" not in step:
                continue
            if "continue-on-error" in step or "--profiles" in step or "--policies" in step or "--red" in step:
                raise ValueError("naming coverage weakened")
            for line in step.splitlines():
                if not line.strip().startswith("python3 scripts/check-task093-helper-demand.py "):
                    continue
                variants = [line]
                if '"$compiler"' in line:
                    loop = re.search(r"for compiler in ([0-9 ]+); do", step)
                    if not loop:
                        raise ValueError("unrecognized compiler loop")
                    variants = [line.replace('"$compiler"', value) for value in loop[1].split()]
                for variant in variants:
                    args = shlex.split(variant)
                    family = args[args.index("--family") + 1]
                    compiler = args[args.index("--compiler") + 1]
                    if "--ada-codec-runtime" in args:
                        raise ValueError("portable runner must choose its own ordinary GPR/profile")
                    actual.append((name, family, compiler))
    expected = [
        ("ada-hexbinary-codec", "portable", "16"),
        ("ada-float32-codec", "portable", "14"),
        ("ada-float32-codec", "float32", "14"),
        ("ada-float32-codec", "float32", "16"),
    ]
    if sorted(actual) != sorted(expected):
        raise ValueError(f"incomplete/misplaced naming matrix: {actual}")
    return actual


def float32_qualification(body):
    """Guard named qualification commands, not opaque whole-workflow bytes."""
    steps = re.split(r"(?m)^      - ", body)
    # CI-SELECT-001 permits exactly the audited PR selection condition. Main
    # remains unconditional; arbitrary/duplicate conditions remain forbidden.
    conditions = re.findall(r"(?m)^    if: (.*)$", steps[0])
    selected_if = "github.event_name != 'pull_request' || fromJSON(needs.changes.outputs.manifest).jobs['ada-float32-codec']"
    if (conditions != [selected_if] or '    needs: [changes]\n' not in steps[0]
            or re.search(r"(?m)^    continue-on-error:", steps[0])):
        raise ValueError("Float32 job must remain an enabled hard gate")
    required = {
        "Provision and verify qualified target": [
            'test "$(uname -m)" = x86_64',
            'test "$(getconf GNU_LIBC_VERSION)" = \'glibc 2.41\'',
            'localedef -i de_DE -f UTF-8 de_DE.UTF-8',
        ],
        "Install verified Alire 2.1.1": [
            "echo '09c66bcd8c35dd4b97b72c3d9b76e44caa6964a2db35aba069f396f00f1f64c7  /tmp/alire.zip' | sha256sum --check --strict",
            'unzip -q /tmp/alire.zip -d /opt/alire',
            'test "$(/opt/alire/bin/alr --version)" = \'alr 2.1.1\'',
        ],
        "Verify ABI environment and bridge failures": [
            "/tmp/task095-evidence/bridge-controls > /tmp/task095-evidence/bridge.log",
            "grep -Fx 'TASK095_BRIDGE_CONTROLS_COMPLETE checks=20' /tmp/task095-evidence/bridge.log",
            'for mode in 0 1 2 3; do',
            '/tmp/task095-evidence/environment-controls "$mode" > "/tmp/task095-evidence/environment-$mode.log"',
            'count=35; if test "$mode" = 0; then count=32; fi',
            'grep -Fx "TASK095_C2_C_ENVIRONMENT_COMPLETE mode=$mode checks=$count" "/tmp/task095-evidence/environment-$mode.log"',
        ],
        "Execute both compilers profiles and assertion policies": [
            'for compiler in 14 16; do', 'for profile in oam qualified; do',
            'python3 scripts/check-task095.py --compiler "$compiler" --profile "$profile" --numeric-corpus --environment --rounding --scratch "/tmp/task095-evidence/$compiler-$profile" --generator "$GITHUB_WORKSPACE/target/debug/ams-gra-codegen-oms"',
            'sha256sum --check --quiet "/tmp/task095-evidence/$compiler-$profile/sources.sha256"',
            'test -s "/tmp/task095-evidence/$compiler-$profile/result.json"',
            'python3 scripts/check-task095.py --compiler "$compiler" --profile "$profile" --identifiers --scratch "/tmp/task095-evidence/identifiers-$compiler-$profile" --generator "$GITHUB_WORKSPACE/target/debug/ams-gra-codegen-oms"',
            'python3 scripts/check-task095.py --compiler "$compiler" --profile oam --environment --rounding --optimized --scratch "/tmp/task095-evidence/optimized-$compiler" --generator "$GITHUB_WORKSPACE/target/debug/ams-gra-codegen-oms"',
        ],
        "Independent x87 MXCSR C rounding matrix O0 and O2": [
            'run: python3 scripts/check-task095-rounding.py --scratch /tmp/task095-evidence/c-rounding',
        ],
        "Binary Float32 composition in the qualified runtime": [
            'for compiler in 14 16; do',
            'python3 scripts/check-task093-mixed.py --compiler "$compiler" --scratch "/tmp/task095-evidence/task093-mixed-$compiler" --generator "$GITHUB_WORKSPACE/target/debug/ams-gra-codegen-oms"',
            'sha256sum --check --quiet "/tmp/task095-evidence/task093-mixed-$compiler/sources.sha256"',
            'test -s "/tmp/task095-evidence/task093-mixed-$compiler/result.json"',
        ],
        "Float32-only helper-demand naming regression": [
            'python3 scripts/perf005-ci.py --compilers 14 16 --alr /opt/alire/bin/alr --runtime-root /var/tmp/perf005-float32 --evidence /tmp/task095-evidence/perf005-ci --generator "$GITHUB_WORKSPACE/target/debug/ams-gra-codegen-oms"',
            'test -s /tmp/task095-evidence/perf005-ci/summary.json',
            'sha256sum --check --quiet /tmp/task095-evidence/perf005-ci/evidence.sha256',
        ],
        "Portable GNAT 14 helper-demand naming regression": [
            'python3 scripts/check-task093-helper-demand.py --family portable --compiler 14 --scratch /tmp/task095-evidence/task093-c5-portable-14 --generator "$GITHUB_WORKSPACE/target/debug/ams-gra-codegen-oms"',
            'sha256sum --check --quiet /tmp/task095-evidence/task093-c5-portable-14/sources.sha256',
        ],
        "Preserve qualification evidence": [
            'if: always()', 'uses: actions/upload-artifact@v4',
            'name: task095-platform-qualified-float32', 'if-no-files-found: error',
            '/tmp/task095-evidence/perf005-ci/**',
            '/tmp/task095-evidence/task093-mixed-*/**',
            '/tmp/task095-evidence/task093-c5-portable-14/**',
        ],
    }
    for step in steps[1:]:
        lines = [line.strip() for line in step.splitlines() if line.strip()]
        name = lines[0].removeprefix("name: ")
        if any("continue-on-error:" in line or
               (line.startswith("if:") and not (name == "Preserve qualification evidence" and line == "if: always()"))
               for line in lines):
            raise ValueError("Float32 step must remain an enabled hard gate: " + name)
        # Fail closed on common shell bypasses, including a separately inserted
        # early success or disabled errexit, rather than only appended '|| true'.
        run = step.split("        run:", 1)[-1] if "        run:" in step else ""
        if re.search(r"\|\||set\s+\+|exit\s+0|(?:^|[;\s])(?:true|false)(?:[;\s]|$)|if\s+(?:false|\[)", run):
            raise ValueError("Float32 shell qualification bypass: " + name)
        if name in required:
            commands = required.pop(name)
            if 'run: |' in lines:
                commands = ['shell: bash', 'set -euo pipefail', *commands]
            if any(line not in lines for line in commands):
                raise ValueError("missing Float32 qualification command/evidence: " + name)
    if required:
        raise ValueError("missing Float32 qualification steps: " + ", ".join(required))


class CompilerPlacement(unittest.TestCase):
    def setUp(self):
        self.text = (ROOT / ".github/workflows/ci.yml").read_text()

    def test_actual_four_invocations_cover_120_executions(self):
        self.assertEqual(len(placement(self.text)) * 2 * 3 * len(runner.NAMES), 120)

    def test_only_exact_pr_selector_can_condition_float32(self):
        condition = "github.event_name != 'pull_request' || fromJSON(needs.changes.outputs.manifest).jobs['ada-float32-codec']"
        for replacement in ('false', 'success()', "github.event_name == 'pull_request'"):
            with self.subTest(replacement=replacement), self.assertRaises(ValueError):
                placement(self.text.replace(condition, replacement))
        with self.assertRaises(ValueError):
            placement(self.text.replace('  ada-float32-codec:\n', '  ada-float32-codec:\n    if: false\n'))

    def test_portable_14_cannot_return_to_ubuntu(self):
        text = self.text.replace("--family portable --compiler 16", "--family portable --compiler 14")
        with self.assertRaises(ValueError):
            placement(text)

    def test_portable_14_cannot_disappear(self):
        text = self.text.replace("python3 scripts/check-task093-helper-demand.py --family portable --compiler 14", "echo removed --family portable --compiler 14")
        with self.assertRaises(ValueError):
            placement(text)

    def test_float32_compilers_cannot_disappear(self):
        for compiler in ["14", "16"]:
            text = self.text.replace("--compilers 14 16", f"--compilers {compiler}")
            with self.subTest(compiler=compiler), self.assertRaises(ValueError):
                placement(text)

    def test_portable_16_cannot_move_into_container(self):
        text = self.text.replace("  ada-hexbinary-codec:\n", "  ada-hexbinary-codec:\n    container: debian\n")
        with self.assertRaises(ValueError):
            placement(text)

    def test_required_setup_and_artifact_are_guarded(self):
        for required in ["gnat gnat-14 locales", "gnat --version | grep -E '^GNAT 14\\.2\\.0([[:space:]]|$)'", "/tmp/task095-evidence/task093-c5-portable-14/**"]:
            with self.subTest(required=required), self.assertRaises(ValueError):
                placement(self.text.replace(required, "removed"))

    def test_naming_matrix_cannot_be_narrowed_or_ignored(self):
        line = "python3 scripts/check-task093-helper-demand.py --family portable --compiler 16"
        for extra in [" --profiles oam", " --policies normal", " --red"]:
            with self.subTest(extra=extra), self.assertRaises(ValueError):
                placement(self.text.replace(line, line + extra))

    def test_five_cases_and_seven_check_marker_retained(self):
        self.assertEqual(runner.NAMES, ["ArraySample", "Array_Encode_Hex_Binary", "Encode_Hex_Binary", "Array_encode_hex_binary", "Array_Decode_Hex_Binary"])
        self.assertIn('TASK093_C4_HELPER_DEMAND_COMPLETE checks= 7', Path(runner.__file__).read_text())

    def test_probe_precedes_naming_and_retains_exact_versions(self):
        source = Path(runner.__file__).read_text()
        self.assertLess(source.index('"toolchain-preflight"'), source.index('for profile in args.profiles:'))
        self.assertIn('"gnat_external" if args.compiler == "14" else "gnat_native"', source)
        self.assertIn('"14.2.0" if args.compiler == "14" else "16.1.0"', source)
        self.assertIn('temporal.guard.validate_solution', source)


class SubprocessDiagnostics(unittest.TestCase):
    def test_failure_prints_diagnostic_and_preserves_command_exit_log(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            diagnostic = "ERROR: Release within requested versions not found: gnat_external=14.2.0"
            command = [sys.executable, "-c", f"import sys; print({diagnostic!r}); sys.exit(1)"]
            stderr = io.StringIO()
            with contextlib.redirect_stderr(stderr), self.assertRaises(RuntimeError):
                runner.checked(command, root, root / "failure")
            self.assertIn(diagnostic, stderr.getvalue())
            self.assertEqual((root / "failure.log").read_text(), diagnostic + "\n")
            record = json.loads((root / "failure.command.json").read_text())
            self.assertEqual(record["command"], command)
            self.assertEqual(record["exit"], 1)
            self.assertEqual(record["expected"], 0)
            self.assertGreaterEqual(record["elapsed_seconds"], 0)

    def test_success_and_explicit_expected_failure_remain_strict(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.assertEqual(runner.checked([sys.executable, "-c", "print('ok')"], root, root / "success"), (0, b"ok\n"))
            self.assertEqual(runner.checked([sys.executable, "-c", "raise SystemExit(1)"], root, root / "negative", expected=1)[0], 1)


if __name__ == "__main__":
    unittest.main()