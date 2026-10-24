#!/usr/bin/env python3
"""Accounting/classification controls; real compiler visibility needs Linux strace."""
import json
from pathlib import Path
import shutil
import sys
import tempfile
import unittest

from perf005_profile import executions, mapping_reads, phase, run


class Accounting(unittest.TestCase):
    def test_mapping_read_reassembles_unfinished_record(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "trace"
            path.write_text('12 100.0 execve("/tool/gnat1", ["gnat1", "-gnatem=/tmp/map", "/case/unit.ads"], 0x1) = 0\n'
                            '12 100.1 read(6</tmp/map>,  <unfinished ...>\n'
                            '12 100.2 <... read resumed>"unit%s\\nunit.ads\\n/case/unit.ads\\n", 4096) = 31\n'
                            '12 101.0 +++ exited with 0 +++\n')
            text = "unit%s\nunit.ads\n/case/unit.ads\n"
            # Set byte count from actual fixture content, not a guessed length.
            path.write_text(path.read_text().replace("= 31", f"= {len(text)}"))
            self.assertEqual(mapping_reads(path), {"/case/unit.ads\n/tmp/map": text})

    def test_ownership(self):
        cases = [(["/tool/bin/gcc", "-c", "/deps/gnatcoll_minimal_26/minimal/src/gnatcoll.ads"], "gnatcoll-minimal"),
                 (["/tool/bin/gcc", "-c", "/deps/gnatcoll_26/core/src/gnatcoll-json.adb"], "gnatcoll-core"),
                 (["gcc", "-c", "/case/oam-ArraySample-support/float32/ams_gra_float32.c"], "repository-support"),
                 (["gcc", "-c", "/case/oam-ArraySample/service_codec.adb"], "generated-consumer"),
                 (["gcc", "-c", "b__consumer.adb"], "bind-link"),
                 (["ar", "cr", "lib.a"], "archive")]
        for command, expected in cases:
            with self.subTest(command=command):
                self.assertEqual(phase(command), expected)

    def test_trace_parser_absolute_exec_and_failure(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "trace"
            path.write_text('12 100.000000 execve("/tool/gcc", ["/tool/gcc", "-c", "x.adb"], 0x1 /* 2 vars */) = 0\n'
                            '12 101.000000 +++ exited with 7 +++\n'
                            '13 100.000000 execve("/tool/gcc", ["/tool/gcc", "-c", "y.adb"], 0x1 /* 2 vars */ <unfinished ...>\n'
                            '13 100.000010 <... execve resumed>) = 0\n'
                            '13 101.000000 +++ exited with 0 +++\n')
            records = executions(path)
            self.assertEqual(records[0]["executable"], "/tool/gcc")
            self.assertEqual(records[0]["exit"], 7)
            self.assertEqual(records[0]["inclusive_elapsed_seconds"], 1)
            self.assertEqual(len(records), 2)
            self.assertEqual(records[1]["exit"], 0)

    def test_sampling_avoids_cumulative_child_cpu(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            # Parent waits while child burns CPU: adding cutime would double count.
            child = "import time; end=time.process_time()+0.5\nwhile time.process_time()<end: pass"
            parent = "import subprocess,sys; subprocess.run([sys.executable,'-c',sys.argv[1]],check=True)"
            code, _ = run([sys.executable, "-c", parent, child], root, root / "controlled", trace=True)
            record = json.loads((root / "controlled.profile.json").read_text())
            sampled = sum(record["sampled_self_cpu_seconds"].values())
            self.assertEqual(code, 0)
            self.assertGreater(sampled, 0.3)
            self.assertLess(sampled, record["cpu_seconds"] * 1.15)
            self.assertLess(abs(record["residual_cpu_seconds"]), 0.25)
            self.assertTrue(any(item["executable"] == sys.executable for item in record["executions"]))

    def test_absolute_compiler_and_failure_propagation(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "broken.c"
            source.write_text("#error PERF005_FAILURE_PROPAGATION\n")
            compiler = shutil.which("gcc")
            self.assertIsNotNone(compiler)
            code, output = run([compiler, "-c", str(source)], root, root / "failure", trace=True)
            self.assertNotEqual(code, 0)
            self.assertIn(b"PERF005_FAILURE_PROPAGATION", output)
            record = json.loads((root / "failure.profile.json").read_text())
            self.assertTrue(any(item["executable"] == compiler and item["exit"] != 0
                                for item in record["executions"]))


if __name__ == "__main__":
    unittest.main()