"""PY-009 mandatory additive Fast/Deep/Sleet controls; no lane/gate changes."""
import unittest

import ci_policy
import ci_selection as selection
from py009_ci import FAST, DEEP, BASELINE, SLEET, BUILD
from py010_ci import BASELINE as CURRENT_BASELINE
from test_ci_select import change


class DateTimeIntegration(unittest.TestCase):
    def setUp(self):
        self.workflows = {p.name: p.read_text() for p in
                          (selection.ROOT / '.github/workflows').glob('*.yml')}
        self.sleet = (selection.ROOT / 'scripts/run-real-sleet-test.sh').read_text()
        self.policy = selection.inventory()

    def test_current_contract(self):
        self.assertEqual(ci_policy.check(self.policy, self.workflows, self.sleet), 40)

    def test_commands_cannot_be_removed_disabled_or_filtered(self):
        current_deep = DEEP.replace(' --output "$RUNNER_TEMP/py006-uci/py009"\n',
                                    ' --output "$RUNNER_TEMP/py006-uci/py009" ' + CURRENT_BASELINE + '\n')
        for workflow, addition in [('ci.yml', FAST), ('deep-ci.yml', current_deep + BUILD)]:
            for line in addition.splitlines(keepends=True):
                for replacement in ['', line.replace('          ', '          true # ', 1),
                                    line.rstrip() + ' --skip datetime\n']:
                    with self.subTest(line=line, replacement=replacement):
                        mutated = dict(self.workflows)
                        mutated[workflow] = mutated[workflow].replace(line, replacement, 1)
                        with self.assertRaises(ValueError):
                            ci_policy.check(self.policy, mutated, self.sleet)

    def test_successor_baseline_and_shared_broker_loop_required(self):
        mutated = dict(self.workflows)
        mutated['deep-ci.yml'] = mutated['deep-ci.yml'].replace(CURRENT_BASELINE, '', 1)
        with self.assertRaises(ValueError):
            ci_policy.check(self.policy, mutated, self.sleet)
        for replacement in ['', SLEET.replace('bash ', 'true # bash '), SLEET.rstrip() + ' --skip\n']:
            with self.assertRaises(ValueError):
                ci_policy.check(self.policy, self.workflows, self.sleet.replace(SLEET, replacement))

    def test_all_task_surfaces_select_both_python_versions_and_deep_owners(self):
        paths = ['crates/cli/tests/python_datetime.rs', 'crates/cli/examples/python_datetime_closure.rs',
                 'scripts/check-py009-uci.py', 'scripts/measure-py009-uci.py', 'scripts/test-py009-uci.py',
                 'scripts/check-py009-sleet.sh', 'scripts/py009_ci.py',
                 'scripts/test_ci_select_py009.py',
                 'python/oms-lacal-runtime/tests/test_datetime_sleet.py']
        paths += [str(p.relative_to(selection.ROOT)) for directory in
                  ['crates/backend-python/src', 'tests/fixtures/python-datetime']
                  for p in (selection.ROOT / directory).glob('*') if p.is_file()]
        for path in paths:
            with self.subTest(path=path):
                result = selection.select([change(path)], self.policy)
                self.assertTrue({'python-model-backend-3.11', 'python-model-backend-3.13'} <= set(result['cells']))
                self.assertTrue({'real-sleet', 'real-uci-regressions'} <= set(result['deep_jobs']))

    def test_commands_must_execute_in_owning_job_and_interpreter_loop(self):
        for workflow, addition in [('ci.yml', FAST), ('deep-ci.yml', DEEP)]:
            mutated = dict(self.workflows)
            mutated[workflow] = mutated[workflow].replace(addition, '', 1) + addition
            with self.assertRaises(ValueError):
                ci_policy.check(self.policy, mutated, self.sleet)
        moved = self.sleet.replace(SLEET, '', 1) + SLEET
        with self.assertRaises(ValueError):
            ci_policy.check(self.policy, self.workflows, moved)


if __name__ == '__main__':
    unittest.main()
