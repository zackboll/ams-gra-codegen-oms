"""Deep pack placement, isolation, original inventory and fail-closed controls."""
from concurrent.futures import ThreadPoolExecutor
import hashlib
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

    def test_reconciled_inputs_preserve_historical_source_identity(self):
        import hashlib
        report = json.loads((run.ROOT / run.INVENTORY['source_reconciliation']).read_text())
        historical = report['historical_source']
        path = historical['inventory']
        raw = (run.ROOT / path).read_bytes()
        self.assertEqual(hashlib.sha256(raw).hexdigest(), historical['inventory_sha256'])
        self.assertEqual(raw, subprocess.check_output([
            'git', 'show', historical['sha'] + ':scripts/ada_deep_pack/inventory.json'], cwd=run.ROOT))
        previous = json.loads(raw)
        updates = report.get('qualification_updates', {})
        dependency_updates = report.get('dependency_updates', {})
        current_inputs = dict(run.INVENTORY['qualification_sha256'])
        for path, update in dependency_updates.items():
            self.assertEqual(current_inputs[path], update['current_sha256'])
            self.assertEqual(hashlib.sha256((run.ROOT / path).read_bytes()).hexdigest(),
                             update['current_sha256'])
            self.assertTrue(update['review'])
            current_inputs[path] = update['previous_sha256']
        self.assertEqual(report['qualification_sha256'], current_inputs)
        self.assertEqual(len(previous['qualification_sha256']), 330)
        self.assertEqual(report['reviewed_inputs'], 330)
        self.assertEqual(report['unchanged_inputs'], 310)
        self.assertEqual(len(report['main_changes']), 20)
        main = report['incorporated_main']['sha']
        self.assertEqual(report['incorporated_main']['tree'], subprocess.check_output([
            'git', 'rev-parse', main + '^{tree}'], cwd=run.ROOT, text=True).strip())
        for update in report.get('main_updates', []):
            changed = subprocess.check_output([
                'git', 'diff', '--name-only', update['previous_main']['sha'],
                update['main']['sha']], cwd=run.ROOT, text=True).splitlines()
            self.assertEqual(changed, update['changed_files'])
            self.assertEqual(update['qualification_inputs_changed'], 0)
            self.assertFalse(set(changed) & set(run.INVENTORY['qualification_sha256']))
        for path, digest in current_inputs.items():
            with self.subTest(path=path):
                original = subprocess.check_output([
                    'git', 'show', previous['origin_commit'] + ':' + path], cwd=run.ROOT)
                current_main = subprocess.check_output(['git', 'show', main + ':' + path], cwd=run.ROOT)
                self.assertEqual(path in report['main_changes'], original != current_main)
                if path in report['main_changes']:
                    change = report['main_changes'][path]
                    self.assertEqual(change['previous_sha256'], previous['qualification_sha256'][path])
                    self.assertEqual(change['main_sha256'], hashlib.sha256(current_main).hexdigest())
                    update = updates.get(path)
                    self.assertEqual(change['current_sha256'], update['previous_sha256'] if update else digest)
                    if update:
                        self.assertEqual(update['current_sha256'], digest)
                        self.assertTrue(update['review'])
                        incorporated = subprocess.check_output([
                            'git', 'show', update['source_sha'] + ':' + path], cwd=run.ROOT)
                        self.assertEqual(hashlib.sha256(incorporated).hexdigest(), digest)
                    self.assertTrue(change['review'])
                else:
                    retained = report['retained_pack_changes'].get(path)
                    self.assertEqual(digest, retained['current_sha256'] if retained
                                     else previous['qualification_sha256'][path])
                if hashlib.sha256(current_main).hexdigest() != digest:
                    self.assertEqual((updates.get(path) or report['retained_pack_changes'][path])['current_sha256'], digest)
        for key in ('origin_commit', 'origin_tree', 'packs', 'suites'):
            self.assertEqual(run.INVENTORY[key], previous[key])

    def test_unrelated_deep_jobs_remain_exactly_current_main(self):
        from workflow_text import blocks
        report = json.loads((run.ROOT / run.INVENTORY['source_reconciliation']).read_text())
        original = blocks(subprocess.check_output([
            'git', 'show', report['incorporated_main']['sha'] + ':.github/workflows/deep-ci.yml'],
            cwd=run.ROOT, text=True))[1]
        current = blocks((run.ROOT / '.github/workflows/deep-ci.yml').read_text())[1]
        suites = set(run.INVENTORY['suites'])
        self.assertEqual(set(current), (set(original) - suites) | {'ada-deep-pack-a', 'ada-deep-pack-b'})
        for name, body in original.items():
            if name not in suites:
                with self.subTest(job=name):
                    # CI-MAIN-002: permit only these exact reviewed corrections;
                    # all other bytes in every original job remain frozen.
                    if name in ('ada-repeated-choice-real-uci', 'ada-hexbinary-real'):
                        old = ('curl --fail --location --retry 3 --output alire.zip \\\n'
                               '            https://github.com/alire-project/alire/releases/download/v2.1.1/alr-2.1.1-bin-x86_64-linux.zip')
                        self.assertEqual(body.count(old), 1)
                        body = body.replace(old, 'python3 "$GITHUB_WORKSPACE/scripts/pinned_download.py" alire alire.zip')
                    if name == 'real-uci-regressions-ascii':
                        old = "grep -qE '^test result: ok\\. 2 passed' <<<\"$output\""
                        new = "grep -qE '^test result: ok\\. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in [0-9.]+s$' <<<\"$output\""
                        self.assertEqual(body.count(old), 1)
                        body = body.replace(old, new)
                    if name == 'real-sleet':
                        body = body.replace('real pinned Sleet (e38f61d8)',
                                            'real pinned Sleet (94796daf)')
                    self.assertEqual(current[name], body)

    def test_transport_helper_is_source_bound(self):
        self.assertEqual(set(run.INVENTORY['infrastructure_sha256']), {'scripts/pinned_download.py'})
        for path, digest in run.INVENTORY['infrastructure_sha256'].items():
            self.assertEqual(run.sha(run.ROOT / path), digest)

    def test_task108_dependencies_and_packed_compilation_are_preserved(self):
        from workflow_text import blocks
        jobs = blocks((run.ROOT / '.github/workflows/deep-ci.yml').read_text())[1]
        for pack in ('a', 'b'):
            self.assertIn('gnat gnat-14 libssl-dev', jobs['ada-deep-pack-' + pack])
        for task in range(100, 107):
            text = (run.ROOT / f'scripts/task{task}/qualify.py').read_text()
            self.assertIn('827b0c17dd964e935f8e88912ce0611ee92b6f16', text)
            self.assertIn('https://github.com/zackboll/aht.git', text)
            self.assertIn('GPR_PROJECT_PATH', text)
            self.assertNotIn('664d5ddeb568626b2f9ff66054c7091894b3b8d4', text)
        from ada_deep_pack.alr import arguments
        with patch.dict(os.environ, ADA_DEEP_REAL_ALR='/verified/alr'):
            self.assertEqual(arguments(['-n', 'build', '--', '-p']),
                             ['/verified/alr', '-n', 'build', '--', '-j1', '-p'])

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

    def test_original_suite_command_hashes_are_unchanged(self):
        historical = json.loads((run.ROOT / 'scripts/ada_deep_pack/inventory-c523158.json').read_text())
        for name, spec in run.INVENTORY['suites'].items():
            with self.subTest(suite=name):
                self.assertEqual(
                    [hashlib.sha256(s['run'].encode()).hexdigest() for s in spec['steps']],
                    [hashlib.sha256(s['run'].encode()).hexdigest()
                     for s in historical['suites'][name]['steps']])

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
            with patch.object(run, 'worker_git_env', side_effect=lambda env, source: env), \
                    patch.object(run, 'execute', side_effect=RuntimeError('injected failure')):
                with ThreadPoolExecutor(max_workers=2) as pool:
                    rows = list(pool.map(lambda n: run.worker(n, root / n, root / 'shared', '14', {}, 0), names))
            self.assertEqual([r['exit'] for r in rows], [1, 1])
            self.assertTrue(all(r['error'] == 'injected failure' for r in rows))
            self.assertTrue(all((root / n / 'completion.json').is_file() for n in names))

    def test_zero_exit_without_all_workers_is_not_success(self):
        with tempfile.TemporaryDirectory() as tmp, patch.dict(os.environ, GITHUB_RUN_ID='1', GITHUB_RUN_ATTEMPT='1'):
            root = Path(tmp)
            source = dict(sha='head', tree='tree', run_id='1', attempt='1')
            for pack in ('a', 'b'):
                for compiler in ('14', '16'):
                    with self.subTest(pack=pack, compiler=compiler):
                        run.write(root / 'summary.json', dict(source=source, pack=pack, compiler=compiler,
                                  runner_slots=1, applicable_suites=run.suites(pack, True), workers=[], exit=0))
                        with self.assertRaisesRegex(RuntimeError, 'failed/incomplete'):
                            verify.verify(root, 'head', 'tree', pack, compiler, True)
                        with self.assertRaisesRegex(RuntimeError, 'source/tree'):
                            verify.verify(root, 'other', 'tree', pack, compiler, True)

    def test_deadline_retains_exit_and_log(self):
        with tempfile.TemporaryDirectory() as tmp:
            stem = Path(tmp) / 'timeout'
            with self.assertRaisesRegex(RuntimeError, 'timeout'):
                run.execute(['sleep', '10'], dict(os.environ), Path(tmp), stem, 0.1)
            self.assertEqual(json.loads(stem.with_suffix('.json').read_text())['reason'], 'timeout')
            self.assertTrue(stem.with_suffix('.exit').is_file())
            self.assertTrue(stem.with_suffix('.log').is_file())


