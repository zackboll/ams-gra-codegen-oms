#!/usr/bin/env python3
"""Independent negative controls for the separate PY-018 acceptance gates."""
import copy
import importlib.util
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]


def module(name, filename):
    spec = importlib.util.spec_from_file_location(name, ROOT / 'scripts' / filename)
    value = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(value)
    return value


compat = module('py018_compat_test', 'check-py018-compatibility.py')
uci = module('py018_uci_test', 'check-py018-uci.py')


class Acceptance(unittest.TestCase):
    def test_historical_success_is_not_rewritten(self):
        _, after = compat.prior.frozen()
        result = compat.validate(after, after)
        self.assertEqual(result['before_ready'], 40)
        self.assertEqual(result['after_ready'], 40)

    def test_regression_hash_closure_pin_or_partial_campaign_is_rejected(self):
        _, after = compat.prior.frozen()
        for mutation in range(9):
            bad = copy.deepcopy(after)
            row = bad['measurements'][0]
            if mutation == 0:
                row['exit'] = 1
            elif mutation == 1:
                row['artifacts']['model.py'] = '0' * 64
            elif mutation == 2:
                row['selected_type_qnames'].pop()
            elif mutation == 3:
                row['root_sha256'] = '0' * 64
            elif mutation == 4:
                bad['completed'] = False
            elif mutation == 5:
                bad['measurements'].pop()
            elif mutation == 6:
                bad['measurements'][-1] = row
            elif mutation == 7:
                row['generation']['exit'] = 1
            else:
                bad['schema_includes']['2.5']['sha256'] = 'wrong'
            with self.subTest(mutation=mutation), self.assertRaises(RuntimeError):
                compat.validate(after, bad)

    def test_unexecuted_ao_cannot_be_promoted_ready(self):
        rows = [dict(version=v, world=w, codec=c, exit=1, artifacts={},
                     selected_type_qnames=['NITF_PackingPlanPET'], closure_sha256='same',
                     contract_sha256='same', stderr='',
                     stdout=('abstract value representations are unsupported: NITF_PackingPlanPET'
                             if w == 'closed-schema' else
                             'abstract value NITF_PackingPlanPET is not closed under open-extensions generation'))
                for v in ['2.5', '2.6'] for w in ['closed-schema', 'open-extensions'] for c in [False, True]]
        uci.validate_delta(rows, rows)
        for mutation in range(4):
            bad = copy.deepcopy(rows)
            if mutation == 0:
                bad[0]['exit'] = 0
            elif mutation == 1:
                bad[0]['stdout'] = 'status: READY'
            elif mutation == 2:
                bad[0]['selected_type_qnames'] = []
            else:
                bad.pop()
            with self.subTest(mutation=mutation), self.assertRaises(RuntimeError):
                uci.validate_delta(rows, bad)


if __name__ == '__main__':
    unittest.main()