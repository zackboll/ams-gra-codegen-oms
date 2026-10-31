"""Portable inventory, full-matrix, compiler/policy and freshness mutations.

Synthetic records test verifier rejection only, never stand in for actual GNAT
diagnostics. Live campaigns retain their genuine compiler traces separately.
"""
import copy
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

import perf009_portable as portable
from perf005_provider import digest

ROOT = Path(__file__).resolve().parents[1]


class Portable(unittest.TestCase):
    def fixture(self, root):
        consumer = root / "oam-ArraySample"
        consumer.mkdir()
        for name in ("helper_demand_consumer.adb", "service_codec.adb", "service_codec.ads",
                     "service_api.ads", "programs.ads", "programs-oam.ads"):
            (consumer / name).write_text("synthetic verifier input\n")
        support = root / "oam-ArraySample-support/src"
        support.mkdir(parents=True)
        for name in ("ams_gra_oms_json_guard.adb", "ams_gra_oms_json_guard.ads"):
            (support / name).write_text("synthetic verifier input\n")
        return consumer

    def test_portable_inventory_is_not_float32_inventory(self):
        with tempfile.TemporaryDirectory() as directory:
            consumer = self.fixture(Path(directory))
            self.assertEqual(len(portable.inventory(consumer)), 5)
            extra = consumer / "unexpected.adb"
            extra.write_text("unexpected")
            with self.assertRaisesRegex(RuntimeError, "inventory"):
                portable.inventory(consumer)
            extra.unlink()
            (consumer / "programs.ads").unlink()
            with self.assertRaisesRegex(RuntimeError, "inventory"):
                portable.inventory(consumer)

    def test_full_ordered_matrix_and_exact_completion_required(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            records = [dict(profile=p, name=n, policy=a, family="portable", compiler="14.2.0",
                            compiler_exit=0, intentional_red=False) for p, n, a in portable.cells()]
            result = dict(completed=True, red=False, records=records)
            for p, n, a in portable.cells():
                (root / f"{p}-{n}-{a}-execute.log").write_bytes(portable.MARKER)
            def publish(value):
                (root / "result.json").write_text(json.dumps(value))
            publish(result)
            portable.validate_result(root, "14")
            for change in ("skip", "duplicate", "order", "family", "compiler", "failed", "red", "incomplete"):
                altered = copy.deepcopy(result)
                if change == "skip":
                    altered["records"].pop()
                elif change == "duplicate":
                    altered["records"][-1] = altered["records"][0]
                elif change == "order":
                    altered["records"].reverse()
                elif change == "family":
                    altered["records"][0]["family"] = "float32"
                elif change == "compiler":
                    altered["records"][0]["compiler"] = "16.1.0"
                elif change == "failed":
                    altered["records"][0]["compiler_exit"] = 1
                elif change == "red":
                    altered["red"] = True
                else:
                    altered["completed"] = False
                publish(altered)
                with self.subTest(change=change), self.assertRaises(RuntimeError):
                    portable.validate_result(root, "14")
            publish(result)
            (root / "oam-ArraySample-normal-execute.log").write_bytes(portable.MARKER + b"extra\n")
            with self.assertRaisesRegex(RuntimeError, "marker"):
                portable.validate_result(root, "14")

    def test_actual_compilation_policy_bind_link_and_freshness_required(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            consumer = self.fixture(root)
            compiler = root / "gcc"
            compiler.write_bytes(b"synthetic compiler identity")
            (root / "oam-ArraySample-provider-identity.log").write_text(
                f"{compiler}\n{digest(compiler)}  {compiler}\n")
            command = ["-f", "-XEXTERNALLY_BUILT=true"]
            data = dict(exit=0, executions=[], gpr_temporary_inputs={
                "/tmp/config.adc": "pragma Assertion_Policy (Check);\n"},
                pre_execution_mapping_inputs={})
            for source in portable.inventory(consumer):
                argv = [str(compiler), "-c", "-gnat2022", "-gnato", "-O0",
                        "-gnatec=/tmp/config.adc", str(source)]
                for executable in (str(compiler), "/synthetic/gnat1"):
                    data["executions"].append(dict(executable=executable, argv=argv,
                                                   exit=0, inclusive_elapsed_seconds=0.1))
                objdir = consumer / "obj" if source.parent == consumer else source.parent.parent / "obj"
                objdir.mkdir(exist_ok=True)
                for suffix in (".ali", ".o"):
                    (objdir / (source.stem + suffix)).write_bytes(b"synthetic artifact")
            for name in ("gnatbind", "ld"):
                data["executions"].append(dict(executable="/synthetic/" + name, argv=[name],
                                               exit=0, inclusive_elapsed_seconds=0.1))
            data["executions"].append(dict(executable=str(compiler),
                argv=[str(compiler), "-c", "b__helper_demand_consumer.adb"],
                exit=0, inclusive_elapsed_seconds=0.1))
            (consumer / "bin").mkdir()
            (consumer / "bin/helper_demand_consumer").write_bytes(b"synthetic executable")
            with patch.dict(portable.COMPILERS, {"14": ("14.2.0", digest(compiler), "", "", "")}):
                self.assertEqual(len(portable.validate_build(command, consumer, data, "14", "Check", 0)["compiled_sources"]), 5)
                for mutation in ("missing", "duplicate", "backend", "bind", "link", "policy", "compiler", "stale", "unforced", "abi", "optimization"):
                    changed = copy.deepcopy(data)
                    stamp = 0
                    cmd = command
                    if mutation == "missing":
                        changed["executions"].pop(0)
                    elif mutation == "duplicate":
                        changed["executions"].append(changed["executions"][0])
                    elif mutation == "backend":
                        changed["executions"].pop(1)
                    elif mutation in ("bind", "link"):
                        changed["executions"] = [r for r in changed["executions"]
                                                 if Path(r["executable"]).name != ("ld" if mutation == "link" else "gnatbind")]
                    elif mutation == "policy":
                        changed["gpr_temporary_inputs"]["/tmp/config.adc"] = "pragma Assertion_Policy (Ignore);\n"
                    elif mutation == "compiler":
                        changed["executions"][0]["executable"] = "/wrong/gcc"
                    elif mutation == "stale":
                        stamp = 2**63
                    elif mutation in ("abi", "optimization"):
                        changed["executions"][0]["argv"].append("-m32" if mutation == "abi" else "-O2")
                    else:
                        cmd = []
                    with self.subTest(mutation=mutation), self.assertRaises(RuntimeError):
                        portable.validate_build(cmd, consumer, changed, "14", "Check", stamp)

    def test_no_partial_or_ci_profile_is_exposed(self):
        for option in ("--profiles", "--policies", "--representative", "--red", "--ci"):
            result = subprocess.run([sys.executable, str(ROOT / "scripts/perf009-portable.py"),
                                     "--compiler", "14", "--scratch", "/unused", "--baseline", "/unused-baseline",
                                     "--generator", "/unused-generator", option], capture_output=True)
            self.assertEqual(result.returncode, 2)
            self.assertIn(b"unrecognized arguments", result.stderr)


if __name__ == "__main__":
    unittest.main()