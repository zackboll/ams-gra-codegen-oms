#!/usr/bin/env python3
"""PY-017 frozen successor integrity and fail-closed evidence controls."""
import copy
import hashlib
import importlib.util
import json
from pathlib import Path
import unittest
from unittest import mock

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('py017_gate', ROOT / 'scripts/check-py017-uci.py')
gate = importlib.util.module_from_spec(spec)
spec.loader.exec_module(gate)
EVIDENCE = ROOT / 'tests/fixtures/python-network-addresses/evidence/v1'


class Evidence(unittest.TestCase):
    def setUp(self):
        self.before, self.after = [json.loads((EVIDENCE / (stage + '.json')).read_text())
                                   for stage in ['before', 'after']]

    def test_exact_delta_and_manifest(self):
        summary = gate.validate(self.before, self.after)
        self.assertEqual(len(summary), 80)
        self.assertEqual(sum(r['ready'] for r in summary if r['stage'] == 'before'), 32)
        self.assertEqual(sum(r['ready'] for r in summary if r['stage'] == 'after'), 40)
        manifest = json.loads((EVIDENCE / 'manifest.json').read_text())
        self.assertEqual(set(manifest['sha256']), {p.relative_to(EVIDENCE).as_posix()
            for p in EVIDENCE.rglob('*') if p.is_file()} - {'manifest.json'})
        for name, digest in manifest['sha256'].items():
            self.assertEqual(hashlib.sha256((EVIDENCE / name).read_bytes()).hexdigest(), digest, name)

    def test_historical_oracles_do_not_depend_on_future_live_source(self):
        with mock.patch.object(gate, 'ROOT', Path('/unavailable-future-current-source')):
            self.assertEqual(len(gate.validate(self.before, self.after)), 80)
        self.assertEqual(len(gate.historical.validate(*gate.historical.frozen())), 80)

    def test_missing_duplicate_forged_or_changed_selections_fail(self):
        for variant in range(13):
            bad = copy.deepcopy(self.after)
            row = bad['measurements'][0]
            if variant == 0:
                bad['completed'] = False
            elif variant == 1:
                bad['measurements'].pop()
            elif variant == 2:
                bad['measurements'][-1] = row
            elif variant == 3:
                row['exit'] = 1
            elif variant == 4:
                row['artifacts'] = {}
            elif variant == 5:
                row['selected_type_qnames'] = []
            elif variant == 6:
                row['root_sha256'] = 'wrong'
            elif variant == 7:
                row['contract_sha256'] = 'wrong'
            elif variant == 8:
                row['stdout'] = 'READY'
            elif variant == 9:
                bad['schema_includes']['2.5']['sha256'] = 'wrong'
            elif variant == 10:
                row['message'] = '{urn:wrong}PositionReport'
            elif variant == 11:
                row['artifacts']['model.py'] = '0' * 64
            else:
                row['generation']['exit'] = 1
            with self.subTest(variant=variant), self.assertRaises(RuntimeError):
                gate.validate(self.before, bad)


if __name__ == '__main__':
    unittest.main()