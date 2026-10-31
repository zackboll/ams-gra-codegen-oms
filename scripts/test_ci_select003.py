"""CI-SELECT-003: strict qualification discovery and selected job ownership."""
import copy
import importlib.util
from pathlib import Path
import tempfile
import unittest

import ci_policy
import ci_selection as selection
from ci_select003 import DEEP, FAST, RULES, SLEET, project, project_sleet

ROOT = selection.ROOT


def changed(path, status='M', mode='100644'):
    return {'paths': [path], 'status': status,
            'old_mode': '100644', 'new_mode': mode}


class ComponentSelection(unittest.TestCase):
    def setUp(self):
        self.policy = selection.inventory()
        self.all = set(selection.deep_jobs(self.policy))

    def test_python_component_modes_and_example(self):
        cases = {
            'python/qualification/fast/test_next.py': set(),
            'python/qualification/uci/test_next.py': {'real-uci-regressions'},
            'python/qualification/sleet/test_next.py': {'real-sleet'},
            'crates/cli/examples/python_future_probe.rs': {'msrv-real-uci', 'real-uci-regressions'},
        }
        for path, wanted in cases.items():
            with self.subTest(path=path):
                result = selection.select([changed(path)], self.policy)
                self.assertEqual(result['mode'], 'selective-pr')
                self.assertEqual(set(result['deep_jobs']), wanted)
                self.assertEqual({j for j, enabled in result['jobs'].items() if enabled},
                                 {'rust', 'python-model-backend'})
                self.assertEqual(result['deep_selected_cells'] + result['deep_skipped_cells'], 12)
                self.assertTrue({'python-model-backend-3.11', 'python-model-backend-3.13'}
                                <= set(result['cells']))

    def test_union_and_full_main(self):
        result = selection.select([changed('python/qualification/uci/test_more.py'),
                                   changed('python/qualification/sleet/test_more.py')], self.policy)
        self.assertEqual(set(result['deep_jobs']), {'real-uci-regressions', 'real-sleet'})
        self.assertEqual(result['mode'], 'selective-pr')
        main = selection.select([], self.policy, full_main=True)
        self.assertEqual(set(main['deep_jobs']), self.all)
        self.assertTrue(all(main['jobs'].values()))

    def test_unknown_scripts_runner_and_suspicious_paths_fail_broad(self):
        for path in (
            '.github/workflows/ci.yml', 'scripts/check-new-python-feature.py',
            'scripts/ci_selection.py', 'python/qualification/run.py',
            'python/qualification/fast/helper.py',
            'python/qualification/uci/test_next.py.bak',
            'python/qualification/sleet/test_nested/helper.py',
            'python/qualification/fixtures/new.sh',
            'crates/cli/examples/python_next/driver.rs',
            'crates/cli/examples/generic.rs',
        ):
            with self.subTest(path=path):
                result = selection.select([changed(path)], self.policy)
                self.assertEqual(result['mode'], 'full-pr')
                self.assertEqual(set(result['deep_jobs']), self.all)

    def test_unsafe_status_mode_and_uncertain_policy_fail_broad(self):
        path = 'python/qualification/fast/test_next.py'
        for changed_item in (changed(path, 'D'), changed(path, 'R100'),
                             changed(path, mode='120000'), changed(path, mode='100755')):
            with self.subTest(c=changed_item):
                self.assertEqual(selection.select([changed_item], self.policy)['mode'], 'full-pr')
        broken = copy.deepcopy(self.policy)
        broken['rules'][0]['deep_jobs'] = ['unknown']
        result = selection.select([changed(path)], broken)
        self.assertEqual(result['mode'], 'full-pr')


class RunnerContract(unittest.TestCase):
    def test_all_modes_have_fixed_registration_sources(self):
        name = ROOT / 'python/qualification/run.py'
        spec = importlib.util.spec_from_file_location('python_qualification', name)
        runner = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(runner)
        for mode in runner.MODES:
            _, sources, identities = runner.discover(mode)
            self.assertTrue(sources)
            self.assertGreaterEqual(len(identities), len(sources))
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            directory = root / 'fast'
            directory.mkdir()
            (directory / 'test_extra.py').write_text(
                'import unittest\nclass Check(unittest.TestCase):\n'
                '    def test_one(self): self.assertTrue(True)\n')
            _, files, identities = runner.discover('fast', root)
            self.assertEqual(len(files), 1)
            self.assertEqual(len(identities), 1)
            (directory / 'helper.py').write_text('pass\n')
            with self.assertRaisesRegex(ValueError, 'unregistered'):
                runner.discover('fast', root)

    def test_exact_hook_projection_and_negative_mutations(self):
        workflows = {p.name: p.read_text() for p in
                     (ROOT / '.github/workflows').glob('*.yml')}
        sleet = (ROOT / 'scripts/run-real-sleet-test.sh').read_text()
        projected = project(workflows)
        self.assertNotIn('python/qualification/run.py', projected['ci.yml'])
        self.assertNotIn('python/qualification/run.py', projected['deep-ci.yml'])
        self.assertNotIn('python/qualification/run.py', project_sleet(sleet))
        self.assertEqual(ci_policy.check(), 40)
        for workflow, hook in [('ci.yml', FAST), ('deep-ci.yml', DEEP)]:
            altered = dict(workflows)
            altered[workflow] = altered[workflow].replace(hook, '', 1)
            with self.assertRaises(ValueError):
                project(altered)
            altered = dict(workflows)
            altered[workflow] = altered[workflow].replace(hook, '', 1) + hook
            with self.assertRaises(ValueError):
                project(altered)
        for alteration in ('', SLEET.replace('"$interpreter"', 'true # "$interpreter"', 1)):
            with self.assertRaises(ValueError):
                project_sleet(sleet.replace(SLEET, alteration, 1))

    def test_rule_prefix_is_exact(self):
        policy = selection.inventory()
        self.assertEqual(policy['rules'][:len(RULES)], RULES)
        for mutation in ([], policy['rules'][1:]):
            damaged = copy.deepcopy(policy)
            damaged['rules'] = mutation
            self.assertNotEqual(damaged['rules'][:len(RULES)], RULES)


if __name__ == '__main__':
    unittest.main()
