"""Diff routing, exact execution, and failure propagation; no compiler required."""
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
import re
from unittest.mock import patch

import pr_smoke as smoke


class Routing(unittest.TestCase):
    def assert_execution(self, paths, expected, workflows=False):
        calls = []
        selected, syntax = smoke.classify(paths)
        smoke.execute(selected, syntax, calls.append, lambda: calls.append("workflows"))
        self.assertEqual(calls, (["workflows"] if workflows else []) + expected)

    def test_python_only(self):
        for path in ("crates/backend-python/src/lib.rs", "python/qualification/fast/test_new.py",
                     "tests/fixtures/python-codec/new.json", "crates/cli/tests/python_codec.rs"):
            self.assert_execution([path], ["python"])

    def test_rust_only(self):
        for path in ("crates/backend-rust/src/lib.rs", "crates/runtime-rust/src/lib.rs",
                     "crates/runtime-api-rust/src/lib.rs", "crates/runtime-rust-facade-tests/tests/a.rs"):
            self.assert_execution([path], ["rust"])

    def test_ada_only(self):
        for path in ("crates/backend-ada/src/lib.rs", "ada/oms-json-support/src/a.adb",
                     "qualification/ada/fast/new/consumer.adb", "tests/ada-generated-codec/codec_consumer.adb",
                     "tests/fixtures/service-generate/codec-ada-minimal.xsd"):
            self.assert_execution([path], ["ada"])

    def test_shared_and_dependencies(self):
        for path in ("crates/ir/src/lib.rs", "crates/xsd-frontend/src/lib.rs",
                     "crates/codegen-core/src/lib.rs", "crates/cli/src/main.rs", "Cargo.lock",
                     "Cargo.toml", "rust-toolchain.toml", "scripts/pr_smoke.py", "tests/pr-smoke/ada_consumer.adb"):
            self.assert_execution([path], list(smoke.BACKENDS))
        self.assert_execution(["crates/backend-python/Cargo.toml"], list(smoke.BACKENDS))

    def test_task102_ada_only(self):
        paths = ("scripts/task102/admission.py", "scripts/task102/cases.py",
                 "scripts/task102/constrained.py", "scripts/task102/qualify.py",
                 "scripts/task102/sleet.py", "scripts/task102/nested/new.py",
                 "scripts/task102_evidence.py", "scripts/test_task102.py",
                 "scripts/test_task102_evidence.py", "scripts/test_task102_new.py",
                 "tests/ada-real-uci-log/log_consumer.adb",
                 "tests/ada-real-uci-log/hash_probe.adb",
                 "tests/fixtures/task102/uci25-service.yaml",
                 "tests/fixtures/task102/uci26-service.yaml")
        for path in paths:
            with self.subTest(path=path):
                self.assert_execution([path], ["ada"])
        self.assert_execution(paths, ["ada"])
        self.assert_execution([paths[0], "docs/task-102-ada-real-uci-log-sleet.md"], ["ada"])
        self.assert_execution([paths[0], "crates/backend-rust/src/lib.rs"], ["rust", "ada"])

    def test_task102_ownership_boundaries(self):
        for path in ("scripts/task102_other.py", "scripts/task102_evidence.py.bak",
                     "scripts/test_task102_evidence.txt", "tests/fixtures/task102-other/new.yaml"):
            with self.subTest(path=path):
                self.assert_execution([path], list(smoke.BACKENDS))

    def test_mixed(self):
        self.assert_execution(["crates/backend-python/src/lib.rs", "ada/new.ads"], ["python", "ada"])
        self.assert_execution(["crates/backend-rust/src/lib.rs", "python/new.py"], ["python", "rust"])

    def test_docs(self):
        self.assert_execution(["README.md", "docs/ci-reset.md", "LICENSE"], [])
        # An implementation named .md is not automatically documentation.
        self.assert_execution(["crates/ir/implementation.md"], list(smoke.BACKENDS))

    def test_workflows(self):
        self.assert_execution([".github/workflows/ci.yml"], [], True)
        self.assert_execution([".github/workflows/ci.yml", "scripts/pr_smoke.py"], list(smoke.BACKENDS), True)

    def test_unknown_ambiguous_and_unclassifiable(self):
        self.assert_execution(["new/implementation.xyz"], list(smoke.BACKENDS))
        for path in ("", "../python/a.py", "/python/a.py", "python//a.py", "python/./a.py",
                     "python/a\nb.py", "python/evil\\name.py", "python/\udcff.py", None):
            self.assert_execution([path], list(smoke.BACKENDS), True)
        self.assert_execution([], list(smoke.BACKENDS), True)

    def test_actual_added_deleted_and_renamed_diff(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            def git(*args):
                return subprocess.check_output(["git", *args], cwd=root, stderr=subprocess.STDOUT).decode().strip()
            git("init", "-q")
            git("config", "user.email", "smoke@example.test")
            git("config", "user.name", "Smoke Test")
            (root / "python").mkdir()
            (root / "python/deleted.py").write_text("same bytes\n")
            git("add", ".")
            git("commit", "-qm", "base")
            base = git("rev-parse", "HEAD")
            (root / "ada").mkdir()
            git("mv", "python/deleted.py", "ada/renamed.adb")
            git("commit", "-qm", "rename")
            with patch.object(smoke, "ROOT", root):
                calls = []
                smoke.execute(*smoke.route(base, git("rev-parse", "HEAD")), calls.append)
                self.assertEqual(calls, ["python", "ada"])
            base = git("rev-parse", "HEAD")
            git("rm", "ada/renamed.adb")
            git("commit", "-qm", "delete")
            with patch.object(smoke, "ROOT", root):
                calls = []
                smoke.execute(*smoke.route(base, git("rev-parse", "HEAD")), calls.append)
                self.assertEqual(calls, ["ada"])

    def test_diff_errors_run_every_suite(self):
        for error in (OSError("git absent"), subprocess.CalledProcessError(128, "git")):
            with patch.object(smoke.subprocess, "check_output", side_effect=error):
                calls = []
                smoke.execute(*smoke.route("a" * 40, "b" * 40), calls.append, lambda: calls.append("syntax"))
                self.assertEqual(calls, ["syntax", *smoke.BACKENDS])
        with patch.object(smoke.subprocess, "check_output", return_value=b"python/\xff.py\0"):
            self.assertEqual(smoke.route("a" * 40, "b" * 40), (set(smoke.BACKENDS), True))


class Execution(unittest.TestCase):
    def test_deliberate_failure_is_red(self):
        with self.assertRaises(subprocess.CalledProcessError):
            smoke.execute({"python"}, False,
                          lambda backend: smoke.checked([sys.executable, "-c", "raise SystemExit(7)"]))

    def test_missing_executable_marker_tests_and_skip_are_red(self):
        with self.assertRaises(FileNotFoundError):
            smoke.checked(["/nonexistent/pr-smoke-executable"])
        with self.assertRaises(RuntimeError):
            smoke.checked([sys.executable, "-c", "print('not executed')"], marker="PASSED")
        for text in ("", "test result: ok. 0 passed; 0 failed; 0 ignored",
                     "test result: ok. 2 passed; 0 failed; 1 ignored",
                     "skipping required probe\ntest result: ok. 1 passed; 0 failed; 0 ignored"):
            with self.subTest(text=text), self.assertRaises(RuntimeError):
                smoke.checked([sys.executable, "-c", f"print({text!r})"], tests=True)

    def test_actual_suites_never_invoke_unrelated_backend(self):
        for backend in smoke.BACKENDS:
            commands = []
            def checked(command, **kwargs):
                commands.append(" ".join(map(str, command)))
                if "--output" in command:
                    Path(command[command.index("--output") + 1]).mkdir()
                return ""
            with patch.object(smoke, "checked", side_effect=checked):
                smoke.execute({backend}, False)
            joined = "\n".join(commands)
            self.assertIn("cargo test --locked -p ams-gra-oms-backend-" + backend, joined)
            self.assertIn("cargo build --locked -p ams-gra-codegen-oms --bin", joined)
            for other in set(smoke.BACKENDS) - {backend}:
                self.assertNotIn("-p ams-gra-oms-backend-" + other, joined)
                self.assertNotIn("--language " + other, joined)
            self.assertNotIn("--workspace", joined)
            self.assertNotIn("deep-ci", joined)
            if backend == "python":
                self.assertIn("codec_runtime.py", joined)
            elif backend == "rust":
                self.assertIn("rustc --edition=2024 main.rs", joined)
            else:
                self.assertIn("gnat --version", joined)
                self.assertIn("alr -n build", joined)

    def test_actual_main_outcome_gate_fails_on_every_unsuccessful_result(self):
        text = (smoke.ROOT / ".github/workflows/ci.yml").read_text()
        gate = text.split("  main-ci-required:\n", 1)[1]
        script = gate.split("          python3 - <<'PYTHON'\n", 1)[1].split("          PYTHON", 1)[0]
        script = "\n".join(line[10:] for line in script.splitlines())
        import json
        for result in ("success", "failure", "skipped", "cancelled", "timed_out"):
            with self.subTest(result=result):
                env = dict(os.environ, NEEDS_JSON=json.dumps({"deep-qualification": {"result": result}}))
                completed = subprocess.run([sys.executable, "-c", script], env=env, capture_output=True)
                self.assertEqual(completed.returncode == 0, result == "success")

    def test_pr_and_main_workflow_contract(self):
        workflows = smoke.ROOT / ".github/workflows"
        pr = (workflows / "pr-smoke.yml").read_text()
        self.assertIn("  pull_request:\n", pr)
        self.assertNotIn("branches:", pr)
        self.assertEqual(re.findall(r"^  ([\w-]+):$", pr.split("jobs:\n", 1)[1], re.M), ["smoke"])
        self.assertIn("name: PR smoke", pr)
        self.assertIn("ref: ${{ github.sha }}", pr)
        for forbidden in ("paths:", "paths-ignore:", "matrix:", "secrets.", "pull_request_target", "continue-on-error"):
            self.assertNotIn(forbidden, pr)
        for file in ("ci.yml", "ci002-validation.yml"):
            self.assertNotIn("  pull_request:", (workflows / file).read_text())
        main = (workflows / "ci.yml").read_text()
        for retired in ("needs.changes", "ci-policy.py", "check-ci-split.sh", "ci-required-gate.py"):
            self.assertNotIn(retired, main)
        self.assertIn("uses: ./.github/workflows/deep-ci.yml", main)
        self.assertIn("  workflow_dispatch:", main)
        self.assertNotIn("inputs.selection", (workflows / "deep-ci.yml").read_text())


if __name__ == "__main__":
    unittest.main()