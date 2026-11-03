"""Deep pack placement, isolation, original inventory and fail-closed controls."""
from concurrent.futures import ThreadPoolExecutor
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

from ada_deep_pack import run, verify
from ada_deep_pack.contracts import check


class DeepPack(unittest.TestCase):
    def tearDown(self):
        run.STOP.clear()

    def test_four_runner_allocations_and_exact_original_campaigns(self):
        self.assertEqual([len(run.INVENTORY['packs'][p]) for p in ('a', 'b')], [4, 3])
        self.assertEqual(sorted(s['task'] for s in run.INVENTORY['suites'].values()), list(range(100, 107)))
        for name in run.INVENTORY['suites']:
            check(self, name)

    def test_applicability_does_not_lose_task100_or_task101(self):
        self.assertEqual(run.suites('a', False), run.INVENTORY['packs']['a'][:2])
        self.assertEqual(run.suites('a', True), run.INVENTORY['packs']['a'])
        self.assertEqual(run.suites('b', False), [])
        self.assertEqual(run.suites('b', True), run.INVENTORY['packs']['b'])

    def test_exact_qualification_inputs(self):
        for path, digest in run.INVENTORY['qualification_sha256'].items():
            with self.subTest(path=path):
                self.assertEqual(run.sha(run.ROOT / path), digest)

    def test_original_command_and_environment_inventory(self):
        from workflow_text import blocks
        original = blocks(subprocess.check_output([
            'git', 'show', run.INVENTORY['origin_commit'] + ':.github/workflows/deep-ci.yml'],
            cwd=run.ROOT, text=True))[1]
        for name, spec in run.INVENTORY['suites'].items():
            for step in spec['steps']:
                self.assertIn('name: ' + step['name'], original[name])
                for line in step['run'].splitlines():
                    self.assertIn(line, original[name])
                for key, value in step.get('env', {}).items():
                    self.assertIn(key + ': ' + value.strip("'"), original[name].replace("'1'", '1'))
            self.assertIn(spec['artifact']['name'], original[name])
            self.assertEqual(spec['timeout_seconds'], 7200)
            self.assertEqual(spec['websockets'], '17.2' if spec['task'] <= 103 else '15.0.1')

    def test_read_only_dependency_tampering_fails_closed(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            file = root / 'provider'
            file.write_text('original')
            with self.assertRaisesRegex(RuntimeError, 'writable'):
                run.manifest(root)
            run.freeze(root)
            original = run.manifest(root)
            file.chmod(0o644)
            with self.assertRaisesRegex(RuntimeError, 'writable'):
                run.manifest(root)
            file.write_text('corrupted')
            run.freeze(root)
            self.assertNotEqual(run.manifest(root), original)
            root.chmod(0o755)
            file.chmod(0o644)

    def test_private_setup_relocation(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            seed = root / 'seed'
            for folder in ('cache/toolchains/compiler', 'settings', 'home', 'consumer/obj', 'consumer/bin'):
                (seed / folder).mkdir(parents=True)
            (seed / 'cache/toolchains/compiler/gnat').write_text('compiler')
            (seed / 'cache/toolchains/compiler/gnat-link').symlink_to('gnat')
            (seed / 'settings/settings.toml').write_text('cache=' + str(seed / 'cache'))
            run.freeze(seed / 'cache/toolchains')
            try:
                for name in ('one', 'two'):
                    run.relocate_setup(seed, root / name)
                    self.assertIn(str(root / name / 'cache'), (root / name / 'settings/settings.toml').read_text())
                    self.assertFalse((root / name / 'cache/toolchains').is_symlink())
                    self.assertEqual(run.manifest(root / name / 'cache/toolchains'),
                                     run.manifest(seed / 'cache/toolchains'))
                    self.assertFalse((root / name / 'consumer/obj').exists())
                (root / 'one/settings/settings.toml').write_text('private')
                self.assertNotEqual((root / 'two/settings/settings.toml').read_text(), 'private')
            finally:
                for tree in (seed, root / 'one', root / 'two'):
                    for p in (tree / 'cache/toolchains').rglob('*'):
                        p.chmod(0o755 if p.is_dir() else 0o644)
                    (tree / 'cache/toolchains').chmod(0o755)

    def test_failed_worker_does_not_erase_other_completions(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            names = run.INVENTORY['packs']['a'][:2]
            for name in names:
                (root / name).mkdir()
            with patch.object(run, 'execute', side_effect=RuntimeError('injected failure')):
                with ThreadPoolExecutor(max_workers=2) as pool:
                    rows = list(pool.map(lambda n: run.worker(n, root / n, root / 'shared', '14', {}, 0), names))
            self.assertEqual([r['exit'] for r in rows], [1, 1])
            self.assertTrue(all((root / n / 'completion.json').is_file() for n in names))

    def test_zero_exit_without_all_workers_is_not_success(self):
        with tempfile.TemporaryDirectory() as tmp, patch.dict(os.environ, GITHUB_RUN_ID='1', GITHUB_RUN_ATTEMPT='1'):
            root = Path(tmp)
            source = dict(sha='head', tree='tree', run_id='1', attempt='1')
            run.write(root / 'summary.json', dict(source=source, pack='a', compiler='14',
                      runner_slots=1, applicable_suites=run.suites('a', True), workers=[], exit=0))
            with self.assertRaisesRegex(RuntimeError, 'failed/incomplete'):
                verify.verify(root, 'head', 'tree', 'a', '14', True)
            with self.assertRaisesRegex(RuntimeError, 'source/tree'):
                verify.verify(root, 'other', 'tree', 'a', '14', True)

    def test_deadline_retains_exit_and_log(self):
        with tempfile.TemporaryDirectory() as tmp:
            stem = Path(tmp) / 'timeout'
            with self.assertRaisesRegex(RuntimeError, 'timeout'):
                run.execute(['sleep', '10'], dict(os.environ), Path(tmp), stem, 0.1)
            self.assertEqual(json.loads(stem.with_suffix('.json').read_text())['reason'], 'timeout')
            self.assertTrue(stem.with_suffix('.exit').is_file())
            self.assertTrue(stem.with_suffix('.log').is_file())


if __name__ == '__main__':
    unittest.main()