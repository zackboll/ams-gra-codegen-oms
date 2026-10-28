#!/usr/bin/env python3
"""Offline adversarial controls for the strict PY-007 service evidence gate."""
import copy
import importlib.util
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('qualification', ROOT / 'scripts/check-py007-uci.py')
q = importlib.util.module_from_spec(spec)
spec.loader.exec_module(q)
BASELINE = json.loads((q.FIXTURE / 'after.json').read_text())['measurements']


def rows():
    return [dict(version=b['version'], message=b['message'], world=b['world'],
                 with_codec=b['codec'], revision=b['revision'], root_sha256=b['root_sha256'],
                 contract_sha256=b['contract_sha256'],
                 after={k: b[k] for k in ['exit', 'stdout', 'stderr']}) for b in BASELINE]


class QualificationControls(unittest.TestCase):
    def test_exact_measured_campaign(self):
        summaries = q.validate(rows(), BASELINE)
        self.assertEqual(len(summaries), 24)
        self.assertEqual({s['process_exit'] for s in summaries}, {1})

    def test_missing_and_duplicate(self):
        for altered in [rows()[:-1], rows() + rows()[:1], rows()[:-1] + rows()[:1], []]:
            with self.subTest(count=len(altered)), self.assertRaises(RuntimeError):
                q.validate(altered, BASELINE)

    def test_identity_and_process_tampering(self):
        for key, bad in [('revision', 'floating'), ('root_sha256', '0' * 64),
                         ('contract_sha256', '1' * 64), ('message', 'NotReal'),
                         ('world', 'implicit')]:
            altered = rows()
            altered[0][key] = bad
            with self.subTest(key=key), self.assertRaises(RuntimeError):
                q.validate(altered, BASELINE)
        for key, bad in [('exit', 0), ('exit', 2), ('stderr', 'skipped'), ('stdout', 'PASSED')]:
            altered = rows()
            altered[0]['after'][key] = bad
            with self.subTest(key=key), self.assertRaises(RuntimeError):
                q.validate(altered, BASELINE)

    def test_edited_oracle_cannot_qualify_ready_or_ambiguity(self):
        for old, new in [('model status: NOT READY', 'model status: READY'),
                         ('model status: NOT READY', ''),
                         ('model boundary:', 'missing boundary:'),
                         ('model status: NOT READY', 'model status: NOT READY\nmodel status: NOT READY'),
                         ('\nstatus: NOT READY\n', '\nstatus: READY\n'),
                         ('codec: not measured', 'codec status: READY')]:
            altered, baseline = rows(), copy.deepcopy(BASELINE)
            index = 1 if old.startswith('codec') else 0
            text = altered[index]['after']['stdout'].replace(old, new)
            altered[index]['after']['stdout'] = baseline[index]['stdout'] = text
            with self.subTest(old=old, new=new), self.assertRaises(RuntimeError):
                q.validate(altered, baseline)

    def test_historical_fixtures_unchanged(self):
        for name, digest in q.HISTORICAL.items():
            self.assertEqual(q.driver.digest(ROOT / 'tests/fixtures/python-header-profiles' / name), digest)


if __name__ == '__main__':
    suite = unittest.defaultTestLoader.loadTestsFromTestCase(QualificationControls)
    if suite.countTestCases() != 5:
        raise RuntimeError('qualification controls missing')
    result = unittest.TextTestRunner(verbosity=2).run(suite)
    if not result.wasSuccessful() or result.skipped or result.testsRun != 5:
        raise SystemExit(1)
    print('PY007 UCI GATE CONTROLS: PASSED tests=5')