class WorkerGitTrust(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.repository = self.root / 'source'
        self.other = self.root / 'other'
        self.job_home = self.root / 'job-home'
        self.home = self.root / 'worker-home'
        self.job_home.mkdir()
        self.home.mkdir()
        self.env = {k: v for k, v in os.environ.items() if not k.startswith('GIT_')}
        self.env.update(HOME=str(self.job_home), XDG_CONFIG_HOME=str(self.root / 'xdg'),
                        GIT_CONFIG_NOSYSTEM='1', PYTHONDONTWRITEBYTECODE='1')
        for repository in (self.repository, self.other):
            subprocess.run(['git', 'init', '-q', repository], env=self.env, check=True)
            subprocess.run(['git', '-c', 'user.name=Test', '-c', 'user.email=test@example.com',
                            'commit', '-q', '--allow-empty', '-m', repository.name],
                           cwd=repository, env=self.env, check=True)
        self.source = dict(sha=self.git('rev-parse', 'HEAD').stdout.strip(),
                           tree=self.git('rev-parse', 'HEAD^{tree}').stdout.strip(),
                           run_id='1', attempt='1')
        # Git's test hook works without root. In a mapped user namespace also
        # create a real ownership mismatch, without changing host-owned files.
        if os.geteuid() == 0 and os.environ.get('ADA_DEEP_TEST_CHOWN') == '1':
            for repository in (self.repository, self.other):
                os.chown(repository, 1, 1)
                os.chown(repository / '.git', 1, 1)
        else:
            self.env['GIT_TEST_ASSUME_DIFFERENT_OWNER'] = '1'
        self.git('config', '--global', '--add', 'safe.directory', str(self.repository))
        self.worker_env = dict(self.env, HOME=str(self.home))
        self.root_patch = patch.object(run, 'ROOT', self.repository)
        self.root_patch.start()
        self.addCleanup(self.root_patch.stop)

    def git(self, *args, env=None, cwd=None):
        return subprocess.run(['git', *args], cwd=cwd or self.repository,
                              env=self.env if env is None else env,
                              capture_output=True, text=True)

    def assert_untrusted(self, env, cwd=None):
        result = self.git('rev-parse', 'HEAD', env=env, cwd=cwd)
        self.assertEqual(result.returncode, 128, result.stderr)
        self.assertIn('detected dubious ownership', result.stderr)

    def test_job_global_trust_is_lost_with_private_home(self):
        self.assertEqual(self.git('rev-parse', 'HEAD').returncode, 0)
        self.assert_untrusted(self.worker_env)

    def test_exact_protected_trust_preserves_settings_and_rejects_other_repo(self):
        self.worker_env.update(GIT_CONFIG_COUNT='1', GIT_CONFIG_KEY_0='core.quotePath',
                               GIT_CONFIG_VALUE_0='false')
        original = dict(self.worker_env)
        (self.home / '.gitconfig').write_text('[user]\n\tname = Private worker\n')
        config = (self.home / '.gitconfig').read_bytes()
        trusted = run.worker_git_env(self.worker_env, self.source)
        self.assertEqual(self.worker_env, original)
        self.assertEqual((self.home / '.gitconfig').read_bytes(), config)
        self.assertEqual(self.git('config', 'user.name', env=trusted).stdout.strip(), 'Private worker')
        self.assertEqual(self.git('config', 'core.quotePath', env=trusted).stdout.strip(), 'false')
        entries = self.git('config', '--show-scope', '--get-all', 'safe.directory', env=trusted)
        self.assertEqual(entries.stdout.splitlines(), ['command\t', 'command\t' + str(self.repository)])
        for ref, expected in [('HEAD', self.source['sha']), ('HEAD^{tree}', self.source['tree'])]:
            self.assertEqual(self.git('rev-parse', ref, env=trusted).stdout.strip(), expected)
        self.assertEqual(self.git('status', '--porcelain', env=trusted).stdout, '')
        (self.repository / 'untracked').write_text('dirty source must remain visible')
        self.assertIn('?? untracked', self.git('status', '--porcelain', env=trusted).stdout)
        self.assert_untrusted(trusted, self.other)
        self.assert_untrusted(self.worker_env)

    def test_inherited_trust_is_reset_and_wrong_paths_fail_closed(self):
        self.worker_env.update(GIT_CONFIG_COUNT='1', GIT_CONFIG_KEY_0='safe.directory',
                               GIT_CONFIG_VALUE_0=str(self.other))
        trusted = run.worker_git_env(self.worker_env, self.source)
        self.assert_untrusted(trusted, self.other)
        for wrong in (self.root / 'missing', self.repository / '.git', self.other):
            with self.subTest(path=wrong), patch.object(run, 'ROOT', wrong):
                with self.assertRaises((RuntimeError, subprocess.CalledProcessError, FileNotFoundError)):
                    run.worker_git_env(self.worker_env, self.source)
        for field in ('sha', 'tree'):
            with self.subTest(field=field), self.assertRaisesRegex(RuntimeError, 'mismatch'):
                run.worker_git_env(self.worker_env, dict(self.source, **{field: '0' * 40}))
        nested = self.repository / 'nested'
        nested.mkdir()
        with patch.object(run, 'ROOT', nested), self.assertRaises(subprocess.CalledProcessError):
            run.worker_git_env(self.worker_env, self.source)

    def test_canonical_path_and_ambiguous_command_configuration(self):
        alias = self.root / 'alias'
        alias.symlink_to(self.repository, target_is_directory=True)
        with patch.object(run, 'ROOT', alias):
            trusted = run.worker_git_env(self.worker_env, self.source)
        self.assertEqual(trusted['GIT_CONFIG_VALUE_1'], str(self.repository))
        for overrides in ({'GIT_CONFIG_COUNT': '-1'}, {'GIT_CONFIG_COUNT': 'bad'},
                          {'GIT_CONFIG_PARAMETERS': "'safe.directory=*'"}):
            with self.subTest(overrides=overrides), self.assertRaises((RuntimeError, ValueError)):
                run.worker_git_env(dict(self.worker_env, **overrides), self.source)
        wildcard = self.root / 'source*'
        wildcard.mkdir()
        with patch.object(run, 'ROOT', wildcard), self.assertRaisesRegex(RuntimeError, 'wildcard'):
            run.worker_git_env(self.worker_env, self.source)

    def test_original_identity_commands_in_actual_worker_environment_all_cells(self):
        real_execute = run.execute
        for pack in ('a', 'b'):
            for compiler in ('14', '16'):
                for name in run.INVENTORY['packs'][pack]:
                    with self.subTest(pack=pack, compiler=compiler, suite=name):
                        directory = self.root / (name + '-' + compiler)
                        (directory / 'evidence/setup/home').mkdir(parents=True)
                        seen = []

                        def execute(command, env, cwd, stem, timeout, cpu):
                            if 'integration-identity.json' in command[-1]:
                                seen.append(env)
                                self.assertEqual(self.git('status', '--porcelain', env=env).returncode, 0)
                                self.assert_untrusted(env, self.other)
                                return real_execute(command, env, cwd, stem, timeout, cpu)
                            # Compiler campaigns and evidence are deliberately
                            # not fabricated as fresh qualification in this test.
                            return dict(command=command, exit=0, reason=None)

                        env = dict(self.worker_env, GITHUB_SHA=self.source['sha'],
                                   GITHUB_RUN_ID='1', GITHUB_RUN_ATTEMPT='1')
                        with patch.dict(os.environ, env, clear=True), \
                                patch.object(run, 'execute', side_effect=execute), \
                                patch.object(run, 'verify_suite', return_value={'test_only': True}), \
                                patch('builtins.print'):
                            result = run.worker(name, directory, self.root / 'shared', compiler,
                                                self.source, min(os.sched_getaffinity(0)))
                        self.assertEqual(result['exit'], 0, result.get('error'))
                        self.assertEqual(len(seen), 1)
                        receipt = json.loads((directory / 'evidence/integration-identity.json').read_text())
                        self.assertEqual(receipt, dict(sha=self.source['sha'], tree=self.source['tree'],
                                                      run_id='1', run_attempt='1', compiler=compiler))
                        # Trust cannot replace mandatory original evidence or artifacts.
                        with self.assertRaises((RuntimeError, ValueError, KeyError)):
                            run.verify_suite(name, directory, compiler, self.source)
                        with self.assertRaisesRegex(RuntimeError, 'no files'), patch.object(run, 'ROOT', self.other):
                            verify.artifact_files(name, self.root / 'absent', self.root / 'shared', compiler)
                        self.assertIn(directory / 'evidence/integration-identity.json',
                                      verify.artifact_files(name, directory, self.root / 'shared', compiler))

    def test_worker_source_failure_retains_completion_and_skips_commands(self):
        directory = self.root / 'failed-worker'
        directory.mkdir()
        with patch.dict(os.environ, self.worker_env, clear=True), \
                patch.object(run, 'execute') as execute, patch('builtins.print'):
            result = run.worker(run.INVENTORY['packs']['a'][0], directory, self.root / 'shared',
                                '14', dict(self.source, sha='0' * 40), 0)
        execute.assert_not_called()
        self.assertEqual(result['exit'], 1)
        self.assertIn('mismatch', result['error'])
        self.assertEqual(json.loads((directory / 'completion.json').read_text()), result)


if __name__ == '__main__':
    unittest.main()