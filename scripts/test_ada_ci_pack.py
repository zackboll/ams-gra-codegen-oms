"""Fail-closed pack orchestration and original qualification inventory regressions."""
import importlib.util
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import time
import unittest
from unittest.mock import patch

from ada_ci_pack import run as pack
from ada_ci_pack import alr


class PackTests(unittest.TestCase):
    def tearDown(self):
        pack.STOP.clear()

    def test_exact_inventory_and_packs(self):
        self.assertEqual(pack.INVENTORY['packs']['a'], [
            'ada-generated-codec', 'ada-repeated-temporal-codec',
            'ada-datetime-codec', 'ada-hexbinary-codec'])
        self.assertEqual(pack.INVENTORY['packs']['b'], [
            'ada-repeated-codec', 'ada-duration-codec', 'ada-repeated-choice-codec'])
        for name, suite in pack.INVENTORY['suites'].items():
            self.assertTrue(suite['steps'])
            self.assertEqual(suite['artifact']['if-no-files-found'], 'error')
            for step in suite['steps']:
                if 'scripts/check-task' in step['run']:
                    self.assertIn('--compiler 16', step['run'])
                    self.assertNotIn('--policies', step['run'])
                    self.assertNotIn('--profiles', step['run'])
            if name != 'ada-json-guard':
                self.assertTrue(any('--generator "$GITHUB_WORKSPACE/target/debug/ams-gra-codegen-oms"'
                                    in step['run'] for step in suite['steps']))

    def test_all_original_inputs_unchanged(self):
        for name, digest in pack.INVENTORY['qualification_sha256'].items():
            with self.subTest(path=name):
                self.assertEqual(pack.sha(pack.ROOT / name), digest)

    def test_inventory_commands_equal_incorporated_main(self):
        # No YAML dependency needed at runtime: compare against frozen main step text.
        text = subprocess.check_output(['git', 'show', pack.INVENTORY['origin_commit'] +
                                        ':.github/workflows/ci.yml'], cwd=pack.ROOT, text=True)
        from workflow_text import blocks
        _, jobs = blocks(text)
        for name, suite in pack.INVENTORY['suites'].items():
            for step in suite['steps']:
                with self.subTest(suite=name, step=step['name']):
                    for line in step['run'].splitlines():
                        self.assertIn(line, jobs[name])

    def test_inner_parallelism_and_explicit_private_settings(self):
        with patch.dict(os.environ, PACK_REAL_ALR='/verified/alr', PACK_SETTINGS='/worker/settings'):
            for command in (['-n', 'build'], ['-n', 'build', '--', '-f', '-cargs:Ada', '-gnatec=x']):
                actual = alr.arguments(command)
                self.assertEqual(actual[:3], ['/verified/alr', '--settings', '/worker/settings'])
                self.assertEqual(actual[actual.index('--') + 1], '-j1')
                if '-cargs:Ada' in command:
                    self.assertLess(actual.index('-j1'), actual.index('-cargs:Ada'))

    def test_no_rewriting_toolchain_pins(self):
        command = ['-n', 'toolchain', '--select', '--local', 'gnat_native=16.1.0', 'gprbuild=26.0.1']
        with patch.dict(os.environ, PACK_REAL_ALR='/verified/alr', PACK_SETTINGS='/worker/settings'):
            self.assertEqual(alr.arguments(command)[3:], command)

    def test_success_and_failure_exit_capture(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            for code in (0, 17):
                stem = root / f'exit-{code}'
                if code:
                    with self.assertRaises(RuntimeError):
                        pack.execute([sys.executable, '-c', f'print("test");exit({code})'],
                                     os.environ, root, stem, 10)
                else:
                    pack.execute([sys.executable, '-c', 'print("test")'], os.environ, root, stem, 10)
                self.assertEqual(json.loads(stem.with_suffix('.json').read_text())['exit'], code)
                self.assertEqual(stem.with_suffix('.exit').read_text(), str(code) + '\n')
                self.assertIn('test', stem.with_suffix('.log').read_text())

    def test_timeout_reaps_sigterm_resistant_process(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            command = [sys.executable, '-c', 'import signal,time;signal.signal(signal.SIGTERM,signal.SIG_IGN);time.sleep(60)']
            started = time.monotonic()
            with self.assertRaises(RuntimeError):
                pack.execute(command, os.environ, root, root / 'timeout', 0.3)
            record = json.loads((root / 'timeout.json').read_text())
            self.assertEqual(record['reason'], 'timeout')
            self.assertEqual(record['exit'], -signal.SIGKILL)
            self.assertLess(time.monotonic() - started, 5)

    def test_forced_termination_is_not_success(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            pack.STOP.set()
            with self.assertRaises(RuntimeError):
                pack.execute([sys.executable, '-c', 'import time;time.sleep(60)'],
                             os.environ, root, root / 'terminated', 10)
            self.assertEqual(json.loads((root / 'terminated.json').read_text())['reason'], 'terminated')

    def test_affinity_uses_allowed_cpu_not_zero(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            cpu = max(os.sched_getaffinity(0))
            record = pack.execute([sys.executable, '-c',
                                   f'import os;assert os.sched_getaffinity(0)=={{{cpu}}}'],
                                  os.environ, root, root / 'cpu', 10, cpu)
            self.assertEqual(record['command'][:3], ['taskset', '--cpu-list', str(cpu)])

    def test_source_identity_mismatch_rejected(self):
        with self.assertRaisesRegex(RuntimeError, 'source identity mismatch'):
            pack.identity('0' * 40)

    def test_missing_artifacts_fail_every_suite(self):
        with tempfile.TemporaryDirectory() as tmp:
            for name in pack.INVENTORY['suites']:
                with self.subTest(suite=name), self.assertRaisesRegex(RuntimeError, 'missing artifact'):
                    pack.verify_artifacts(name, Path(tmp))

    def test_missing_markers_fail_every_suite(self):
        with tempfile.TemporaryDirectory() as tmp:
            for name in pack.INVENTORY['suites']:
                with self.subTest(suite=name), self.assertRaisesRegex(RuntimeError, 'completion evidence'):
                    pack.verify_markers(name, Path(tmp))

    def test_each_original_artifact_pattern_is_required(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / 'exists.log').write_text('present')
            suite = dict(artifact=dict(path='${{ runner.temp }}/*.log\n${{ runner.temp }}/missing.lock'))
            with patch.dict(pack.INVENTORY['suites'], fake=suite), self.assertRaises(RuntimeError):
                pack.verify_artifacts('fake', root)

    def test_worker_failure_does_not_prevent_another_completion(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            first, second = root / 'one', root / 'two'
            first.mkdir()
            second.mkdir()
            with patch.object(pack, 'original_steps', side_effect=[RuntimeError('injected'), []]), \
                    patch.object(pack, 'verify_artifacts'), patch.object(pack, 'verify_markers', return_value=['evidence']):
                records = [pack.worker('one', first, {}, 7, {'sha': 'source'}),
                           pack.worker('two', second, {}, 8, {'sha': 'source'})]
            self.assertEqual([record['exit'] for record in records], [1, 0])
            self.assertTrue((first / 'completion.json').is_file())
            self.assertTrue((second / 'completion.json').is_file())

    def test_exit_zero_without_evidence_is_red(self):
        with tempfile.TemporaryDirectory() as tmp:
            with patch.object(pack, 'original_steps', return_value=[]):
                result = pack.worker('ada-generated-codec', Path(tmp), {}, 0, {})
            self.assertEqual(result['exit'], 1)
            self.assertIn('missing artifact', result['error'])

    def test_guard_retains_frozen_output_validation(self):
        spec = importlib.util.spec_from_file_location('guard', pack.ROOT / 'scripts/check-task087a.py')
        guard = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(guard)
        for text in (b'', b'GUARDED CORPUS PASSED cases= 199\n', b'skipped\n'):
            with self.assertRaises(RuntimeError):
                guard.validate_output(text)


if __name__ == '__main__':
    unittest.main()