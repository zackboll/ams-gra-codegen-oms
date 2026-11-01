"""Registration, source reconciliation, hook ownership and fail-closed controls."""
import copy
import importlib.util
import json
import os
import re
from pathlib import Path
import shlex
import shutil
import tempfile
import unittest
from unittest.mock import patch

from workflow_text import blocks
HOOKS = (('ci.yml', 'ada-lacal-runtime-core', '      - name: Execute source-bound Ada Fast registrations\n        shell: bash\n        env:\n          PYTHONDONTWRITEBYTECODE: \'1\'\n        run: python3 scripts/run-owned-ada.py fast --compiler \'${{ matrix.compiler }}\' --setup /tmp/task099-evidence/setup --generator "$GITHUB_WORKSPACE/target/debug/ams-gra-codegen-oms" --output /tmp/ci004-ada-fast\n'), ('ci.yml', 'python-model-backend', '      - name: Execute source-bound Python Fast inventory\n        run: |\n          set -euo pipefail\n          python scripts/run-owned-python.py fast\n          python -O scripts/run-owned-python.py fast\n      - name: Upload mandatory source compiler and execution evidence\n        if: always()\n        uses: actions/upload-artifact@v4\n        with:\n          name: ci004-python-fast-${{ matrix.python-version }}\n          path: ${{ runner.temp }}/ci004-python-fast/**\n          if-no-files-found: error\n'), ('deep-ci.yml', 'ada-real-uci-dlz-sleet', '      - name: Execute source-bound Ada pinned registrations with shared broker\n        shell: bash\n        env:\n          PYTHONDONTWRITEBYTECODE: \'1\'\n        run: |\n          set -euo pipefail\n          /tmp/task101-venv/bin/python scripts/run-owned-ada.py pinned --compiler \'${{ matrix.compiler }}\' --setup /tmp/task101-evidence/setup --generator "$GITHUB_WORKSPACE/target/release/ams-gra-codegen-oms" --output /tmp/ci004-ada-pinned --sleet /tmp/task101-sleet --shared-campaign /tmp/task101-evidence/campaign --uci25 "$(cat /tmp/task101-evidence/uci25-root.txt)" --uci26 "$(cat /tmp/task101-evidence/uci26-root.txt)"\n'), ('deep-ci.yml', 'real-uci-regressions', '      - name: Execute source-bound Python pinned UCI inventory in both interpreters\n        run: |\n          set -euo pipefail\n          for ci004_interpreter in python3.11 python3.13; do\n            "$ci004_interpreter" scripts/run-owned-python.py uci\n            "$ci004_interpreter" -O scripts/run-owned-python.py uci\n          done\n      - name: Upload mandatory source compiler and execution evidence\n        uses: actions/upload-artifact@v4\n        with:\n          name: ci004-python-uci\n          path: ${{ runner.temp }}/ci004-python-uci/**\n          if-no-files-found: error\n'), ('deep-ci.yml', 'real-sleet', '      - name: Upload mandatory source compiler and execution evidence\n        if: always()\n        uses: actions/upload-artifact@v4\n        with:\n          name: ci004-python-sleet\n          path: ${{ runner.temp }}/ci004-python-sleet/**\n          if-no-files-found: error\n'))
import owned_qualification as owned
from owned_ada_evidence import verify_bundle
from ci_retry_test_data import metadata
from owned_python_evidence import verify_bundle as verify_python

ROOT = owned.ROOT


def changed(path, **kwargs):
    return dict(paths=[path], status='M', old_mode='100644', new_mode='100644', **kwargs)


