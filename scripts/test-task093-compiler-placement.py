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


class CompilerPlacement(unittest.TestCase):
    def setUp(self):
        self.text = (ROOT / ".github/workflows/ci.yml").read_text()

    def test_actual_four_invocations_cover_120_executions(self):
        self.assertEqual(len(placement(self.text)) * 2 * 3 * len(runner.NAMES), 120)

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
            text = self.text.replace("for compiler in 14 16; do", f"for compiler in {compiler}; do")
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