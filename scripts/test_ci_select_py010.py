"""PY-010 mandatory additive Fast/Deep/Sleet controls; no lane/gate changes."""
import unittest

import ci_policy
import ci_selection as selection
from py010_ci import FAST, DEEP, DEEP_SETUP, BASELINE, SLEET, SLEET_ARTIFACT
from test_ci_select import change


class VisibleAsciiIntegration(unittest.TestCase):
    def setUp(self):
        self.workflows = {p.name: p.read_text() for p in
                          (selection.ROOT / '.github/workflows').glob('*.yml')}
        self.sleet = (selection.ROOT / 'scripts/run-real-sleet-test.sh').read_text()
        self.policy = selection.inventory()

    def test_current_contract(self):
        self.assertEqual(ci_policy.check(self.policy, self.workflows, self.sleet), 40)

    def test_commands_cannot_be_removed_disabled_or_filtered(self):
        for workflow, addition in [('ci.yml', FAST), ('deep-ci.yml', DEEP)]:
            for line in addition.splitlines(keepends=True):
                for replacement in ['', line.replace('          ', '          true # ', 1),
                                    line.rstrip() + ' --skip datetime\n']:
                    if replacement == line:
                        continue
                    with self.subTest(line=line, replacement=replacement):
                        mutated = dict(self.workflows)
                        mutated[workflow] = mutated[workflow].replace(line, replacement, 1)
                        with self.assertRaises(ValueError):
                            ci_policy.check(self.policy, mutated, self.sleet)

    def test_sleet_evidence_cannot_be_removed_disabled_or_moved(self):
        for replacement in ['', SLEET_ARTIFACT.replace('always()', 'false'),
                            SLEET_ARTIFACT.replace('if-no-files-found: error', 'if-no-files-found: ignore')]:
            mutated = dict(self.workflows)
            mutated['deep-ci.yml'] = mutated['deep-ci.yml'].replace(SLEET_ARTIFACT, replacement, 1)
            with self.assertRaises(ValueError):
                ci_policy.check(self.policy, mutated, self.sleet)
        mutated = dict(self.workflows)
        mutated['deep-ci.yml'] = mutated['deep-ci.yml'].replace(SLEET_ARTIFACT, '', 1) + SLEET_ARTIFACT
        with self.assertRaises(ValueError):
            ci_policy.check(self.policy, mutated, self.sleet)
        for replacement in ['', DEEP_SETUP.replace('3.13', '3.12')]:
            mutated = dict(self.workflows)
            mutated['deep-ci.yml'] = mutated['deep-ci.yml'].replace(DEEP_SETUP, replacement, 1)
            with self.assertRaises(ValueError):
                ci_policy.check(self.policy, mutated, self.sleet)

    def test_final_gate_requires_fresh_unique_sleet_artifact(self):
        import copy
        from test_ci_select import GateTests
        control = GateTests()
        control.setUp()
        control.validate()
        index = next(n for n, artifact in enumerate(control.artifacts) if artifact['name'] == 'py010-sleet')
        healthy = copy.deepcopy(control.artifacts[index])
        for mutation in [{'expired': True}, {'size_in_bytes': 0}, {'created_at': '2000-01-01T00:00:00Z'}]:
            control.artifacts[index].update(mutation)
            with self.assertRaisesRegex(ValueError, 'required evidence'):
                control.validate()
            control.artifacts[index] = copy.deepcopy(healthy)
        control.artifacts.pop(index)
        with self.assertRaisesRegex(ValueError, 'required evidence'):
            control.validate()
        control.artifacts.extend([healthy, healthy])
        with self.assertRaisesRegex(ValueError, 'required evidence'):
            control.validate()

    def test_successor_baseline_and_shared_broker_loop_required(self):
        mutated = dict(self.workflows)
        mutated['deep-ci.yml'] = mutated['deep-ci.yml'].replace(BASELINE, '', 1)
        with self.assertRaises(ValueError):
            ci_policy.check(self.policy, mutated, self.sleet)
        for replacement in ['', SLEET.replace('bash ', 'true # bash '), SLEET.rstrip() + ' --skip\n']:
            with self.assertRaises(ValueError):
                ci_policy.check(self.policy, self.workflows, self.sleet.replace(SLEET, replacement))

    def test_all_task_surfaces_select_both_python_versions_and_deep_owners(self):
        paths = ['crates/cli/tests/python_visible_ascii.rs', 'crates/backend-python/tests/visible_ascii.rs',
                 'crates/cli/examples/python_visible_ascii_profiles.rs', 'scripts/check-py010-parity.py',
                 'scripts/check-py010-uci.py', 'scripts/measure-py010-uci.py', 'scripts/test-py010-uci.py',
                 'scripts/check-py010-sleet.sh', 'scripts/py010_ci.py',
                 'scripts/test_ci_select_py010.py',
                 'python/oms-lacal-runtime/tests/test_visible_ascii_sleet.py']
        paths += [str(p.relative_to(selection.ROOT)) for directory in
                  ['crates/backend-python/src', 'tests/fixtures/python-visible-ascii']
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
