#!/usr/bin/env python3
"""PY-011 readiness evidence adversaries; frozen historical evidence stays intact."""
import copy
import hashlib
import importlib.util
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('py011_gate', ROOT / 'scripts/check-py011-uci.py')
gate = importlib.util.module_from_spec(spec)
spec.loader.exec_module(gate)
EVIDENCE = ROOT / 'tests/fixtures/python-nato-special-words/evidence/v1'


class ReadinessEvidence(unittest.TestCase):
    def setUp(self):
        self.before = json.loads((EVIDENCE / 'before.json').read_text())
        self.after = json.loads((EVIDENCE / 'after.json').read_text())

    def test_exact_delta(self):
        self.assertEqual(len(gate.validate(self.before, self.after)), 48)
        manifest = json.loads((EVIDENCE / 'manifest.json').read_text())
        for name, digest in manifest['sha256'].items():
            self.assertEqual(hashlib.sha256((EVIDENCE / name).read_bytes()).hexdigest(), digest, name)
        # Historical evidence describes PY-011, not every future backend. Anchor
        # its complete recorded source inventory rather than requiring current
        # production bytes to stay frozen. Works in shallow main CI checkouts.
        recorded = json.dumps(self.after['source']['production_sha256'],
                              sort_keys=True, separators=(',', ':')).encode()
        self.assertEqual(hashlib.sha256(recorded).hexdigest(),
                         '85d5db2b222282efb25b89809974e5dbee2a7893e96da02522995246e8961dc0')

    def test_incomplete_duplicate_and_forged_readiness(self):
        for variant in range(7):
            bad = copy.deepcopy(self.after)
            if variant == 0:
                bad['completed'] = False
            elif variant == 1:
                bad['measurements'].pop()
            elif variant == 2:
                bad['measurements'][-1] = bad['measurements'][0]
            elif variant == 3:
                bad['measurements'][0]['exit'] = 0
            elif variant == 4:
                bad['measurements'][0]['selected_type_qnames'] = []
            elif variant == 5:
                bad['measurements'][0]['generation']['stdout'] = 'generated'
            else:
                bad['measurements'][0]['artifacts'] = {'model.py': 'unqualified'}
            with self.subTest(variant=variant), self.assertRaises(RuntimeError):
                gate.validate(self.before, bad)


if __name__ == '__main__':
    unittest.main()