class Registration(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix='ci004-registration-')
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        shutil.copytree(ROOT / 'qualification', self.root / 'qualification')
        shutil.copytree(ROOT / 'python/qualification', self.root / 'python/qualification')
        self.task = self.root / 'qualification/ada/fast/ci004-smoke'
        self.manifest = self.task / 'manifest.json'

    def update(self, **fields):
        item = json.loads(self.manifest.read_text())
        item.update(fields)
        self.manifest.write_text(json.dumps(item))

    def test_source_only_new_task_and_owned_fixture_need_no_policy_edits(self):
        before = {p: (ROOT / p).read_bytes() for p in
                  ['.github/workflows/ci.yml', '.github/workflows/deep-ci.yml', 'scripts/owned_qualification.py']}
        new = self.task.with_name('future-task')
        shutil.copytree(self.task, new)
        item = json.loads((new / 'manifest.json').read_text())
        item['task_id'] = 'future-task'
        (new / 'manifest.json').write_text(json.dumps(item))
        records = owned.registrations(self.root)
        self.assertEqual(len(records), len(owned.registrations(ROOT)) + 1)
        self.assertEqual(owned.ownership('qualification/ada/fast/future-task/owned_smoke.adb', self.root)['jobs'],
                         ['rust', 'ada-lacal-runtime-core'])
        self.assertEqual(before, {p: (ROOT / p).read_bytes() for p in before})

    def test_missing_duplicate_unknown_and_unsafe_inputs_rejected(self):
        original = self.manifest.read_text()
        cases = [dict(command='true'), dict(url='https://example.test/code'), dict(compiler='16'),
                 dict(version=True), dict(task_id='other'), dict(main='../escape.adb'),
                 dict(sources=['owned_smoke.adb', 'owned_smoke.adb']),
                 dict(tests=['checked-arithmetic']), dict(profile='expected-failure')]
        for fields in cases:
            with self.subTest(fields=fields):
                self.manifest.write_text(original)
                self.update(**fields)
                with self.assertRaises(ValueError):
                    owned.registrations(self.root)
        self.manifest.write_text(original.replace('"version": 1', '"version": 1, "version": 1'))
        with self.assertRaisesRegex(ValueError, 'duplicate JSON'):
            owned.registrations(self.root)
        self.manifest.write_text(original)
        (self.task / 'unexpected.sh').write_text('exit 0')
        with self.assertRaises(ValueError):
            owned.registrations(self.root)
        (self.task / 'unexpected.sh').unlink()
        (self.task / 'owned_smoke.adb').unlink()
        with self.assertRaises(ValueError):
            owned.registrations(self.root)

    def test_symlinks_hardlinks_executables_and_ambiguous_owners_rejected(self):
        source = self.task / 'owned_smoke.adb'
        backup = self.root / 'backup.adb'
        source.rename(backup)
        source.symlink_to(backup)
        with self.assertRaises(ValueError):
            owned.registrations(self.root)
        source.unlink()
        os.link(backup, source)
        with self.assertRaises(ValueError):
            owned.registrations(self.root)
        source.unlink()
        backup.rename(source)
        source.chmod(0o755)
        with self.assertRaises(ValueError):
            owned.registrations(self.root)
        source.chmod(0o644)
        shutil.copytree(self.task, self.root / 'qualification/ada/uci/ci004-smoke')
        with self.assertRaises(ValueError):
            owned.registrations(self.root)

    def test_python_fixture_inventory_and_source_test_omission(self):
        base = self.root / 'python/qualification/fast'
        manifest = base / 'fixtures/test_owned_fixture/manifest.json'
        original = manifest.read_text()
        for fields in [dict(files=['../escape.json']), dict(tests=[]), dict(shell='true'),
                       dict(files=['values.json'])]:
            item = json.loads(original)
            item.update(fields)
            manifest.write_text(json.dumps(item))
            with self.subTest(fields=fields), self.assertRaises(ValueError):
                owned.python_fixtures(self.root)
        manifest.write_text(original)
        source = base / 'test_owned_fixture.py'
        source.write_text(source.read_text() + '\ndef load_tests(loader, tests, pattern): return tests\n')
        with self.assertRaisesRegex(ValueError, 'filter'):
            owned.python_fixtures(self.root)



