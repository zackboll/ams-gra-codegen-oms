#!/usr/bin/env python3
"""Comparator rejection and narrow compiler-command normalization controls."""
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from perf005_evidence import effective_args

SPEC = importlib.util.spec_from_file_location("perf005_compare", Path(__file__).with_name("perf005-compare.py"))
compare = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(compare)


class Equivalence(unittest.TestCase):
    def test_partial_campaign_is_not_equivalence(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            left, right = root / "left", root / "right"
            left.mkdir()
            right.mkdir()
            for side in (left, right):
                (side / "result.json").write_text(json.dumps(dict(completed=True, red=False, records=[])))
            with self.assertRaisesRegex(RuntimeError, "incomplete or unordered"):
                compare.compare(left, right)

    def test_command_normalization_retains_effective_options(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            def inputs(campaign):
                return dict(gpr_temporary_inputs={
                    "/tmp/GPR.1/response": f"-I{campaign}/include -O2",
                    "/tmp/GPR.1/specs": "*cc1:\n+ @/tmp/GPR.1/response\n",
                    "/tmp/GPR.1/config": "pragma Assertion_Policy (Check);\n"})
            argv = ["@/tmp/GPR.1/response", "-specs=/tmp/GPR.1/specs", "-gnatec=/tmp/GPR.1/config"]
            left, right = root / "baseline", root / "pilot"
            original = effective_args(inputs(left), argv, left)
            self.assertEqual(original, effective_args(inputs(right), argv, right))
            for name in ("response", "specs", "config"):
                data = inputs(right)
                data["gpr_temporary_inputs"]["/tmp/GPR.1/" + name] += "changed"
                self.assertNotEqual(original, effective_args(data, argv, right))
            data = inputs(right)
            data["gpr_temporary_inputs"]["/tmp/GPR.1/config"] = "pragma Assertion_Policy (Ignore);\n"
            self.assertNotEqual(original, effective_args(data, argv, right))
            del data["gpr_temporary_inputs"]["/tmp/GPR.1/response"]
            with self.assertRaisesRegex(RuntimeError, "incomplete evidence"):
                effective_args(data, argv, right)

    def test_mapping_requires_read_time_input_not_mutated_output(self):
        data = dict(gpr_temporary_inputs={"/tmp/GPR.1/map": "unit%s\nunit.ads\n/wrong/unit.ads\n"})
        with self.assertRaisesRegex(RuntimeError, "pre-execution mapping"):
            effective_args(data, ["-gnatem=/tmp/GPR.1/map"], Path("/case"), source="/case/unit.ads")
        data["pre_execution_mapping_inputs"] = {
            "/case/unit.ads\n/tmp/GPR.1/map": "unit%s\nunit.ads\n/case/unit.ads\n"}
        original = effective_args(data, ["-gnatem=/tmp/GPR.1/map"], Path("/case"), source="/case/unit.ads")
        data["pre_execution_mapping_inputs"]["/case/unit.ads\n/tmp/GPR.1/map"] = "unit%s\nunit.ads\n/unrelated/unit.ads\n"
        self.assertNotEqual(original, effective_args(data, ["-gnatem=/tmp/GPR.1/map"], Path("/case"), source="/case/unit.ads"))


if __name__ == "__main__":
    unittest.main()