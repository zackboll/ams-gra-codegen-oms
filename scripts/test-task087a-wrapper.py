#!/usr/bin/env python3
"""Adversarial registration controls: real subprocesses, no Ada-tool skips."""
import copy
import importlib.util
import pathlib
import sys
import tempfile
import unittest

sys.dont_write_bytecode = True

spec = importlib.util.spec_from_file_location(
    "qualification", pathlib.Path(__file__).with_name("check-task087a.py"))
qualification = importlib.util.module_from_spec(spec)
spec.loader.exec_module(qualification)


class WrapperControls(unittest.TestCase):
    def fake(self, text, status=0):
        with tempfile.TemporaryDirectory(prefix="task087a-wrapper-") as directory:
            root = pathlib.Path(directory)
            command = [sys.executable, "-c",
                       f"import sys; sys.stdout.write({text!r}); sys.exit({status})"]
            qualification.run_probe(command, root, root / "fake.log")

    def test_nonzero_even_with_markers(self):
        with self.assertRaisesRegex(RuntimeError, "command failed"):
            self.fake("GUARDED CORPUS PASSED cases= 199\nTASK087 SUPPORT BOUNDARIES: PASSED\n", 1)

    def test_zero_cases(self):
        with self.assertRaisesRegex(RuntimeError, "corpus count"):
            self.fake("GUARDED CORPUS PASSED cases= 0\nTASK087 SUPPORT BOUNDARIES: PASSED\n")

    def test_missing_completion(self):
        with self.assertRaisesRegex(RuntimeError, "completion marker"):
            self.fake("GUARDED CORPUS PASSED cases= 199\n")

    def test_incorrect_count(self):
        with self.assertRaisesRegex(RuntimeError, "corpus count"):
            self.fake("GUARDED CORPUS PASSED cases= 198\nTASK087 SUPPORT BOUNDARIES: PASSED\n")

    def test_skipped(self):
        with self.assertRaisesRegex(RuntimeError, "skipped qualification"):
            self.fake("SKIPPED unavailable toolchain\nGUARDED CORPUS PASSED cases= 199\nTASK087 SUPPORT BOUNDARIES: PASSED\n")

    def test_invented_pass_evidence(self):
        with self.assertRaisesRegex(RuntimeError, "frozen 199-case"):
            self.fake("PASS\nGUARDED CORPUS PASSED cases= 199\nTASK087 SUPPORT BOUNDARIES: PASSED\n")

    def test_unexpected_exception(self):
        with self.assertRaisesRegex(RuntimeError, "unexpected exception"):
            self.fake("EXCEPTION internal indexing defect\n")

    def test_dependency_pin_controls(self):
        releases = [
            {"name": name, "version": version, "origin": {"hashes": [qualification.PROVIDER_HASH]}}
            for name, version in [("gnatcoll", "26.0.0"), ("gnatcoll_minimal", "26.0.0"),
                                  ("gprbuild", "26.0.1"), ("gnat_native", "16.1.0")]]
        data = {"solution": {"state": [{"release": r} for r in releases]}}
        qualification.validate_solution(data, "gnat_native")
        for index in range(4):
            wrong = copy.deepcopy(data)
            wrong["solution"]["state"][index]["release"]["version"] = "0.0.0"
            with self.subTest(index=index), self.assertRaisesRegex(RuntimeError, "dependency"):
                qualification.validate_solution(wrong, "gnat_native")
        wrong = copy.deepcopy(data)
        wrong["solution"]["state"][0]["release"]["origin"]["hashes"] = []
        with self.assertRaisesRegex(RuntimeError, "archive hash"):
            qualification.validate_solution(wrong, "gnat_native")


if __name__ == "__main__":
    unittest.main(verbosity=2)