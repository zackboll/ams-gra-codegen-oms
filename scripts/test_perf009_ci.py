"""Production integration contracts, not alternate test-only execution adapters."""
import importlib.util
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

import perf009_ci
from perf009_provider import verify_provider

ROOT = Path(__file__).resolve().parents[1]


class Production(unittest.TestCase):
    def test_exact_two_production_owners(self):
        from workflow_text import blocks
        _, jobs = blocks((ROOT / '.github/workflows/ci.yml').read_text())
        inventory = json.loads((ROOT / 'scripts/ada_ci_pack/inventory.json').read_text())
        commands = '\n'.join(s['run'] for s in inventory['suites']['ada-hexbinary-codec']['steps'])
        self.assertIn('scripts/perf009-ci.py --family portable --compiler 16', commands)
        self.assertIn('scripts/perf009-ci.py --family portable --compiler 14', jobs['ada-float32-codec'])
        self.assertIn('scripts/perf005-ci.py --compilers 14 16', jobs['ada-float32-codec'])
        self.assertIn('sudo apt-get install -y strace', jobs['ada-codec-pack-a'])
        self.assertIn('--workers 4', jobs['ada-codec-pack-a'])

    def test_partial_inventory_and_missing_private_state_are_rejected(self):
        for extra in (['--profiles', 'oam'], ['--policies', 'normal'], ['--red'], []):
            args = ['python3', str(ROOT / 'scripts/check-task093-helper-demand.py'),
                    '--family', 'portable', '--compiler', '14', '--scratch', '/unused',
                    '--generator', '/unused', '--portable-provider-reuse', *extra]
            result = subprocess.run(args, capture_output=True)
            self.assertEqual(result.returncode, 2)
            self.assertIn(b'complete original portable inventory', result.stderr)

    def test_no_build_fallback_on_mandatory_verification_failure(self):
        campaign = perf009_ci.Campaign(Path('/private/campaign'), '14', Path('/private/state'), 1, True)
        with patch.object(campaign.pilot, 'build', side_effect=RuntimeError('corrupt provider')), \
                patch.object(campaign, 'checked') as ordinary:
            with self.assertRaisesRegex(RuntimeError, 'corrupt provider'):
                campaign(['alr', '-n', 'build', '--', '-f'], Path('/consumer'), Path('/build'))
            ordinary.assert_not_called()

    def test_full_inventory_remains_five_names_two_namespaces_three_policies(self):
        from perf009_portable import cells
        self.assertEqual(len(cells()), 30)
        self.assertEqual(len(set(cells())), 30)
        self.assertEqual({p for _, _, p in cells()}, {'normal', 'Check', 'Ignore'})
        self.assertEqual({p for p, _, _ in cells()}, {'oam', 'qualified'})


if __name__ == '__main__':
    unittest.main()