class HookContracts(unittest.TestCase):
    def check_ada_generators_built_by_owners(self, workflows, hooks):
        for workflow, owner, hook in hooks:
            if not owner.startswith('ada-'):
                continue
            with tempfile.TemporaryDirectory(prefix='ci004-generator-') as directory:
                workspace = Path(directory)
                body = blocks(workflows[workflow])[1][owner]
                self.assertEqual(body.count(hook), 1)
                preparation = body.split(hook, 1)[0]
                builds = re.findall(r'^\s*(?:run: )?(cargo(?: \+\S+)? build [^\n]+)$', preparation, re.M)
                generators = []
                for build in builds:
                    argv = shlex.split(build)
                    if '-p' in argv and argv[argv.index('-p') + 1] == 'ams-gra-codegen-oms':
                        profile = 'release' if '--release' in argv else 'debug'
                        generators.append(workspace / 'target' / profile / 'ams-gra-codegen-oms')
                self.assertEqual(len(generators), 1, 'expected one owning-job generator build')
                built, = generators
                built.parent.mkdir(parents=True)
                built.write_bytes(b'owning-job build fixture, not an execution qualification\n')
                paths = re.findall(r'--generator "\$GITHUB_WORKSPACE/([^"\n]+)"', hook)
                self.assertEqual(len(paths), 1)
                requested = workspace / paths[0]
                # Never consult ROOT/target: a pre-existing developer build must
                # not hide a hook requesting a profile its owner never built.
                self.assertTrue(requested.is_file(), 'generator not built by owning job: ' + paths[0])
                self.assertEqual(requested, built)

    def test_fixed_ada_hooks_use_generators_built_by_owning_jobs(self):
        workflows = {p.name: p.read_text() for p in (ROOT / '.github/workflows').glob('*.yml')}
        self.check_ada_generators_built_by_owners(workflows, HOOKS)

    def test_pinned_debug_generator_rejected_without_local_build_fallback(self):
        workflows = {p.name: p.read_text() for p in (ROOT / '.github/workflows').glob('*.yml')}
        hooks = []
        for workflow, owner, hook in HOOKS:
            if owner == 'ada-real-uci-dlz-sleet':
                wrong = hook.replace('target/release/', 'target/debug/')
                self.assertNotEqual(wrong, hook)
                workflows[workflow] = workflows[workflow].replace(hook, wrong, 1)
                hook = wrong
            hooks.append((workflow, owner, hook))
        # Mutate the workflow AND its fixed contract so exact-text agreement
        # alone cannot bless a generator absent from the owning job's build.
        with self.assertRaisesRegex(AssertionError, 'generator not built by owning job'):
            self.check_ada_generators_built_by_owners(workflows, hooks)




