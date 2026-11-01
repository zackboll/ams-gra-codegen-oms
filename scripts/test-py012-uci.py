#!/usr/bin/env python3
"""Fail-closed adversaries for the new evidence, without rewriting old oracles."""
import copy
import hashlib
import importlib.util
import json
from pathlib import Path
import unittest
from unittest import mock

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('py012_gate', ROOT / 'scripts/check-py012-uci.py')
gate = importlib.util.module_from_spec(spec)
spec.loader.exec_module(gate)
EVIDENCE = ROOT / 'tests/fixtures/python-unbounded-repetition/evidence/v1'


def authenticate_historical_source(source):
    # PY-012's AFTER source is immutable history, not the current backend. Pin
    # the complete canonical inventory independently of the evidence manifest.
    # No source files or Git objects are needed in shallow main CI checkouts.
    recorded = json.dumps(source['production_sha256'],
                          sort_keys=True, separators=(',', ':')).encode()
    gate.driver.require(hashlib.sha256(recorded).hexdigest() ==
                        '5b9709087f851c80a80176065a0bb5ef112e2b4c4b157d40e2c916f258e5847e',
                        'historical production inventory changed')
    gate.driver.require(source['commit'] == 'a062b1a96874fb77c69305ffcdca14a97c5e147f'
                        and source['tree'] == '39ca54f00b80b3bb3a02df0b432435a01a854889'
                        and source['dirty'] is False, 'historical source provenance changed')


class ReadinessEvidence(unittest.TestCase):
    def setUp(self):
        self.before = json.loads((EVIDENCE / 'before.json').read_text())
        self.after = json.loads((EVIDENCE / 'after.json').read_text())

    def test_exact_delta(self):
        self.assertEqual(len(gate.validate(self.before, self.after)), 48)
        authenticate_historical_source(self.after['source'])
        manifest = json.loads((EVIDENCE / 'manifest.json').read_text())
        for name, digest in manifest['sha256'].items():
            self.assertEqual(hashlib.sha256((EVIDENCE / name).read_bytes()).hexdigest(), digest, name)

    def test_historical_evidence_without_current_sources(self):
        # Even an unavailable current source tree must not invalidate history.
        # The evidence remains available; no Git history lookup is performed.
        with mock.patch(__name__ + '.ROOT', Path('/unavailable-current-python-backend')):
            self.test_exact_delta()

    def test_tampered_historical_inventory(self):
        for variant in ('missing', 'added', 'modified', 'substituted'):
            bad = copy.deepcopy(self.after['source'])
            recorded = bad['production_sha256']
            name = next(iter(recorded))
            if variant == 'missing':
                del recorded[name]
            elif variant == 'added':
                recorded['crates/backend-python/src/extra.py'] = '0' * 64
            elif variant == 'modified':
                recorded[name] = '0' * 64
            else:
                recorded['crates/backend-python/src/substituted.py'] = recorded.pop(name)
            with self.subTest(variant=variant), self.assertRaisesRegex(
                    RuntimeError, 'historical production inventory changed'):
                authenticate_historical_source(bad)

    def test_tampered_historical_provenance(self):
        for name, value in [('commit', '0' * 40), ('tree', '0' * 40), ('dirty', True)]:
            bad = copy.deepcopy(self.after['source'])
            bad[name] = value
            with self.subTest(field=name), self.assertRaisesRegex(
                    RuntimeError, 'historical source provenance changed'):
                authenticate_historical_source(bad)

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