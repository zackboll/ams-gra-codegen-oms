"""CI-BENCH-004R5: exact reviewed Task101 timeout, not a relaxed policy."""
import copy
import hashlib
import json
import re
import unittest

import ci_policy
import ci_selection as selection


JOB = 'ada-real-uci-dlz-sleet'


class Task101TimeoutTests(unittest.TestCase):
    def setUp(self):
        self.policy = selection.inventory()
        self.workflows = {p.name: p.read_text() for p in
                          (selection.ROOT / '.github/workflows').glob('*.yml')}
        self.sleet = (selection.ROOT / 'scripts/run-real-sleet-test.sh').read_text()

    def record(self, policy):
        return next(r for r in policy['jobs'] if r['job_id'] == JOB)

    def test_reviewed_240_passes_without_other_inventory_changes(self):
        self.assertEqual(ci_policy.check(self.policy, self.workflows, self.sleet), 42)
        record = copy.deepcopy(self.record(self.policy))
        self.assertEqual(record['original_job']['timeout-minutes'], 240)
        self.assertEqual(record['expected_header'].count('timeout-minutes: 240'), 1)
        # Independently frozen from published 7df7c147: undo ONLY the two
        # approved timeout fields. Covers matrix, compiler identities, steps,
        # artifacts, prerequisites, concurrency and every other record field.
        record['original_job']['timeout-minutes'] = 120
        record['expected_header'] = record['expected_header'].replace(
            'timeout-minutes: 240', 'timeout-minutes: 120')
        self.assertEqual(hashlib.sha256(json.dumps(record, sort_keys=True).encode()).hexdigest(),
                         'f0e9a8d95445f64538967207e71b4f8923cdc6b42c272b38f515123e8a9f1ec3')

    def test_unapproved_or_missing_workflow_timeout_fails(self):
        for timeout in (120, 241, 480, None):
            with self.subTest(timeout=timeout):
                workflows = dict(self.workflows)
                body = ci_policy.blocks(workflows['deep-ci.yml'])[1][JOB]
                replacement = '' if timeout is None else f'    timeout-minutes: {timeout}\n'
                changed = body.replace('    timeout-minutes: 240\n', replacement, 1)
                self.assertNotEqual(body, changed)
                workflows['deep-ci.yml'] = workflows['deep-ci.yml'].replace(body, changed, 1)
                with self.assertRaisesRegex(ValueError,
                                            '^qualification runner/matrix/dependency policy changed: ' + JOB + '$'):
                    ci_policy.check(self.policy, workflows, self.sleet)

    def test_coordinated_inventory_edit_cannot_authorize_timeout(self):
        for timeout in (120, 241, None):
            with self.subTest(timeout=timeout):
                policy = copy.deepcopy(self.policy)
                record = self.record(policy)
                if timeout is None:
                    del record['original_job']['timeout-minutes']
                    record['expected_header'] = record['expected_header'].replace(
                        '\n    timeout-minutes: 240', '')
                else:
                    record['original_job']['timeout-minutes'] = timeout
                    record['expected_header'] = record['expected_header'].replace(
                        'timeout-minutes: 240', f'timeout-minutes: {timeout}')
                with self.assertRaisesRegex(ValueError, '^Task101 mandatory qualification header/payload changed$'):
                    ci_policy.check(policy, self.workflows, self.sleet)

    def test_other_deep_job_timeouts_remain_exact(self):
        jobs = ci_policy.blocks(self.workflows['deep-ci.yml'])[1]
        self.assertEqual({name: re.findall(r'^    timeout-minutes: (.+)$', body, re.M)
                          for name, body in jobs.items() if name != JOB}, {
            'ada-websocket-real-sleet': ['120'],
            'ada-repeated-choice-real-uci': ['60'],
            'real-uci-temporal-naming': ['240'],
            'real-uci-schema-ipv6': ['240'],
            'real-uci-integral-unicode': ['240'],
            'real-uci-regressions': ['240'],
            'real-uci-regressions-ascii': ['240'],
            'real-uci-regressions-alternating': ['240'],
            'msrv-real-uci': ['60'], 'real-sleet': ['60'],
            'ada-hexbinary-real': ['40'],
        })


if __name__ == '__main__':
    unittest.main()