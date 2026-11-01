"""Product qualification runner execution and no-skip rejection controls."""
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest
ROOT = Path(__file__).resolve().parents[1]


class ProductDiscovery(unittest.TestCase):
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
