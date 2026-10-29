#!/usr/bin/env python3
"""Must-execute adversarial controls for PY-008 versioned service evidence."""
import copy
import importlib.util
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('py008_gate', ROOT / 'scripts/check-py008-uci.py')
q = importlib.util.module_from_spec(spec)
spec.loader.exec_module(q)


def baseline():
    return json.loads((q.FIXTURE / 'after.json').read_text())['measurements']


def rows():
    return [dict(version=b['version'], revision=b['revision'], root_sha256=b['root_sha256'],
                 message=b['message'], world=b['world'], with_codec=b['codec'],
                 contract_sha256=b['contract_sha256'],
                 after={key: b[key] for key in ['exit', 'stdout', 'stderr']}) for b in baseline()]


class Gate(unittest.TestCase):
    def test_exact_current_and_historical(self):
        q.historical()
        self.assertEqual(len(q.validate(rows(), baseline())), 24)

    def test_missing_duplicate_and_unknown(self):
        for bad in [[], rows()[:-1], rows() + rows()[:1], rows()[:-1] + rows()[:1]]:
            with self.assertRaises(RuntimeError):
                q.validate(bad, baseline())

    def test_source_and_contract_identity(self):
        for key in ['version', 'revision', 'root_sha256', 'message', 'world', 'contract_sha256']:
            bad = rows()
            bad[0][key] = 'corrupt'
            with self.subTest(key=key), self.assertRaises(RuntimeError):
                q.validate(bad, baseline())
        bad, frozen = rows(), copy.deepcopy(baseline())
        bad[0]['contract_sha256'] = frozen[0]['contract_sha256'] = '0' * 64
        with self.assertRaises(RuntimeError):
            q.validate(bad, frozen)

    def test_process_and_blocker_delta(self):
        for key, value in [('exit', 0), ('exit', 2), ('stderr', ''), ('stdout', 'skipped')]:
            bad = rows()
            bad[0]['after'][key] = value
            with self.assertRaises(RuntimeError):
                q.validate(bad, baseline())
        bad = rows()
        bad[0]['after']['stdout'] = bad[0]['after']['stdout'].replace('model boundary:', 'wrong blocker:')
        with self.assertRaises(RuntimeError):
            q.validate(bad, baseline())

    def test_edited_oracle_cannot_qualify_ready_or_ambiguity(self):
        for old, new, index in [
            ('model status: NOT READY', 'model status: READY', 0),
            ('model status: NOT READY', 'model status: NOT READY\nmodel status: NOT READY', 0),
            ('model boundary:', 'missing boundary:', 0),
            ('\nstatus: NOT READY\n', '\nstatus: READY\n', 0),
            ('codec: not measured', 'codec status: READY', 1),
        ]:
            bad, frozen = rows(), copy.deepcopy(baseline())
            text = bad[index]['after']['stdout'].replace(old, new)
            bad[index]['after']['stdout'] = frozen[index]['stdout'] = text
            with self.subTest(old=old, new=new), self.assertRaises(RuntimeError):
                q.validate(bad, frozen)


if __name__ == '__main__':
    suite = unittest.defaultTestLoader.loadTestsFromTestCase(Gate)
    if suite.countTestCases() != 5:
        raise RuntimeError('missing gate controls')
    result = unittest.TextTestRunner(verbosity=2).run(suite)
    if not result.wasSuccessful() or result.skipped or result.testsRun != 5:
        raise SystemExit(1)
    print('PY008 UCI GATE CONTROLS: PASSED tests=5')