#!/usr/bin/env python3
"""Independent historical authentication plus adversarial qualification controls."""
import copy
import hashlib
import importlib.util
import json
from pathlib import Path
import unittest
from unittest import mock

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('py013_gate', ROOT / 'scripts/check-py013-uci.py')
gate = importlib.util.module_from_spec(spec)
spec.loader.exec_module(gate)
EVIDENCE = ROOT / 'tests/fixtures/python-hexbinary/evidence/v1'


class Evidence(unittest.TestCase):
    def setUp(self):
        self.before = json.loads((EVIDENCE / 'before.json').read_text())
        self.after = json.loads((EVIDENCE / 'after.json').read_text())

    def test_exact_delta_and_manifest(self):
        self.assertEqual(len(gate.validate(self.before, self.after)), 64)
        manifest = json.loads((EVIDENCE / 'manifest.json').read_text())
        self.assertEqual(set(manifest['sha256']), {p.name for p in EVIDENCE.iterdir()} - {'manifest.json'})
        for name, digest in manifest['sha256'].items():
            self.assertEqual(hashlib.sha256((EVIDENCE / name).read_bytes()).hexdigest(), digest, name)

    def test_historical_authentication_without_current_sources(self):
        with mock.patch.object(gate, 'ROOT', Path('/unavailable-future-current-source')):
            self.test_exact_delta_and_manifest()

    def test_source_tampering_even_with_recomputed_manifest(self):
        for stage, value in [('before', self.before), ('after', self.after)]:
            for variant in ['missing', 'added', 'modified', 'substituted', 'commit', 'tree',
                            'generator_sha256', 'closure_tool_sha256']:
                bad = copy.deepcopy(value['source'])
                inventory = bad['production_sha256']
                name = next(iter(inventory))
                if variant == 'missing':
                    del inventory[name]
                elif variant == 'added':
                    inventory['crates/backend-python/src/extra.py'] = '0' * 64
                elif variant == 'modified':
                    inventory[name] = '0' * 64
                elif variant == 'substituted':
                    inventory['crates/backend-python/src/substitute.py'] = inventory.pop(name)
                else:
                    bad[variant] = '0' * 40
                # Authentication is independent of any manifest supplied by an author.
                hashlib.sha256(json.dumps(bad, sort_keys=True).encode()).hexdigest()
                with self.subTest(stage=stage, variant=variant), self.assertRaises(RuntimeError):
                    gate.authenticate(bad, stage)

    def test_incomplete_duplicate_forged_and_changed_selections(self):
        for variant in range(12):
            bad = copy.deepcopy(self.after)
            row = bad['measurements'][0]
            if variant == 0:
                bad['completed'] = False
            elif variant == 1:
                bad['measurements'].pop()
            elif variant == 2:
                bad['measurements'][-1] = row
            elif variant == 3:
                row['exit'] = 0
            elif variant == 4:
                row['selected_type_qnames'] = []
            elif variant == 5:
                row['generation']['stdout'] = 'READY'
            elif variant == 6:
                row['artifacts'] = {'model.py': 'unqualified'}
            elif variant == 7:
                row['root_sha256'] = 'wrong'
            elif variant == 8:
                row['contract_sha256'] = 'wrong'
            elif variant == 9:
                row['stdout'] = 'status: READY\n'
            elif variant == 10:
                bad['schema_includes']['2.5']['sha256'] = 'wrong'
            else:
                row['message'] = '{urn:wrong}PositionReport'
            with self.subTest(variant=variant), self.assertRaises(RuntimeError):
                gate.validate(self.before, bad)


if __name__ == '__main__':
    unittest.main()