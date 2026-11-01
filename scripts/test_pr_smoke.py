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

    def test_task104_ada_only(self):
        paths = ("scripts/task104/admission.py", "scripts/task104/cases.py",
                 "scripts/task104/inventory.py", "scripts/task104/qualify.py",
                 "scripts/task104/sleet.py", "scripts/task104/nested/new.py",
                 "scripts/task104_evidence.py", "scripts/test_task104.py",
                 "scripts/test_task104_evidence.py", "scripts/test_task104_new.py",
                 "tests/ada-real-uci-survivability/survivability_consumer.adb",
                 "tests/ada-real-uci-survivability/codec_probe.adb",
                 "tests/fixtures/task104/uci25-service.yaml")
        for path in paths:
            with self.subTest(path=path):
                self.assert_execution([path], ["ada"])
        self.assert_execution(paths, ["ada"])
        self.assert_execution([paths[0], "docs/task-104-ada-real-uci-survivability-sleet.md"], ["ada"])
        self.assert_execution([paths[0], "crates/backend-rust/src/lib.rs"], ["rust", "ada"])

    def test_task104_ownership_boundaries(self):
        for path in ("scripts/task104_other.py", "scripts/task104_evidence.py.bak",
                     "scripts/task104-other/new.py", "scripts/test_task104_evidence.txt",
                     "scripts/test_task104/nested.py", "scripts/test_task104.py.bak",
                     "tests/fixtures/task104-other/new.yaml", "tests/fixtures/task1040/new.yaml"):
            with self.subTest(path=path):
                self.assert_execution([path], list(smoke.BACKENDS))
        # The existing broad tests/ada-* rule also owns similarly named Ada tests;
        # they must not be mistaken for an unknown shared path.
        self.assert_execution(["tests/ada-real-uci-survivability-other/probe.adb"], ["ada"])

    def test_task104_reconciliation_runs_all_smoke_and_syntax(self):
        self.assert_execution(["scripts/task104/qualify.py", "scripts/pr_smoke.py",
                               ".github/workflows/deep-ci.yml"], list(smoke.BACKENDS), workflows=True)

    def test_task105_ada_only(self):
        paths = ("scripts/task105/admission.py", "scripts/task105/cases.py",
                 "scripts/task105/qualify.py", "scripts/task105/schema.py",
                 "scripts/task105/sleet.py", "scripts/task105/nested/new.py",
                 "scripts/task105_evidence.py", "scripts/test_task105.py",
                 "scripts/test_task105_evidence.py", "scripts/test_task105_new.py",
                 "tests/ada-real-uci-survivability26/survivability_consumer.adb",
                 "tests/ada-real-uci-survivability26/codec_consumer.adb",
                 "tests/ada-real-uci-survivability26/codec_probe.adb",
                 "tests/fixtures/task105/uci26-service.yaml",
                 "tests/fixtures/task105/nested/new.json")
        for path in paths:
            with self.subTest(path=path):
                self.assert_execution([path], ["ada"])
        self.assert_execution(paths, ["ada"])
        self.assert_execution([paths[0], "docs/task-105-ada-survivability-uci26-sleet.md"], ["ada"])
        self.assert_execution([paths[0], "crates/backend-rust/src/lib.rs"], ["rust", "ada"])

    def test_task105_ownership_boundaries(self):
        for path in ("scripts/task105_other.py", "scripts/task105_evidence.py.bak",
                     "scripts/task105-other/new.py", "scripts/task1050/new.py",
                     "scripts/test_task105_evidence.txt", "scripts/test_task105/nested.py",
                     "scripts/test_task105.py.bak", "tests/fixtures/task105-other/new.yaml",
                     "tests/fixtures/task1050/new.yaml"):
            with self.subTest(path=path):
                self.assert_execution([path], list(smoke.BACKENDS))
        # Preserve main's broader tests/ada-* ownership, including similar names.
        self.assert_execution(["tests/ada-real-uci-survivability26-other/probe.adb"], ["ada"])

    def test_task105_reconciliation_runs_all_smoke_and_syntax(self):
        self.assert_execution(["scripts/task105/qualify.py", "scripts/pr_smoke.py",
                               "scripts/test_pr_smoke.py", ".github/workflows/deep-ci.yml"],
                              list(smoke.BACKENDS), workflows=True)

    def test_task106_ada_only(self):
        paths = ("scripts/task106/admission.py", "scripts/task106/cases.py",
                 "scripts/task106/qualify.py", "scripts/task106/sleet.py",
                 "scripts/task106/nested/new.py", "scripts/task106_evidence.py",
                 "scripts/test_task106.py", "scripts/test_task106_evidence.py",
                 "scripts/test_task106_new.py",
                 "tests/ada-real-uci-multimessage/multimessage_consumer.adb",
                 "tests/ada-real-uci-multimessage/nested/probe.adb",
                 "tests/fixtures/task106/uci25-service.yaml",
                 "tests/fixtures/task106/nested/new.json")
        for path in paths:
            with self.subTest(path=path):
                self.assert_execution([path], ["ada"])
        self.assert_execution(paths, ["ada"])
        self.assert_execution([paths[0], "docs/task-106-ada-multimessage-sleet.md"], ["ada"])
        self.assert_execution([paths[0], "crates/backend-rust/src/lib.rs"], ["rust", "ada"])

    def test_task106_ownership_boundaries(self):
        for path in ("scripts/task106_other.py", "scripts/task106_evidence.py.bak",
                     "scripts/task106-other/new.py", "scripts/task1060/new.py",
                     "scripts/test_task106_evidence.txt", "scripts/test_task106/nested.py",
                     "scripts/test_task106.py.bak", "tests/fixtures/task106-other/new.yaml",
                     "tests/fixtures/task1060/new.yaml"):
            with self.subTest(path=path):
                self.assert_execution([path], list(smoke.BACKENDS))
        # Preserve main's broader tests/ada-* ownership, including similar names.
        self.assert_execution(["tests/ada-real-uci-multimessage-other/probe.adb"], ["ada"])

    def test_task106_reconciliation_runs_all_smoke_and_syntax(self):
        self.assert_execution(["scripts/task106/qualify.py", "scripts/pr_smoke.py",
                               "scripts/test_pr_smoke.py", ".github/workflows/deep-ci.yml"],
                              list(smoke.BACKENDS), workflows=True)

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

    def test_task103_ada_only(self):
        paths = ("scripts/task103/admission.py", "scripts/task103/cases.py",
                 "scripts/task103/qualify.py", "scripts/task103/sleet.py",
                 "scripts/task103/nested/new.py", "scripts/task103_evidence.py",
                 "scripts/test_task103.py", "scripts/test_task103_evidence.py",
                 "scripts/test_task103_new.py", "tests/ada-real-uci-rdma/rdma_consumer.adb",
                 "tests/ada-real-uci-rdma/codec_probe.adb",
                 "tests/fixtures/task103/uci25-service.yaml",
                 "tests/fixtures/task103/uci26-service.yaml")
        for path in paths:
            with self.subTest(path=path):
                self.assert_execution([path], ["ada"])
        self.assert_execution(paths, ["ada"])
        self.assert_execution([paths[0], "docs/task-103-ada-real-uci-rdma-sleet.md"], ["ada"])
        self.assert_execution([paths[0], "crates/backend-rust/src/lib.rs"], ["rust", "ada"])
        self.assert_execution([paths[0], "scripts/pr_smoke.py"], list(smoke.BACKENDS))

    def test_task103_ownership_boundaries(self):
        for path in ("scripts/task103_other.py", "scripts/task1030/qualify.py",
                     "scripts/task103_evidence.py.bak", "scripts/task103_evidence_extra.py",
                     "scripts/test_task103_evidence.txt", "scripts/test_task103_evidence.py.bak",
                     "tests/fixtures/task103-other/new.yaml", "tests/fixtures/task1030/new.yaml"):
            with self.subTest(path=path):
                self.assert_execution([path], list(smoke.BACKENDS))
        # The existing broad tests/ada- rule remains Ada-owned independently.
        self.assert_execution(["tests/ada-real-uci-rdma-other/probe.adb"], ["ada"])

    def test_cpp_only(self):
        for path in ("crates/backend-cpp/src/lib.rs", "crates/backend-cpp/src/service_codec.rs",
                     "crates/backend-cpp/src/codec_runtime.hpp", "crates/backend-cpp/tests/temporal.rs",
                     "tests/cpp-codec/consumer.cpp", "tests/cpp-codec/new.cpp",
                     "tests/fixtures/cpp-codec/oam.xsd", "tests/fixtures/cpp-codec/expected/service_codec.hpp",
                     "crates/cli/tests/cpp_codec.rs", "scripts/check-cpp-oms-001.sh",
                     "scripts/check-cpp-oms-001-parity.sh"):
            with self.subTest(path=path):
                self.assert_execution([path], ["cpp"])
        self.assert_execution(["tests/cpp-codec/consumer.cpp", "crates/backend-cpp/src/lib.rs",
                               "docs/task-cpp-oms-001-cpp-oms-json-codecs.md"], ["cpp"])
        self.assert_execution(["crates/backend-cpp/src/lib.rs", ".github/workflows/pr-smoke.yml"], ["cpp"], True)

    def test_cpp_ownership_boundaries(self):
        for path in ("crates/backend-cpp-other/src/lib.rs", "crates/backend-cppx/a.rs",
                     "tests/cpp-codec-other/a.cpp", "tests/cpp-codecs/a.cpp",
                     "tests/fixtures/cpp-codec-other/a.xsd", "scripts/check-cpp-oms-001.sh.bak",
                     "scripts/check-cpp-oms-002.sh", "crates/cli/tests/cpp_codec_extra.rs",
                     "crates/cli/tests/cpp_codec.rs.bak", "crates/cli/tests/service_codec.rs"):
            with self.subTest(path=path):
                self.assert_execution([path], list(smoke.BACKENDS))

    def test_cpp_mixed_shared_and_unknown(self):
        self.assert_execution(["tests/cpp-codec/consumer.cpp", "crates/backend-python/src/lib.rs"],
                              ["python", "cpp"])
        self.assert_execution(["crates/backend-cpp/src/lib.rs", "crates/backend-rust/src/lib.rs"],
                              ["rust", "cpp"])
        self.assert_execution(["crates/backend-cpp/src/lib.rs", "ada/new.ads"], ["ada", "cpp"])
        self.assert_execution(["crates/backend-cpp/src/lib.rs", "python/new.py", "ada/new.ads",
                               "crates/backend-rust/src/lib.rs"], list(smoke.BACKENDS))
        # Shared inputs run every suite, C++ included.
        for shared in ("crates/codegen-core/src/service_codec.rs", "crates/cli/src/lib.rs",
                       "crates/ir/src/lib.rs", "Cargo.lock"):
            self.assert_execution(["crates/backend-cpp/src/lib.rs", shared], list(smoke.BACKENDS))
        self.assert_execution(["new/implementation.xyz", "tests/cpp-codec/consumer.cpp"], list(smoke.BACKENDS))
        # Previous routes are unchanged by the C++ route.
        self.assert_execution(["crates/backend-python/src/lib.rs"], ["python"])
        self.assert_execution(["crates/backend-rust/src/lib.rs"], ["rust"])
        self.assert_execution(["crates/backend-ada/src/lib.rs"], ["ada"])

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
                    out = Path(command[command.index("--output") + 1])
                    out.mkdir()
                    for name in ("oam.hpp", "service_api.hpp", "service_codec.hpp"):
                        (out / name).write_text("")
                if Path(str(command[0])).name == "consumer":
                    return "6313 checks, 0 failures\n"
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
            elif backend == "cpp":
                self.assertIn("--language cpp", joined)
                self.assertIn("--with-codec", joined)
                self.assertIn("--test cpp_codec", joined)
                for flag in ("-std=c++17", "-Werror", "-Wconversion", "-Wold-style-cast"):
                    self.assertIn(flag, joined)
                self.assertIn("tests/cpp-codec/consumer.cpp", joined)
                self.assertNotIn("-fsanitize", joined)
                self.assertNotIn("clang", joined)
                self.assertNotIn("gnat", joined)
            else:
                self.assertIn("gnat --version", joined)
                self.assertIn("alr -n build", joined)

    def run_cpp_smoke(self, consumer_output="6313 checks, 0 failures\n", write_codec=True,
                      failing=None, missing=None):
        seen = []
        def checked(command, **kwargs):
            seen.append((list(map(str, command)), kwargs))
            name = Path(str(command[0])).name
            if name == missing:
                raise FileNotFoundError(name)
            if failing and name == failing:
                raise subprocess.CalledProcessError(1, command)
            if "--output" in command:
                out = Path(command[command.index("--output") + 1])
                out.mkdir()
                for file in ("oam.hpp", "service_api.hpp") + (("service_codec.hpp",) if write_codec else ()):
                    (out / file).write_text("")
            return consumer_output if name == "consumer" else ""
        with tempfile.TemporaryDirectory() as directory, patch.object(smoke, "checked", side_effect=checked):
            smoke.cpp_smoke(Path(directory) / "consumer")
        return seen

    def test_cpp_smoke_executes_generation_compile_and_consumer(self):
        seen = self.run_cpp_smoke()
        commands = [" ".join(command) for command, _ in seen]
        self.assertEqual([c.split()[0] for c in commands if c.startswith(("g++", "cargo"))].count("g++"), 2)
        cargo = [kwargs for command, kwargs in seen if command[0] == "cargo"][0]
        self.assertTrue(cargo["tests"])
        self.assertIn("3 passed; 0 failed", cargo["marker"])
        cargo_command = [command for command, _ in seen if command[0] == "cargo"][0]
        for name in smoke.CPP_SMOKE_TESTS:
            self.assertIn(name, cargo_command)
        self.assertIn("--exact", cargo_command)
        generate = [c for c in commands if "--language cpp" in c][0]
        self.assertIn("service-generate", generate)
        self.assertIn("--with-codec", generate)
        self.assertIn("tests/fixtures/cpp-codec/oam.xsd", generate)
        self.assertIn("tests/fixtures/cpp-codec/service.yaml", generate)
        # Order: tests, generate, compile, then execute the consumer.
        order = [next(i for i, c in enumerate(commands) if needle in c)
                 for needle in ("--test cpp_codec", "service-generate", "-Werror")]
        order.append(next(i for i, c in enumerate(commands) if c.endswith("/consumer") and " " not in c))
        self.assertEqual(order, sorted(order))
        self.assertEqual(order[-1], len(commands) - 1)

    def test_cpp_smoke_fails_closed(self):
        with self.assertRaises(FileNotFoundError):  # no compiler
            self.run_cpp_smoke(missing="g++")
        with self.assertRaises(subprocess.CalledProcessError):  # compile error
            self.run_cpp_smoke(failing="g++")
        with self.assertRaises(subprocess.CalledProcessError):  # failed test run
            self.run_cpp_smoke(failing="cargo")
        with self.assertRaises(subprocess.CalledProcessError):  # failed consumer
            self.run_cpp_smoke(failing="consumer")
        with self.assertRaises(RuntimeError):  # codec not generated
            self.run_cpp_smoke(write_codec=False)
        for output in ("", "0 checks, 0 failures\n", "5 checks, 0 failures\n",
                       "6313 checks, 2 failures\n", "no summary\n"):
            with self.subTest(output=output), self.assertRaises(RuntimeError):
                self.run_cpp_smoke(consumer_output=output)

    def test_cpp_smoke_missing_compiler_is_red_for_real(self):
        with patch.object(smoke.subprocess, "run", side_effect=FileNotFoundError("g++")), \
                tempfile.TemporaryDirectory() as directory, self.assertRaises(FileNotFoundError):
            smoke.cpp_smoke(Path(directory) / "consumer")

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
        # CPP-OMS-001R1: one added step in the same single-runner job.
        self.assertEqual(pr.count("runs-on:"), 1)
        self.assertIn("      - name: C++ smoke\n        if: steps.route.outputs.cpp == 'true'\n"
                      "        run: python3 scripts/pr_smoke.py run cpp", pr)
        toolchain = pr.split("dtolnay/rust-toolchain", 1)[1].split("\n", 2)[1]
        for backend in smoke.BACKENDS:
            if backend != "python":
                self.assertIn(f"steps.route.outputs.{backend} == 'true'", toolchain)
        for backend in smoke.BACKENDS:
            self.assertEqual(pr.count(f"run: python3 scripts/pr_smoke.py run {backend}")
                             + pr.count(f"run: python scripts/pr_smoke.py run {backend}"), 1)
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