"""Focused coverage of the shared main/Deep runner lane assignments."""
from pathlib import Path
import re
import unittest

from workflow_text import blocks


ROOT = Path(__file__).resolve().parents[1]


class MainLanes(unittest.TestCase):
    def test_ten_shared_noncancelling_lanes(self):
        used = set()
        for filename in ('ci.yml', 'deep-ci.yml'):
            header, jobs = blocks((ROOT / '.github/workflows' / filename).read_text())
            self.assertNotIn('cancel-in-progress: true', header)
            for name, job in jobs.items():
                # The reusable caller is not a runner; the unchanged outcome gate
                # is deliberately outside the qualification lane pool.
                if '    runs-on:' not in job or name == 'main-ci-required':
                    continue
                with self.subTest(workflow=filename, job=name):
                    self.assertIn('    concurrency:\n', job)
                    self.assertIn('      queue: max\n', job)
                    self.assertNotIn('cancel-in-progress: true', job)
                    if 'matrix.slot' in job:
                        self.assertIn(
                            "      group: ${{ format('oms-main-slot-{0}', matrix.slot) }}\n", job)
                        axis = re.search(r"^        (compiler|python-version): \[(.+)\]$", job, re.M)
                        self.assertIsNotNone(axis)
                        versions = re.findall(r"'([^']+)'", axis[2])
                        cells = re.findall(
                            r"^          - " + axis[1] + r": '([^']+)'\n            slot: (\d+)$",
                            job, re.M)
                        self.assertCountEqual([version for version, _ in cells], versions)
                        lanes = [int(lane) for _, lane in cells]
                        self.assertEqual(len(lanes), len(set(lanes)))
                        self.assertEqual(len(lanes), len(re.findall(r'^            slot:', job, re.M)))
                    else:
                        groups = re.findall(
                            r"^      group: \$\{\{ 'oms-main-slot-(\d+)' \}\}$", job, re.M)
                        self.assertEqual(len(groups), 1)
                        self.assertNotIn('      matrix:', job)
                        lanes = [int(groups[0])]
                    self.assertTrue(lanes)
                    self.assertTrue(all(1 <= lane <= 10 for lane in lanes))
                    used.update(lanes)
        self.assertEqual(used, set(range(1, 11)))


if __name__ == '__main__':
    unittest.main()