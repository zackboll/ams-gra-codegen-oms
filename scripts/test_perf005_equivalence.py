#!/usr/bin/env python3
"""Comparator rejection and narrow compiler-command normalization controls."""
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

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
            def trace(name, campaign, option):
                path = root / name
                argv = ["/tool/gcc", "-c", "-x", "ada", option,
                        f"-gnatec=/{campaign}/Check.adc", "-gnatec=/tmp/GPR.1/config.TMP",
                        f"/{campaign}/alire/cache/dependencies/gnatcoll_26/core/src/gnatcoll-json.adb"]
                path.write_text(f'1 100.000000 execve("/tool/gcc", {json.dumps(argv)}, 0x1 /* vars */) = 0\n'
                                '1 101.000000 +++ exited with 0 +++\n')
                return compare.provider_commands(path)
            baseline = trace("baseline", "baseline", "-O2")
            self.assertEqual(baseline, trace("candidate", "candidate", "-O2"))
            self.assertNotEqual(baseline, trace("wrong", "candidate", "-O0"))


if __name__ == "__main__":
    unittest.main()