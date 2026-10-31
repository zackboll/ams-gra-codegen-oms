"""CI-SELECT-003: strict qualification discovery and selected job ownership."""
import copy
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

import ci_policy
import ci_selection as selection
from ci_select003 import DEEP, FAST, RULES, SLEET, project, project_sleet
from py010_ci import project as project_py010, project_sleet as project_py010_sleet

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
            'crates/cli/examples/python_future_probe.rs': {
                'msrv-real-uci', 'real-sleet', 'real-uci-regressions'},
            'crates/cli/examples/python_datetime_closure.rs': {
                'msrv-real-uci', 'real-sleet', 'real-uci-regressions'},
        }
        for path, wanted in cases.items():
            with self.subTest(path=path):
                result = selection.select([changed(path)], self.policy)
                self.assertEqual(result['mode'], 'selective-pr')
                self.assertEqual(set(result['deep_jobs']), wanted)
                self.assertEqual({j for j, enabled in result['jobs'].items() if enabled},
                                 {'rust', 'python-model-backend'})
                # Current main adds Task101's two mandatory Deep compiler cells;
                # Python ownership remains bounded and does not select either.
                self.assertEqual(result['deep_selected_cells'] + result['deep_skipped_cells'], 14)
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

    def test_main_task101_ownership_and_compiler_cells_are_preserved(self):
        record = selection.deep_jobs(self.policy)['ada-real-uci-dlz-sleet']
        self.assertEqual([(c['variant'], c['lane'], c['artifacts']) for c in record['cells']],
                         [({'compiler': c}, lane, ['task101-ada-real-uci-dlz-' + c])
                          for c, lane in [('14', 3), ('16', 5)]])
        cases = {
            'tests/ada-real-uci-dlz/dlz_consumer.adb': {'ada-real-uci-dlz-sleet'},
            'ada/oms-lacal-runtime/websocket/oms_lacal_websocket.ads': {
                'ada-real-uci-dlz-sleet', 'ada-websocket-real-sleet'},
        }
        for path, wanted in cases.items():
            with self.subTest(path=path):
                result = selection.select([changed(path)], self.policy)
                self.assertEqual(result['mode'], 'selective-pr')
                self.assertEqual(set(result['deep_jobs']), wanted)

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
        workflows = project_py010(workflows)
        sleet = project_py010_sleet(sleet)
        projected = project(workflows)
        self.assertNotIn('python/qualification/run.py', projected['ci.yml'])
        self.assertNotIn('python/qualification/run.py', projected['deep-ci.yml'])
        self.assertNotIn('python/qualification/run.py', project_sleet(sleet))
        self.assertEqual(ci_policy.check(), 42)
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

    def test_python_example_deep_owner_removal_is_rejected(self):
        policy = selection.inventory()
        rule = next(r for r in policy['rules'] if r['category'] == 'python-cli-example')
        rule['deep_jobs'].remove('real-sleet')
        with self.assertRaisesRegex(ValueError,
                                    '^CI-SELECT-003 reviewed Python Deep ownership changed$'):
            ci_policy.check(policy=policy)

    def test_main_task101_deep_owner_removal_is_rejected(self):
        policy = selection.inventory()
        rule = next(r for r in policy['rules'] if r['category'] == 'ada-websocket-implementation')
        rule['deep_jobs'].remove('ada-real-uci-dlz-sleet')
        with self.assertRaisesRegex(ValueError, '^reviewed Deep dependency ownership changed$'):
            ci_policy.check(policy=policy)


