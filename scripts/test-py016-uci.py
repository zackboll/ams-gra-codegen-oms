#!/usr/bin/env python3
"""Independent historical source and adversarial PY-016 evidence controls."""
import copy
import hashlib
import importlib.util
import json
from pathlib import Path
import unittest
from unittest import mock

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('py016_gate', ROOT / 'scripts/check-py016-uci.py')
gate = importlib.util.module_from_spec(spec)
spec.loader.exec_module(gate)
EVIDENCE = ROOT / 'tests/fixtures/python-whitespace-visible/evidence/v1'


class Evidence(unittest.TestCase):
    def setUp(self):
        self.before, self.after = gate.frozen()
        self.authentication = json.loads((EVIDENCE / 'source-authentication.json').read_text())

    def test_exact_delta_and_manifest(self):
        summary = gate.validate(self.before, self.after)
        self.assertEqual(len(summary), 80)
        self.assertEqual(sum(row['ready'] for row in summary), 32)
        manifest = json.loads((EVIDENCE / 'manifest.json').read_text())
        self.assertEqual(set(manifest['sha256']), {p.relative_to(EVIDENCE).as_posix()
            for p in EVIDENCE.rglob('*') if p.is_file()} - {'manifest.json'})
        for name, digest in manifest['sha256'].items():
            self.assertEqual(hashlib.sha256((EVIDENCE / name).read_bytes()).hexdigest(), digest, name)

    def test_historical_authentication_without_future_current_sources(self):
        with mock.patch.object(gate, 'ROOT', Path('/unavailable-future-current-source')):
            self.assertEqual(len(gate.validate(self.before, self.after)), 80)
            for stage, value in [('before', self.before), ('after', self.after)]:
                gate.authenticate(value['source'], self.authentication[stage])

    def test_source_inventory_and_executable_tampering(self):
        for stage, measured in [('before', self.before), ('after', self.after)]:
            for variant in ['missing', 'extra', 'modified', 'substituted', 'commit', 'tree', 'generator_sha256', 'closure_tool_sha256']:
                bad = copy.deepcopy(measured['source'])
                inventory = bad['production_sha256']
                name = next(iter(inventory))
                if variant == 'missing':
                    del inventory[name]
                elif variant == 'extra':
                    inventory['extra.py'] = '0' * 64
                elif variant == 'modified':
                    inventory[name] = '0' * 64
                elif variant == 'substituted':
                    inventory['substitute.py'] = inventory.pop(name)
                else:
                    bad[variant] = '0' * 64
                with self.subTest(stage=stage, variant=variant), self.assertRaises(RuntimeError):
                    gate.authenticate(bad, self.authentication[stage])

    def test_incomplete_duplicate_forged_or_changed_selections(self):
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
            else:
                blocked = next(r for r in bad['measurements'] if 'RDMA_InitializeSetup' in r['message'])
                blocked['preexisting_generation'] = None
            with self.subTest(variant=variant), self.assertRaises(RuntimeError):
                gate.validate(self.before, bad)


if __name__ == '__main__':
    unittest.main()