class Evidence(unittest.TestCase):
    def fixture(self):
        records = [r for r in owned.ada() if r['mode'] == 'fast']
        expected = [c for r in records for c in owned.cells(r)]
        files = {'gnat-version.stdout': b'GNAT 14.2.0\n', 'owned-source.stdout': b'a\nb\n',
                 'owned-final-source.stdout': b'a\nb\n', 'owned-final-clean.stdout': b'',
                 'owned-clean.stdout': b'', 'generator.sha256': b'cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc\n'}
        files['tool-identities.stdout'] = ''.join('/bin/' + name + '\n' + 'd'*64 + '  /bin/' + name + '\n'
                                                  for name in ('gnat', 'gcc', 'gprbuild')).encode()
        steps = []
        for label in ('printenv', 'gnat-version', 'gprbuild-version', 'owned-source', 'owned-clean', 'tool-identities',
                      'owned-final-source', 'owned-final-clean'):
            steps.append(dict(label=label, exit=0))
            for suffix in ('.command.json', '.stdout', '.stderr', '.exit'):
                files.setdefault(label + suffix, b'0\n' if suffix == '.exit' else b'')
        for record in records:
            for policy in owned.POLICIES:
                for optimization in owned.OPTIMIZATIONS:
                    label = record['manifest']['task_id'] + '-' + policy + '-O' + optimization
                    for action in ('build', 'execute'):
                        steps.append(dict(label=label + '-' + action, exit=0))
                        files[label + '-' + action + '.exit'] = b'0\n'
                        files[label + '-' + action + '.stderr'] = b''
                        files[label + '-' + action + '.stdout'] = b''
                        files[label + '-' + action + '.command.json'] = json.dumps(dict(argv=['gprbuild', '-gnato', '-O' + optimization, '-gnatec=policy.adc'])).encode()
                    files[label + '-execute.stdout'] = ''.join('CI004_PASS ' + test + '\n'
                                                               for test in record['manifest']['tests']).encode()
                    files[label + '/policy.adc'] = ('' if policy == 'Normal' else f'pragma Assertion_Policy ({policy});\n').encode()
                    for name in record['manifest']['sources']:
                        files[label + '/' + name] = (ROOT / record['path'] / name).read_bytes()
        result = dict(version=1, completed=True, registrations=records, required_cells=expected,
                      actual_cells=expected, steps=steps,
                      identity=dict(sha='a', tree='b', run_id='123', run_attempt='2', compiler='14',
                                    compiler_version='14.2.0', mode='fast', generator_sha256='c'*64),
                      files={p: owned.digest(data) for p, data in files.items()})
        files['result.json'] = json.dumps(result).encode()
        return files

    def verify(self, files):
        return verify_bundle(files, 'fast', '14', 'a', 'b', '123', '2')

    def test_omitted_skipped_forged_wrong_compiler_stale_and_masked_receipts(self):
        original = self.fixture()
        self.assertEqual(self.verify(original)['cells'], sum(len(owned.cells(r)) for r in owned.ada()
                                                             if r['mode'] == 'fast'))
        for field, value in [('actual_cells', []), ('required_cells', []), ('completed', False),
                             ('registrations', []), ('steps', []), ('files', {})]:
            files = dict(original)
            result = json.loads(files['result.json'])
            result[field] = value
            files['result.json'] = json.dumps(result).encode()
            with self.subTest(field=field), self.assertRaises(ValueError):
                self.verify(files)
        for key, value in [('compiler', '16'), ('run_attempt', '1'), ('sha', 'stale'), ('mode', 'pinned')]:
            files = dict(original)
            result = json.loads(files['result.json'])
            result['identity'][key] = value
            files['result.json'] = json.dumps(result).encode()
            with self.subTest(key=key), self.assertRaises(ValueError):
                self.verify(files)
        for path in list(original):
            files = dict(original)
            del files[path]
            with self.subTest(missing=path), self.assertRaises((ValueError, KeyError)):
                self.verify(files)
        files = dict(original)
        path = 'ci004-smoke-Normal-O0-execute.stdout'
        files[path] = b'CI004_PASS checked-arithmetic\n'
        result = json.loads(files['result.json'])
        result['files'][path] = owned.digest(files[path])
        files['result.json'] = json.dumps(result).encode()
        with self.assertRaisesRegex(ValueError, 'skipped'):
            self.verify(files)


    def test_python_receipts_reject_omitted_skipped_forged_stale_wrong_pins(self):
        fixtures = [r for r in owned.python_fixtures() if r['mode'] == 'uci']
        sources = sorted((ROOT / 'python/qualification/uci').glob('test_*.py'))
        tests = sorted(test for source in sources for test in owned.python_ids(source))
        hashes = {p.relative_to(ROOT).as_posix(): owned.digest(p.read_bytes()) for p in sources}
        for record in fixtures:
            hashes.update(record['hashes'])
        from task101_evidence import PINS, INCLUDES
        original = {}
        for version in ('3.11', '3.13'):
            for policy in ('normal', 'optimized'):
                prefix = version + '-' + policy + '/'
                log = b'synthetic verifier fixture, not execution qualification\n'
                result = dict(version=1, identity=dict(sha='a', tree='b', run_id='123', run_attempt='2',
                    mode='uci', python=version, python_version=version + '.0 fixture', optimization=policy,
                    interpreter_sha256='c'*64, generator_sha256='d'*64, uci_roots=PINS, uci_includes=INCLUDES),
                    hashes=hashes, fixtures=fixtures, test_ids=tests, log_sha256=owned.digest(log),
                    execution=dict(status='success', reasons=[], tests_registered=len(tests), tests_run=len(tests),
                        tests_passed=len(tests), failures=0, errors=0, skipped=0, expected_failures=0,
                        unexpected_successes=0, executed_test_ids=tests, passed_test_ids=tests))
                original[prefix + 'result.json'] = json.dumps(result).encode()
                original[prefix + 'execution.log'] = log
        self.assertEqual(verify_python(original, 'uci', ['3.11', '3.13'], 'a', 'b', '123', '2')['tests'], len(tests))
        path = '3.11-normal/result.json'
        for section, key, value in [('identity', 'python', '3.12'), ('identity', 'sha', 'stale'),
                                    ('identity', 'run_attempt', '1'), ('identity', 'uci_roots', {}),
                                    ('identity', 'uci_includes', {}), ('identity', 'mode', 'sleet'),
                                    ('execution', 'skipped', 1), ('execution', 'expected_failures', 1),
                                    ('execution', 'failures', 1), ('execution', 'tests_run', 0),
                                    ('execution', 'executed_test_ids', [])]:
            files = dict(original)
            result = json.loads(files[path])
            result[section][key] = value
            files[path] = json.dumps(result).encode()
            with self.subTest(section=section, key=key), self.assertRaises(ValueError):
                verify_python(files, 'uci', ['3.11', '3.13'], 'a', 'b', '123', '2')
        for path in original:
            files = dict(original)
            del files[path]
            with self.subTest(path=path), self.assertRaises((ValueError, KeyError)):
                verify_python(files, 'uci', ['3.11', '3.13'], 'a', 'b', '123', '2')


if __name__ == '__main__':
    unittest.main()