class RunnerExecution(unittest.TestCase):
    """Invoke the real CLI in isolated copies; no production bypass switches."""

    PASS = ('import unittest\nclass Check(unittest.TestCase):\n'
            '    def test_pass(self): self.assertEqual(2 + 2, 4)\n')
    CASES = {
        'pass': (PASS, None),
        'decorator_skip': ('import unittest\nclass Check(unittest.TestCase):\n'
                           '    @unittest.skip("mandatory decorator skip")\n'
                           '    def test_required(self): self.fail("must execute")\n', 'skipped'),
        'runtime_skip': ('import unittest\nclass Check(unittest.TestCase):\n'
                         '    def test_required(self): self.skipTest("mandatory runtime skip")\n',
                         'skipped'),
        'expected_failure': ('import unittest\nclass Check(unittest.TestCase):\n'
                             '    @unittest.expectedFailure\n'
                             '    def test_required(self): self.fail("must pass")\n',
                             'expected_failures'),
        'pass_and_expected_failure': (PASS + '    @unittest.expectedFailure\n'
                                     '    def test_required(self): self.fail("must pass")\n',
                                     'expected_failures'),
        'pass_and_skip': (PASS + '    @unittest.skip("mandatory mixed skip")\n'
                          '    def test_required(self): self.fail("must execute")\n', 'skipped'),
        'unexpected_success': ('import unittest\nclass Check(unittest.TestCase):\n'
                               '    @unittest.expectedFailure\n'
                               '    def test_required(self): self.assertTrue(True)\n',
                               'unexpected_successes'),
        'assertion_failure': ('import unittest\nclass Check(unittest.TestCase):\n'
                              '    def test_required(self): self.fail("ordinary failure")\n',
                              'failures'),
        'exception': ('import unittest\nclass Check(unittest.TestCase):\n'
                      '    def test_required(self): raise RuntimeError("ordinary error")\n',
                      'errors'),
        'missing_module': ('import missing_qualification_module\n', 'discovery errors'),
        'no_sources': ('', 'no Python qualification test sources'),
        'empty_module': ('import unittest\n', 'incomplete/duplicate'),
        'empty_suite': (PASS + '\ndef load_tests(loader, tests, pattern):\n'
                        '    return unittest.TestSuite()\n', 'incomplete/duplicate'),
        'duplicate_identity': (PASS + '\ndef load_tests(loader, tests, pattern):\n'
                               '    test = Check("test_pass")\n'
                               '    return unittest.TestSuite([test, test])\n',
                               'incomplete/duplicate'),
        'helper': (PASS, 'unregistered'),
        'nested': (PASS, 'unregistered'),
        'missing_source_identity': (PASS, 'incomplete/duplicate'),
        'missing_execution': ('import unittest\nclass Check(unittest.TestCase):\n'
                              '    def test_required(self): self.assertTrue(True)\n'
                              '    def run(self, result=None): return result\n', 'tests_run'),
        'class_skip': ('import unittest\n@unittest.skip("class skip")\n'
                       'class Check(unittest.TestCase):\n'
                       '    def test_required(self): self.fail("must execute")\n', 'skipped'),
        'class_setup_skip': ('import unittest\nclass Check(unittest.TestCase):\n'
                             '    @classmethod\n'
                             '    def setUpClass(cls): raise unittest.SkipTest("setup skip")\n'
                             '    def test_required(self): self.fail("must execute")\n', 'skipped'),
        'module_skip': ('import unittest\nraise unittest.SkipTest("module skip")\n',
                        'incomplete/duplicate'),
        'duplicate_execution': (PASS + '    def test_second(self): self.assertTrue(True)\n'
                                '    def run(self, result=None):\n'
                                '        return unittest.TestCase.run(Check("test_pass"), result)\n',
                                'duplicate'),
    }

    def invoke(self, runner, mode, env, optimized):
        return subprocess.run([sys.executable] + (['-O'] if optimized else []) +
                              [str(runner), mode], env=env, capture_output=True,
                              text=True, timeout=30)

    def run_case(self, mode, case, optimized, stale):
        source, reason = self.CASES[case]
        with tempfile.TemporaryDirectory(prefix='ci-select003-runner-') as temporary:
            root = Path(temporary)
            runner = root / 'python/qualification/run.py'
            tests = runner.parent / mode
            tests.mkdir(parents=True)
            shutil.copyfile(ROOT / 'python/qualification/run.py', runner)
            env = dict(os.environ, PYTHONDONTWRITEBYTECODE='1', RUNNER_TEMP=str(root / 'evidence'))
            evidence = root / 'evidence/py006-uci'
            evidence.mkdir(parents=True)
            receipt = evidence / 'ci-select003-python.json'
            # These are isolated input-presence fixtures, not pinned qualification
            # evidence. Authentic inputs are exercised separately in qualification.
            for name in ('AMS_GRA_UCI_2_5_ROOT', 'AMS_GRA_UCI_2_6_ROOT', 'AMS_GRA_SLEET_BIN'):
                fixture = root / name
                fixture.write_text('isolated input fixture\n')
                env[name] = str(fixture)
            generator = root / 'target/release/ams-gra-codegen-oms'
            generator.parent.mkdir(parents=True)
            generator.write_text('isolated generator fixture\n')
            module = tests / 'test_control.py'
            if stale:
                module.write_text(self.PASS)
                previous = self.invoke(runner, mode, env, optimized)
                self.assertEqual(previous.returncode, 0, previous.stdout + previous.stderr)
                self.assertEqual(json.loads(receipt.read_text())['status'], 'success')
            module.write_text(source)
            if case == 'no_sources':
                module.unlink()
            elif case == 'helper':
                (tests / 'helper.py').write_text('pass\n')
            elif case == 'nested':
                (tests / 'nested').mkdir()
            elif case == 'missing_source_identity':
                (tests / 'test_unrepresented.py').write_text('import unittest\n')
            result = self.invoke(runner, mode, env, optimized)
            output = result.stdout + result.stderr
            marker = f'CI_SELECT_003_{mode.upper()}_COMPLETE'
            if reason is None:
                self.assertEqual(result.returncode, 0, output)
                self.assertIn(marker, result.stdout)
                if mode == 'uci':
                    document = json.loads(receipt.read_text())
                    self.assertEqual(document['schema_version'], 2)
                    self.assertEqual(document['status'], 'success')
                    self.assertEqual(document['reasons'], [])
                    for field in ('tests_registered', 'tests_run', 'tests_passed'):
                        self.assertEqual(document[field], 1)
                    for field in ('failures', 'errors', 'skipped', 'expected_failures',
                                  'unexpected_successes'):
                        self.assertEqual(document[field], 0)
                    self.assertEqual(document['sources'], ['test_control.py'])
                    self.assertEqual(document['test_ids'], ['test_control.Check.test_pass'])
                    self.assertEqual(document['executed_test_ids'], document['test_ids'])
                    self.assertEqual(document['passed_test_ids'], document['test_ids'])
            else:
                self.assertNotEqual(result.returncode, 0, output)
                self.assertNotIn(marker, output)
                self.assertIn(reason, output)
                self.assertFalse(receipt.exists(), output)
                if 'CI_SELECT_003_QUALIFICATION_FAILED ' in result.stderr:
                    execution = json.loads(result.stderr.split(
                        'CI_SELECT_003_QUALIFICATION_FAILED ', 1)[1])
                    self.assertEqual(execution['status'], 'failed')
                    self.assertTrue(execution['reasons'])
                    self.assertIn('tests_registered', execution)
                    self.assertIn('tests_run', execution)
                    self.assertIn('tests_passed', execution)
                    if reason in ('skipped', 'expected_failures', 'unexpected_successes',
                                  'failures', 'errors'):
                        self.assertEqual(execution[reason], 1)
            self.assertFalse(list(evidence.glob('.ci-select003-*')))

    def test_actual_runner_adversarial_matrix(self):
        for mode in ('fast', 'uci', 'sleet'):
            for case in self.CASES:
                for optimized in (False, True):
                    for stale in ((False, True) if mode == 'uci' else (False,)):
                        with self.subTest(mode=mode, case=case, optimized=optimized, stale=stale):
                            self.run_case(mode, case, optimized, stale)

    def test_uci_invalid_inputs_clear_previous_success(self):
        for optimized in (False, True):
            for missing in ('AMS_GRA_UCI_2_5_ROOT', 'AMS_GRA_UCI_2_6_ROOT', 'generator'):
                with self.subTest(optimized=optimized, missing=missing), \
                        tempfile.TemporaryDirectory() as temporary:
                    root = Path(temporary)
                    runner = root / 'python/qualification/run.py'
                    runner.parent.mkdir(parents=True)
                    shutil.copyfile(ROOT / 'python/qualification/run.py', runner)
                    evidence = root / 'py006-uci'
                    evidence.mkdir()
                    receipt = evidence / 'ci-select003-python.json'
                    receipt.write_text('{"status": "success", "tests_run": 999}\n')
                    env = dict(os.environ, RUNNER_TEMP=str(root), PYTHONDONTWRITEBYTECODE='1')
                    for name in ('AMS_GRA_UCI_2_5_ROOT', 'AMS_GRA_UCI_2_6_ROOT'):
                        fixture = root / name
                        fixture.write_text('input fixture\n')
                        env[name] = str(fixture)
                    if missing != 'generator':
                        del env[missing]
                    result = self.invoke(runner, 'uci', env, optimized)
                    self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)
                    self.assertFalse(receipt.exists())
                    self.assertNotIn('CI_SELECT_003_UCI_COMPLETE', result.stdout)
                    self.assertIn('missing', result.stderr)

    def test_uci_receipt_requires_existing_artifact_directory_and_atomic_publication(self):
        for optimized in (False, True):
            for invalid in ('missing_environment', 'missing_directory', 'publication_failure'):
                with self.subTest(optimized=optimized, invalid=invalid), \
                        tempfile.TemporaryDirectory() as temporary:
                    root = Path(temporary)
                    runner = root / 'python/qualification/run.py'
                    tests = runner.parent / 'uci'
                    tests.mkdir(parents=True)
                    shutil.copyfile(ROOT / 'python/qualification/run.py', runner)
                    evidence = root / 'evidence/py006-uci'
                    if invalid != 'missing_directory':
                        evidence.mkdir(parents=True)
                    env = dict(os.environ, RUNNER_TEMP=str(root / 'evidence'),
                               PYTHONDONTWRITEBYTECODE='1')
                    for name in ('AMS_GRA_UCI_2_5_ROOT', 'AMS_GRA_UCI_2_6_ROOT'):
                        fixture = root / name
                        fixture.write_text('input fixture\n')
                        env[name] = str(fixture)
                    generator = root / 'target/release/ams-gra-codegen-oms'
                    generator.parent.mkdir(parents=True)
                    generator.write_text('generator fixture\n')
                    source = self.PASS
                    if invalid == 'missing_environment':
                        del env['RUNNER_TEMP']
                    elif invalid == 'publication_failure':
                        # The test really passes but removes the evidence directory;
                        # publication must fail without a success marker or receipt.
                        source = ('import os\nfrom pathlib import Path\nimport unittest\n'
                                  'class Check(unittest.TestCase):\n'
                                  '    def test_pass(self):\n'
                                  '        (Path(os.environ["RUNNER_TEMP"]) / "py006-uci").rmdir()\n')
                    (tests / 'test_control.py').write_text(source)
                    result = self.invoke(runner, 'uci', env, optimized)
                    self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)
                    self.assertNotIn('CI_SELECT_003_UCI_COMPLETE', result.stdout)
                    self.assertFalse((evidence / 'ci-select003-python.json').exists())
                    self.assertIn('RECEIPT_FAILED' if invalid == 'publication_failure'
                                  else 'CI_SELECT_003_INVALID', result.stderr)


if __name__ == '__main__':
    unittest.main()
