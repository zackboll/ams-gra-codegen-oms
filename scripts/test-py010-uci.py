#!/usr/bin/env python3
"""Negative controls for exact PY-010 measured selection/generation evidence."""
import copy
import hashlib
import importlib.util
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('py010_gate', ROOT / 'scripts/check-py010-uci.py')
q = importlib.util.module_from_spec(spec)
spec.loader.exec_module(q)


def frozen():
    return json.loads((q.FIXTURE / 'after.json').read_text())['measurements']


class Gate(unittest.TestCase):
    def test_exact_and_historical(self):
        q.historical()
        rows = frozen()
        self.assertEqual(len(q.validate(rows, rows)), 24)
        before = json.loads((q.FIXTURE / 'before.json').read_text())
        self.assertTrue(before['completed'])
        self.assertEqual(len(before['measurements']), 24)
        for old, new in zip(before['measurements'], rows):
            self.assertEqual(old['selected_type_qnames'], new['selected_type_qnames'])
            self.assertEqual(old['closure_sha256'], new['closure_sha256'])
            self.assertEqual(old['exit'], 1)
            self.assertEqual(old['artifacts'], {})

    def test_truncated_duplicate_unknown(self):
        rows = frozen()
        for bad in [[], rows[:-1], rows + rows[:1], rows[:-1] + rows[:1]]:
            with self.assertRaises(RuntimeError):
                q.validate(bad, rows)

    def test_identity_and_closure(self):
        rows = frozen()
        for key in ['version', 'revision', 'root_sha256', 'message', 'world',
                    'contract_sha256', 'closure_sha256', 'selected_type_qnames']:
            bad = copy.deepcopy(rows)
            bad[0][key] = 'corrupt'
            with self.subTest(key=key), self.assertRaises(RuntimeError):
                q.validate(bad, rows)

    def test_verdict_diagnostics_and_artifacts(self):
        rows = frozen()
        for key, value in [('exit', 0), ('stdout', 'status: READY\n'), ('stderr', ''),
                           ('artifacts', {'model.py': 'unqualified'})]:
            bad = copy.deepcopy(rows)
            bad[0][key] = value
            with self.subTest(key=key), self.assertRaises(RuntimeError):
                q.validate(bad, rows)
        for key, value in [('exit', 0), ('stdout', 'generated'), ('stderr', '')]:
            bad = copy.deepcopy(rows)
            bad[0]['generation'][key] = value
            with self.subTest(key=key), self.assertRaises(RuntimeError):
                q.validate(bad, rows)

    def test_edited_oracle_cannot_qualify_ready_or_bad_closure(self):
        rows = frozen()
        for mutate in ['ready', 'closure']:
            bad = copy.deepcopy(rows)
            if mutate == 'ready':
                bad[0]['exit'] = 0
                bad[0]['stdout'] = bad[0]['stdout'].replace('NOT READY', 'READY')
            else:
                bad[0]['selected_type_qnames'] = []
            with self.subTest(mutate=mutate), self.assertRaises(RuntimeError):
                q.validate(bad, bad)

    def test_frozen_measurement_manifest(self):
        manifest = json.loads((q.FIXTURE / 'manifest.json').read_text())
        self.assertEqual(set(manifest['sha256']), {'before.json', 'after.json'})
        for name, digest in manifest['sha256'].items():
            self.assertEqual(hashlib.sha256((q.FIXTURE / name).read_bytes()).hexdigest(), digest, name)


if __name__ == '__main__':
    unittest.main()
