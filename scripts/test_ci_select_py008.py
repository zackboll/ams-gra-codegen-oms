"""PY-008 reconciliation: attributed, non-vacuous offline policy controls."""
import re
import unittest

import ci_policy
import ci_selection as selection
from test_ci_select import change, gate


class Float64Integration(unittest.TestCase):
    def setUp(self):
        self.policy = selection.inventory()
        self.workflows = {p.name: p.read_text() for p in
                          (selection.ROOT / '.github/workflows').glob('*.yml')}
        self.sleet = (selection.ROOT / 'scripts/run-real-sleet-test.sh').read_text()
        self.assertEqual(ci_policy.check(self.policy, self.workflows, self.sleet), 36)

    def reject(self, workflow, before, after, diagnostic):
        mutated = dict(self.workflows)
        self.assertEqual(mutated[workflow].count(before), 1, before)
        mutated[workflow] = mutated[workflow].replace(before, after, 1)
        self.assertNotEqual(mutated[workflow], self.workflows[workflow])
        with self.assertRaisesRegex(ValueError, diagnostic):
            ci_policy.check(self.policy, mutated, self.sleet)

    def test_every_float64_surface_selects_both_versions_and_deep(self):
        paths = set(ci_policy.PY008_SURFACES)
        paths.update(str(p.relative_to(selection.ROOT)) for p in
                     (selection.ROOT / 'tests/fixtures/python-float64').rglob('*') if p.is_file())
        for path in sorted(paths):
            with self.subTest(path=path):
                result = selection.select([change(path)], self.policy)
                self.assertTrue(result['deep'])
                self.assertTrue({'python-model-backend-3.11', 'python-model-backend-3.13'} <= set(result['cells']))

    def test_fast_and_deep_commands_cannot_be_removed_skipped_renamed_or_filtered(self):
        for workflow, job, command in (
                ('ci.yml', 'python-model-backend', 'cargo test --locked -p ams-gra-codegen-oms --test python_float64 -- --nocapture'),
                ('ci.yml', 'python-model-backend', 'python scripts/test-py008-uci.py'),
                ('ci.yml', 'python-model-backend', 'python scripts/check-py008-numeric.py --generated'),
                ('ci.yml', 'python-model-backend', 'python -O scripts/check-py008-numeric.py --generated'),
                ('deep-ci.yml', 'real-uci-regressions', 'python3 scripts/test-py008-uci.py'),
                ('deep-ci.yml', 'real-uci-regressions', 'python3 scripts/check-py008-uci.py --generator'),
                ('deep-ci.yml', 'real-sleet', '"$interpreter" scripts/check-py008-numeric.py --full'),
                ('deep-ci.yml', 'real-sleet', '"$interpreter" -O scripts/check-py008-numeric.py --full')):
            for replacement in ('', 'true # ' + command, command.replace('py008', 'py009').replace('python_float64', 'python_float32'), command + ' --skip float64'):
                with self.subTest(workflow=workflow, command=command, replacement=replacement):
                    self.reject(workflow, command, replacement,
                                'inherited qualification payload/evidence changed: ' + job)

    def test_all_version_and_optimization_cells_are_mandatory(self):
        self.reject('ci.yml', "python-version: ['3.11', '3.13']", "python-version: ['3.11']",
                    'qualification runner/matrix/dependency policy changed: python-model-backend')
        self.reject('deep-ci.yml', 'for interpreter in python3.11 python3.13; do',
                    'for interpreter in python3.11; do', 'payload/evidence changed: real-sleet')
        self.reject('deep-ci.yml', '"$interpreter" -O scripts/check-py008-numeric.py --full',
                    '"$interpreter" scripts/check-py008-numeric.py --full', 'payload/evidence changed: real-sleet')

    def test_shared_sleet_execution_cannot_be_removed_skipped_or_filtered(self):
        command = '  PYTHON="$interpreter" bash "$ROOT/scripts/check-py008-sleet.sh"'
        for replacement in ('', '  true # ' + command, command + ' --skip float64',
                            command.replace('py008', 'py009')):
            with self.subTest(replacement=replacement):
                self.assertEqual(self.sleet.count(command), 1)
                mutated = self.sleet.replace(command, replacement)
                self.assertNotEqual(mutated, self.sleet)
                with self.assertRaisesRegex(ValueError, 'PY-008 shared-Sleet execution changed'):
                    ci_policy.check(self.policy, self.workflows, mutated)
        for prefix in ('exit 0\n', 'if false; then\n'):
            mutated = prefix + self.sleet + ('\nfi\n' if 'if false' in prefix else '')
            with self.subTest(prefix=prefix), self.assertRaisesRegex(
                    ValueError, 'PY-008 shared-Sleet execution changed'):
                ci_policy.check(self.policy, self.workflows, mutated)

    def test_artifact_and_source_identity_cannot_change(self):
        for before, after, job in (
                ('name: py008-numeric', 'name: py008-unchecked', 'real-sleet'),
                ('path: ${{ runner.temp }}/py008-*.json', 'path: /tmp/unrelated.json', 'real-sleet'),
                ('name: py006-uci', 'name: py008-uci', 'real-uci-regressions')):
            self.reject('deep-ci.yml', before, after, 'payload/evidence changed: ' + job)
        body = re.search(r'^  real-uci-regressions:\n.*?(?=^  [\w-]+:|\Z)',
                         self.workflows['deep-ci.yml'], re.M | re.S)[0]
        altered = body.replace('test "$actual" = "$GITHUB_SHA"', 'true # source unchecked', 1)
        self.reject('deep-ci.yml', body, altered, 'payload/evidence changed: real-uci-regressions')

    def test_inherited_py007_aggregate_shards_and_security_stay_required(self):
        for workflow, command, diagnostic in (
                ('ci.yml', 'cargo test --locked -p ams-gra-codegen-oms --test python_inheritance -- --nocapture',
                 'payload/evidence changed: python-model-backend'),
                ('deep-ci.yml', 'python3 scripts/test-py007-uci.py', 'payload/evidence changed: real-uci-regressions'),
                ('ci.yml', '  pr-ci-required:\n', 'unmapped/missing workflow job: ci.yml'),
                ('ci002-validation.yml', '          persist-credentials: false', 'payload/evidence changed: test')):
            self.reject(workflow, command, '', diagnostic)
        for shard in ('real-uci-temporal-naming', 'real-uci-schema-ipv6', 'real-uci-integral-unicode'):
            self.reject('deep-ci.yml', '  ' + shard + ':\n', '  bypass-' + shard + ':\n',
                        'missing primary shard or surviving serial job')

    def test_aggregate_inventory_requires_numeric_and_uci_evidence(self):
        records = {r['job_id']: r for r in self.policy['jobs'] if r['workflow'] == 'deep-ci.yml'}
        for job, artifact in (('real-sleet', 'py008-numeric'), ('real-uci-regressions', 'py006-uci')):
            record = records[job]
            self.assertIn(artifact, record['cells'][0]['artifacts'])
            with self.assertRaisesRegex(ValueError, 'missing/expired/stale required evidence: ' + artifact):
                gate.validate_cells(record, [dict(name=job, status='completed', conclusion='success',
                    started_at='2026-10-29T00:00:00Z', steps=[dict(name=s.get('name', 'action'), conclusion='success')
                    for s in record['original_steps']])], [])


if __name__ == '__main__':
    unittest.main()