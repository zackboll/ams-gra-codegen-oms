#!/usr/bin/env python3
"""Fail-closed adversaries for the new evidence, without rewriting old oracles."""
import copy
import hashlib
import importlib.util
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('py012_gate', ROOT / 'scripts/check-py012-uci.py')
gate = importlib.util.module_from_spec(spec)
spec.loader.exec_module(gate)
EVIDENCE = ROOT / 'tests/fixtures/python-unbounded-repetition/evidence/v1'


class ReadinessEvidence(unittest.TestCase):
    def setUp(self):
        self.before = json.loads((EVIDENCE / 'before.json').read_text())
        self.after = json.loads((EVIDENCE / 'after.json').read_text())

    def test_exact_delta(self):
        self.assertEqual(len(gate.validate(self.before, self.after)), 48)
        for name, digest in self.after['source']['production_sha256'].items():
            self.assertEqual(hashlib.sha256((ROOT / name).read_bytes()).hexdigest(), digest, name)
        manifest = json.loads((EVIDENCE / 'manifest.json').read_text())
        for name, digest in manifest['sha256'].items():
            self.assertEqual(hashlib.sha256((EVIDENCE / name).read_bytes()).hexdigest(), digest, name)

    def test_incomplete_duplicate_and_forged_evidence(self):
        for variant in range(11):
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
            elif variant == 6:
                bad['measurements'][0]['artifacts'] = {'model.py': 'unqualified'}
            elif variant == 7:
                bad['measurements'][0]['root_sha256'] = 'wrong'
            elif variant == 8:
                bad['measurements'][0]['contract_sha256'] = 'different'
            elif variant == 9:
                bad['measurements'][0]['stdout'] = 'status: READY\n'
            else:
                bad['schema_includes']['2.5']['sha256'] = 'wrong'
            with self.subTest(variant=variant), self.assertRaises(RuntimeError):
                gate.validate(self.before, bad)


if __name__ == '__main__':
    unittest.main()