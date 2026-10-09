#!/usr/bin/env python3
"""Adversarial Task069 exact-test/marker guards, with no downloads or builds."""
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parent.parent


class Wrappers(unittest.TestCase):
    def run_wrapper(self, name, output, status=0):
        with tempfile.TemporaryDirectory(prefix="task069-wrapper-") as tmp:
            directory = Path(tmp)
            for tool, body in {
                "cargo": '#!/bin/sh\nprintf "%s\\n" "$MOCK_OUTPUT"\nexit "$MOCK_STATUS"\n',
                "sha256sum": "#!/bin/sh\ncat >/dev/null\nexit 0\n",
            }.items():
                path = directory / tool
                path.write_text(body)
                path.chmod(0o755)
            env = dict(os.environ, PATH=tmp + os.pathsep + os.environ["PATH"],
                       MOCK_OUTPUT=output, MOCK_STATUS=str(status),
                       AMS_GRA_UCI_2_5_ROOT="mock25", AMS_GRA_UCI_2_6_ROOT="mock26")
            return subprocess.run(["bash", str(ROOT / "scripts" / name)],
                                  env=env, capture_output=True, text=True).returncode

    def test_fast_rejects_empty_filtered_ignored_and_failed(self):
        good = "test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured"
        self.assertEqual(self.run_wrapper("check-task069-fast.sh", good), 0)
        for bad in ["", good.replace("1 passed", "0 passed"),
                    good.replace("0 ignored", "1 ignored"),
                    good.replace("0 failed", "1 failed")]:
            self.assertNotEqual(self.run_wrapper("check-task069-fast.sh", bad), 0)
        self.assertNotEqual(self.run_wrapper("check-task069-fast.sh", good, 101), 0)

    def test_pinned_requires_exact_marker_and_one_test(self):
        marker = "TASK069 REAL UCI SEMANTIC DIFF: PASSED"
        good = "test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured\n" + marker
        self.assertEqual(self.run_wrapper("check-task069-pinned.sh", good), 0)
        for bad in [good.replace(marker, ""), good.replace(marker, "foreign " + marker),
                    good.replace("1 passed", "0 passed"),
                    good.replace("0 ignored", "1 ignored")]:
            self.assertNotEqual(self.run_wrapper("check-task069-pinned.sh", bad), 0)
        self.assertNotEqual(self.run_wrapper("check-task069-pinned.sh", good, 101), 0)


if __name__ == "__main__":
    unittest.main()