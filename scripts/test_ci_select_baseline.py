"""Offline regression for the reviewed baseline-only execution budget."""
import unittest

import ci_policy
import ci_selection as selection


class BaselineBudget(unittest.TestCase):
    def setUp(self):
        self.workflows = {p.name: p.read_text() for p in
                          (selection.ROOT / '.github/workflows').glob('*.yml')}
        self.baseline = ci_policy.blocks(self.workflows['ci.yml'])[1]['baseline']

    def check_mutation(self, baseline):
        workflows = dict(self.workflows)
        self.assertEqual(workflows['ci.yml'].count(self.baseline), 1)
        workflows['ci.yml'] = workflows['ci.yml'].replace(self.baseline, baseline)
        with self.assertRaisesRegex(ValueError, 'mandatory baseline/final gate changed: baseline'):
            ci_policy.check(workflows=workflows)

    def test_reviewed_twenty_minute_budget_is_accepted(self):
        self.assertEqual(ci_policy.field(self.baseline, 'timeout-minutes'), ['20'])
        self.assertEqual(ci_policy.check(workflows=self.workflows), 48)

    def test_other_timeout_values_and_omission_are_rejected(self):
        for replacement in ('', '    timeout-minutes: 10\n',
                            '    timeout-minutes: 19\n', '    timeout-minutes: 21\n',
                            '    timeout-minutes: 360\n'):
            with self.subTest(replacement=replacement):
                self.check_mutation(self.baseline.replace('    timeout-minutes: 20\n', replacement))

    def test_every_mandatory_run_line_cannot_be_removed_or_masked(self):
        # Includes the entire policy/adversarial chain and pinned actionlint
        # acquisition/hash check; these are workflow mutations, not executions.
        commands = [line for line in self.baseline.splitlines(keepends=True)
                    if line.startswith('          ') and line.strip()]
        self.assertTrue(any('bash scripts/test-check-ci-split.sh' in c for c in commands))
        self.assertTrue(any('sha256sum --check --strict' in c for c in commands))
        for command in commands:
            for replacement in ('', '          true # disabled mandatory command\n'):
                with self.subTest(command=command.strip(), replacement=replacement):
                    self.check_mutation(self.baseline.replace(command, replacement, 1))


if __name__ == '__main__':
    unittest.main()