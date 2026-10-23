#!/usr/bin/env python3
"""Job-local mutation regression, including overlapping independent setup."""
import importlib.util
import pathlib
import re
import subprocess
import sys
import tempfile
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[1]
sys.dont_write_bytecode = True
spec = importlib.util.spec_from_file_location("layout", ROOT / "scripts/check-task077b-ci-layout.py")
layout = importlib.util.module_from_spec(spec)
spec.loader.exec_module(layout)


class MutationOwnership(unittest.TestCase):
    def test_only_exact_evidence_upload_is_exempt(self):
        original = (ROOT / ".github/workflows/deep-ci.yml").read_text()
        for changed in [
                original.replace('if: always()', 'if: false'),
                original.replace('name: Preserve Task 061 per-selection evidence', 'name: Some gate'),
                original.replace('name: Preserve Task 061 per-selection evidence\n',
                                 'name: Preserve Task 061 per-selection evidence\n        continue-on-error: true\n'),
                original.replace('name: task061-service-check\n', 'name: foreign-artifact\n')]:
            with tempfile.TemporaryDirectory() as tmp:
                path = pathlib.Path(tmp) / 'deep.yml'
                path.write_text(changed)
                self.assertIn('Task077B enabled hard gate: real-uci-regressions', layout.check(path))

    def test_actual_task092_and_overlapping_predecessor(self):
        original = (ROOT / ".github/workflows/deep-ci.yml").read_text()
        self.assertLess(original.index("  ada-repeated-choice-real-uci:"),
                        original.index("  real-uci:"))
        independent = '''  independent-layout-regression:
    runs-on: ubuntu-latest
    steps:
      - name: Independent setup with overlapping commands
        run: |
          UCI25="$(scripts/fetch-pinned-uci-2.5.sh "$RUNNER_TEMP/independent25")"
          UCI26="$(scripts/fetch-pinned-uci-2.6.sh "$RUNNER_TEMP/independent26")"
          cargo build --release --locked -p ams-gra-codegen-oms
          cargo fetch --locked
'''
        for text in [original, original.replace("jobs:\n", "jobs:\n" + independent, 1)]:
            with self.subTest(prepended=text != original), tempfile.TemporaryDirectory(
                    prefix="task077b-owner-") as tmp:
                path = pathlib.Path(tmp) / "deep.yml"
                path.write_text(text)
                self.assertEqual(layout.check(path), [])
                actual_run = subprocess.run
                observed = []

                def inspect_mutation(command, **kwargs):
                    mutated = pathlib.Path(command[3]).read_text()
                    jobs = dict(re.findall(r"^  ([\w-]+):\n(.*?)(?=^  [\w-]+:|\Z)",
                                           text, re.M | re.S))
                    changed = dict(re.findall(r"^  ([\w-]+):\n(.*?)(?=^  [\w-]+:|\Z)",
                                              mutated, re.M | re.S))
                    for job in jobs.keys() - {"real-uci", "real-uci-regressions"}:
                        self.assertEqual(changed.get(job), jobs[job], job)
                    observed.append(mutated)
                    return actual_run(command, **kwargs)

                # The complete existing 54-case self-test still runs the real
                # guard and checks each attributed diagnostic and non-no-op.
                from unittest.mock import patch
                with patch.object(layout.subprocess, "run", side_effect=inspect_mutation):
                    layout.self_test(str(path), str(ROOT / "scripts/check-ci-split.sh"),
                                     str(ROOT / ".github/workflows/ci.yml"),
                                     str(ROOT / "scripts/run-real-sleet-test.sh"))
                self.assertEqual(len(observed), 54)
                self.assertEqual(observed.count(text), 1)  # Healthy case only.


if __name__ == "__main__":
    unittest.main()