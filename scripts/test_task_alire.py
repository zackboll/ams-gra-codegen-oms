"""Task-owned Alire configuration and unchanged qualification contracts."""
from pathlib import Path
import os
import shutil
import subprocess
import tempfile
import unittest
from unittest.mock import Mock, patch

from task_alire import configure
from workflow_text import blocks

ROOT = Path(__file__).resolve().parents[1]


class TaskAlire(unittest.TestCase):
    def test_private_settings_cache_and_exact_verified_download_hook(self):
        with tempfile.TemporaryDirectory() as temporary:
            commands = []
            for name in ('task092', 'task093-25', 'task093-26'):
                scratch = Path(temporary) / name
                scratch.mkdir()
                checked = Mock()
                alr = configure(scratch, checked, ROOT)
                self.assertEqual(alr, ['alr', '-s', str(scratch / 'alire-settings'), '-n'])
                calls = [call.args for call in checked.call_args_list]
                self.assertEqual(len(calls), 4)
                for command, cwd, log in calls:
                    self.assertEqual(command[:4], alr)
                    self.assertEqual(command[4:7], ['settings', '--global', '--set'])
                    self.assertEqual(cwd, ROOT)
                    self.assertEqual(log.parent, scratch)
                self.assertEqual(calls[0][0][-2:], ['cache.dir', str(scratch / 'alire-cache')])
                self.assertEqual(calls[1][0][-2:], ['toolchain.dir', str(scratch / 'alire-cache/toolchains')])
                self.assertEqual(calls[2][0][-2:], ['dependencies.shared', 'false'])
                self.assertEqual(calls[3][0][-2], 'origins.archive.download_cmd')
                self.assertTrue(calls[3][0][-1].endswith(
                    str(ROOT / 'scripts/pinned_download.py') + ' ${URL} ${DEST}'))
                commands.append(alr)
            self.assertEqual(len({command[2] for command in commands}), 3)

    def test_refuses_reused_settings_and_ambiguous_hook_paths(self):
        with tempfile.TemporaryDirectory() as temporary:
            scratch = Path(temporary)
            configure(scratch, Mock(), ROOT)
            checked = Mock()
            with self.assertRaises(FileExistsError):
                configure(scratch, checked, ROOT)
            checked.assert_not_called()
            ambiguous = scratch / 'space in path'
            ambiguous.mkdir()
            with self.assertRaisesRegex(RuntimeError, 'whitespace-free'):
                configure(ambiguous, checked, ROOT)
            checked.assert_not_called()

    def test_real_tasks_keep_all_alire_commands_in_private_settings(self):
        for task in (92, 93):
            text = (ROOT / f'scripts/check-task0{task}-real.py').read_text()
            with self.subTest(task=task):
                self.assertIn('alr = configure(scratch, checked,', text)
                self.assertNotIn('["alr", "-n"', text)
                self.assertIn('"gprbuild=26.0.1"', text)
                self.assertIn('"14.2.0"', text)
                self.assertIn('"16.1.0"', text)
                self.assertIn('for policy in ["normal", "Check", "Ignore"]:', text)
                self.assertIn('validate_solution(tomllib.loads(lock.read_text()), compiler)', text)
                self.assertIn('alr + ["exec", "--", tool, "--version"]', text)

    def test_unicode_example_is_built_locked_before_first_registered_use(self):
        jobs = blocks((ROOT / '.github/workflows/deep-ci.yml').read_text())[1]
        job = jobs['real-uci-regressions']
        build = 'cargo build --locked --release -p ams-gra-codegen-oms --example python_unicode_profiles'
        self.assertEqual(job.count(build), 1)
        self.assertLess(job.index(build), job.index('scripts/run-owned-python.py uci'))
        for token in ('python3.11 python3.13', '-O scripts/run-owned-python.py uci'):
            self.assertIn(token, job)

    def test_packed_control_requires_exact_inherited_private_settings_and_hook(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            env = dict(ADA_DEEP_REAL_ALR='/verified/alr', AMS_GRA_ADA_BUILD_JOBS='1',
                       HOME=str(root / 'setup/home'))
            hook = f'python3 {ROOT / "scripts/pinned_download.py"} ${{URL}} ${{DEST}}'
            for values, error in [
                ([str(root / 'setup/cache'), 'false', hook], None),
                (['/shared/cache'], 'cache.dir'),
                ([str(root / 'setup/cache'), 'true'], 'dependencies.shared'),
                ([str(root / 'setup/cache'), 'false', 'curl ${URL} -o ${DEST}'], 'download hook'),
            ]:
                checked = Mock(side_effect=[(v + '\n').encode() for v in values])
                with patch.dict(os.environ, env), self.subTest(values=values):
                    if error:
                        with self.assertRaisesRegex(RuntimeError, error):
                            configure(root, checked, ROOT)
                    else:
                        self.assertEqual(configure(root, checked, ROOT), ['alr', '-n'])
                self.assertFalse((root / 'alire-settings').exists())

    @unittest.skipUnless(shutil.which('alr'), 'live Alire unavailable')
    def test_actual_task102_style_wrapper_does_not_receive_duplicate_settings(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            setup = root / 'setup'
            (setup / 'home').mkdir(parents=True)
            settings = setup / 'settings'
            settings.mkdir()
            real = shutil.which('alr')
            for key, value in [('cache.dir', str(setup / 'cache')), ('dependencies.shared', 'false'),
                               ('origins.archive.download_cmd',
                                f'python3 {ROOT / "scripts/pinned_download.py"} ${{URL}} ${{DEST}}')]:
                subprocess.run([real, '-s', str(settings), '-n', 'settings', '--global', '--set', key, value],
                               check=True, capture_output=True)
            wrapper = root / 'bin'
            wrapper.mkdir()
            (wrapper / 'alr').write_text(f'#!/bin/sh\nexec {real} -s {settings} "$@"\n')
            (wrapper / 'alr').chmod(0o755)
            env = dict(HOME=str(setup / 'home'), ADA_DEEP_REAL_ALR=real, AMS_GRA_ADA_BUILD_JOBS='1',
                       PATH=str(wrapper) + os.pathsep + os.environ['PATH'])

            def checked(command, cwd, log):
                result = subprocess.check_output(command, cwd=cwd)
                log.write_bytes(result)
                return result

            with patch.dict(os.environ, env):
                alr = configure(root, checked, ROOT)
                self.assertEqual(alr, ['alr', '-n'])
                output = subprocess.check_output(alr + ['version'], text=True)
                self.assertIn(str(settings), output)
                self.assertIn(str(setup / 'cache/toolchains'), output)


if __name__ == '__main__':
    unittest.main()