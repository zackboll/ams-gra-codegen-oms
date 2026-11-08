"""Fail-closed pack orchestration and original qualification inventory regressions."""
import importlib.util
import json
import os
from pathlib import Path
import signal
import shutil
import subprocess
import sys
import tempfile
import time
import unittest
from unittest.mock import patch

from ada_ci_pack import run as pack
from ada_ci_pack import alr
from ada_ci_pack import verify


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
                        self.assertIn(line.replace("scripts/perf009-ci.py", "scripts/check-task093-helper-demand.py"), jobs[name])

    def test_inner_parallelism_and_explicit_private_settings(self):
        with patch.dict(os.environ, PACK_REAL_ALR='/verified/alr', PACK_SETTINGS='/worker/settings'):
            for command in (['-n', 'build'], ['-n', 'build', '--', '-f', '-cargs:Ada', '-gnatec=x']):
                actual = alr.arguments(command)
                self.assertEqual(actual[:2], ['/verified/alr', '--settings=/worker/settings'])
                self.assertEqual(actual[actual.index('--') + 1], '-j1')
                if '-cargs:Ada' in command:
                    self.assertLess(actual.index('-j1'), actual.index('-cargs:Ada'))

    def test_no_rewriting_toolchain_pins(self):
        command = ['-n', 'toolchain', '--select', '--local', 'gnat_native=16.1.0', 'gprbuild=26.0.1']
        with patch.dict(os.environ, PACK_REAL_ALR='/verified/alr', PACK_SETTINGS='/worker/settings'):
            self.assertEqual(alr.arguments(command)[2:], command)

    def test_archive_recovery_uses_only_worker_private_settings(self):
        with tempfile.TemporaryDirectory() as tmp, patch.object(pack.subprocess, 'run') as run:
            directory = Path(tmp) / 'worker'
            pack.environment(directory, '/verified/alr')
            hooks = [call.args[0] for call in run.call_args_list
                     if 'origins.archive.download_cmd' in call.args[0]]
            self.assertEqual(len(hooks), 1)
            self.assertEqual(hooks[0], ['/verified/alr', '--settings=' + str(directory / 'settings'),
                '-n', 'settings', '--global', '--set', 'origins.archive.download_cmd',
                f'python3 {pack.ROOT / "scripts/pinned_download.py"} ${{URL}} ${{DEST}}'])

    @unittest.skipUnless(shutil.which('alr'), 'live Alire unavailable; hosted qualification still mandatory')
    def test_live_alire_private_configuration_and_wrapper(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / 'private'
            env = pack.environment(root, shutil.which('alr'))
            self.assertEqual(subprocess.check_output(['alr', '--version'], env=env).strip(), b'alr 2.1.1')
            version = subprocess.check_output(['alr', 'version'], env=env, text=True)
            self.assertIn(str(root / 'settings'), version)
            self.assertIn(str(root / 'cache'), version)
            self.assertIn(str(root / 'cache/toolchains'), version)
            self.assertEqual(subprocess.check_output(['alr', '-n', 'settings', '--global', '--get',
                                                     'dependencies.shared'], env=env).strip().lower(), b'false')

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

    def test_exact_marker_inventory_and_missing_cell(self):
        # Exercise the real independent verifier, including all original profiles/policies.
        names = {'ada-generated-codec': [
            ('task087b-codec', ['normal', 'Check', 'Ignore'], 'TASK087B_CODEC_COMPLETE cases= 41', '-run.log'),
            ('task087b-codec', ['subset-normal', 'subset-Check', 'subset-Ignore'], 'TASK087B_SUBSET_COMPLETE cases= 7', '-run.log'),
            ('task088-choice', ['oam-normal', 'oam-Check', 'oam-Ignore'], 'TASK088_CODEC_COMPLETE cases= 172', '-run.log'),
            ('task088-choice', ['qualified-normal', 'qualified-Check', 'qualified-Ignore'], 'TASK088_QUALIFIED_COMPLETE cases= 2', '-run.log')],
            'ada-repeated-temporal-codec': [(folder, [str(i) for i in range(6)], marker, '-execute.log')
                for folder, marker in [('task094', 'TASK094_CODEC_COMPLETE checks= 54 rejections= 17'),
                                       ('task094-composition', 'TASK094_COMPOSITION_COMPLETE checks= 19 rejections= 3'),
                                       ('task094-combined', 'TASK094_COMBINED_COMPLETE checks= 26 rejections= 2')]],
            'ada-datetime-codec': [
                ('task090', ['named-normal', 'named-Check', 'named-Ignore'], 'TASK090_NAMED_COMPLETE cases= 161', '-run.log'),
                ('task090', ['direct-normal', 'direct-Check', 'direct-Ignore'], 'TASK090_DIRECT_COMPLETE cases= 205', '-run.log'),
                ('task090r', ['normal', 'Check', 'Ignore'], 'TASK090R_COMBINED_PASS', '-run.log')]}
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            for name, specs in names.items():
                cell_root = root / name
                files = []
                for folder, cells, marker, suffix in specs:
                    (cell_root / folder).mkdir(parents=True, exist_ok=True)
                    for cell in cells:
                        file = cell_root / folder / (cell + suffix)
                        file.write_text(marker + '\n')
                        files.append(file)
                self.assertEqual(len(pack.verify_markers(name, cell_root)), len(files))
                file = files[0]
                original = file.read_text()
                for mutated in ('', original.replace('COMPLETE', 'SKIPPED'), original + original):
                    file.write_text(mutated)
                    with self.subTest(suite=name, mutated=mutated), self.assertRaises(RuntimeError):
                        pack.verify_markers(name, cell_root)
                file.write_text(original)

    def test_independent_verifier_rejects_failed_or_missing_constituent(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            source = {'sha': 'head', 'tree': 'tree'}
            summary = dict(source=source, exit=0, pack='a', runner_slots=1,
                           generator_sha256='generator', workers=[])
            pack.write(root / 'summary.json', summary)
            with patch.object(verify.run, 'sha', return_value='generator'):
                with self.assertRaisesRegex(RuntimeError, 'missing/duplicate constituent'):
                    verify.verify(root, 'head', 'tree', 'a')
                summary['exit'] = 1
                pack.write(root / 'summary.json', summary)
                with self.assertRaisesRegex(RuntimeError, 'did not complete'):
                    verify.verify(root, 'head', 'tree', 'a')
                with self.assertRaisesRegex(RuntimeError, 'source identity mismatch'):
                    verify.verify(root, 'wrong', 'tree', 'a')

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