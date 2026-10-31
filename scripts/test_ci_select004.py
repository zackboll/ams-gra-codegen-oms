"""Registration, source reconciliation, hook ownership and fail-closed controls."""
import copy
import importlib.util
import json
import os
import re
from pathlib import Path
import shutil
import tempfile
import unittest
from unittest.mock import patch

import ci_policy
import ci_selection
from ci_select004 import HOOKS, project, project_sleet, SLEET, PYTHON_ENV
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
                  ['.github/workflows/ci.yml', '.github/workflows/deep-ci.yml', 'scripts/ci_policy.py']}
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

    def test_selector_exact_owners_union_and_unsafe_fallback(self):
        policy = ci_selection.inventory()
        cases = [('qualification/ada/fast/ci004-smoke/owned_smoke.adb', {'ada-lacal-runtime-core'}, set()),
                 ('qualification/ada/uci/ci004-dlz-codec/cases.json', {'ada-lacal-runtime-core'}, {'ada-real-uci-dlz-sleet'}),
                 ('qualification/ada/sleet/ci004-dlz-transport/service.yaml', {'ada-lacal-runtime-core'}, {'ada-real-uci-dlz-sleet'}),
                 ('python/qualification/fast/fixtures/test_owned_fixture/values.json', {'python-model-backend'}, set())]
        cases.extend([
            ('python/qualification/uci/fixtures/test_owned_uci/values.json', {'python-model-backend'}, {'real-uci-regressions'}),
            ('python/qualification/sleet/fixtures/test_owned_transport/required.json', {'python-model-backend'}, {'real-sleet'})])
        with patch.object(ci_selection, 'ROOT', self.root):
            for path, jobs, deep in cases:
                result = ci_selection.select([changed(path)], policy)
                self.assertEqual(result['mode'], 'selective-pr')
                self.assertEqual({j for j, enabled in result['jobs'].items() if enabled}, jobs | {'rust'})
                self.assertEqual(set(result['deep_jobs']), deep)
            result = ci_selection.select([changed(case[0]) for case in cases], policy)
            self.assertEqual({j for j, enabled in result['jobs'].items() if enabled},
                             {'rust', 'ada-lacal-runtime-core', 'python-model-backend'})
            for path in ('qualification/ada/fast/not-registered/test.adb',
                         'qualification/ada/fast/ci004-smoke/../escape.adb',
                         'scripts/run-owned-ada.py'):
                self.assertEqual(ci_selection.select([changed(path)], policy)['mode'], 'full-pr')
            general = ci_selection.select([changed('crates/backend-ada/src/task_specific.rs')], policy)
            self.assertTrue(general['deep'])
            self.assertGreater(general['selected_cells'], 3)
            self.update(tests=[])
            self.assertEqual(ci_selection.select([changed(cases[0][0])], policy)['mode'], 'full-pr')


class HookContracts(unittest.TestCase):
    def test_fixed_hooks_are_mandatory_and_owned_by_existing_jobs(self):
        workflows = {p.name: p.read_text() for p in (ROOT / '.github/workflows').glob('*.yml')}
        project(workflows)
        self.assertEqual(ci_policy.check(), 42)
        for workflow, job, addition in HOOKS:
            for replacement in ('', addition.replace('if: always()', 'if: false'),
                                addition.replace('set -euo pipefail', 'true # disabled')):
                if replacement == addition:
                    continue
                mutated = dict(workflows)
                mutated[workflow] = mutated[workflow].replace(addition, replacement, 1)
                with self.subTest(job=job), self.assertRaises(ValueError):
                    project(mutated)
            mutated = dict(workflows)
            mutated[workflow] = mutated[workflow].replace(addition, '', 1) + addition
            with self.assertRaises(ValueError):
                project(mutated)
        text = (ROOT / 'scripts/run-real-sleet-test.sh').read_text()
        project_sleet(text)
        with self.assertRaises(ValueError):
            project_sleet(text.replace(SLEET, '', 1))

    def test_python_bytecode_prevention_is_mandatory_before_shared_tests(self):
        workflows = {p.name: p.read_text() for p in (ROOT / '.github/workflows').glob('*.yml')}
        for workflow, owner, _ in HOOKS:
            if owner.startswith('ada-'):
                continue
            body = dict(re.findall(r'^  ([\w-]+):\n(.*?)(?=^  [\w-]+:|\Z)',
                                   workflows[workflow], re.M | re.S))[owner]
            for replacement in ('', PYTHON_ENV.replace('BYTECODE=1', 'BYTECODE=0')):
                altered = dict(workflows)
                altered[workflow] = altered[workflow].replace(body, body.replace(PYTHON_ENV, replacement, 1), 1)
                with self.subTest(owner=owner), self.assertRaisesRegex(ValueError, 'bytecode-free'):
                    project(altered)


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

    def test_final_reconciliation_requires_artifacts_and_exact_job_ownership(self):
        spec = importlib.util.spec_from_file_location('ci004_gate', ROOT / 'scripts/ci-required-gate.py')
        gate = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(gate)
        manifest = dict(jobs={'ada-lacal-runtime-core': True}, deep_jobs=[])
        with self.assertRaisesRegex(ValueError, 'missing/duplicate'):
            gate.reconcile_owned([], manifest, 'a', 'b', '123', '2', jobs=[], attempts={})
        artifacts = [dict(name='task099-ada-lacal-runtime-core-' + c) for c in ('14', '16')]
        jobs, attempts = metadata(artifacts)
        for snapshot in attempts.values():
            if not snapshot['jobs']:
                continue
            for job, compiler in zip(snapshot['jobs'], ('14', '16'), strict=True):
                job['name'] = f'ada-lacal-runtime-core ({compiler})'
        for job, compiler in zip(jobs, ('14', '16'), strict=True):
            job['name'] = f'ada-lacal-runtime-core ({compiler})'
        with patch('owned_ada_evidence.verify_bundle', return_value=dict(verified=True)) as verify:
            summary = gate.reconcile_owned(artifacts, manifest, 'a', 'b', '123', '2',
                                           loader=lambda artifact: {}, jobs=jobs, attempts=attempts)
            self.assertEqual(set(summary), {a['name'] for a in artifacts})
            self.assertEqual(verify.call_count, 2)
        for mutation in ('wrong-owner', 'stale', 'failed', 'missing-artifact', 'unexpected-owner'):
            a, j, history = copy.deepcopy((artifacts, jobs, attempts))
            if mutation == 'wrong-owner':
                j[0]['name'] = 'python-model-backend (3.11)'
            elif mutation == 'stale':
                a[0]['created_at'] = '2026-10-27T01:08:30Z'
            elif mutation == 'failed':
                j[0]['steps'][0]['conclusion'] = 'failure'
            elif mutation == 'missing-artifact':
                a.pop()
            else:
                a.append(dict(name='ci004-python-sleet'))
            with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                gate.reconcile_owned(a, manifest, 'a', 'b', '123', '2', loader=lambda artifact: {},
                                     jobs=j, attempts=history)

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