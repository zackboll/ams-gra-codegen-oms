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


    def test_ada_codec_packs_are_mandatory(self):
        import json
        inventory = json.loads((ROOT / 'scripts/ada_ci_pack/inventory.json').read_text())
        _, jobs = blocks((ROOT / '.github/workflows/ci.yml').read_text())
        former = {'ada-json-guard', *inventory['packs']['a'], *inventory['packs']['b']}
        self.assertTrue(former.isdisjoint(jobs))
        mandatory = re.search(r'^    needs: \[([^\n]+)\]$', jobs['main-ci-required'], re.M)
        self.assertIsNotNone(mandatory)
        required = set(mandatory[1].split(', '))
        self.assertTrue(former.isdisjoint(required))
        for pack, suites in inventory['packs'].items():
            name = 'ada-codec-pack-' + pack
            with self.subTest(pack=pack):
                self.assertIn(name, jobs)
                self.assertIn(name, required)
                self.assertIn('scripts/ada_ci_pack/run.py --pack ' + pack +
                              ' --workers ' + str(len(suites)), jobs[name])
                self.assertIn('scripts/ada_ci_pack/verify.py --pack ' + pack, jobs[name])
                self.assertIn('task087a-ada-json-guard-pack-' + pack, jobs[name])
                self.assertIn('if-no-files-found: error', jobs[name])
                self.assertIn('queue: max', jobs[name])
                for suite in suites:
                    self.assertIn('/' + suite + '/', jobs[name])
                    self.assertIn('name: ' + inventory['suites'][suite]['artifact']['name'], jobs[name])
                for suite in ['ada-json-guard', *suites]:
                    artifact = inventory['suites'][suite]['artifact']
                    directory = 'seed' if suite == 'ada-json-guard' else suite
                    prefix = '${{ runner.temp }}/' + name + '/' + directory
                    for pattern in artifact['path'].splitlines():
                        self.assertIn(pattern.replace('${{ runner.temp }}', prefix), jobs[name])


if __name__ == '__main__':
    unittest.main()