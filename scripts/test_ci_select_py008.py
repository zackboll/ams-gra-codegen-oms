"""PY-008 reconciliation: attributed, non-vacuous offline policy controls."""
import copy
import hashlib
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
        self.assertEqual(ci_policy.check(self.policy, self.workflows, self.sleet), 44)

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
                    for s in record.get('integration_steps', record['original_steps'])])], [])

    def test_reviewed_baseline_and_additive_payloads_are_separate(self):
        for key, digest in ci_policy.PY008_INTEGRATED_PAYLOADS.items():
            record = next(r for r in self.policy['jobs'] if (r['workflow'], r['job_id']) == key)
            self.assertEqual(hashlib.sha256(ci_policy.normalized(record['original_steps_yaml']).encode()).hexdigest(),
                             ci_policy.INHERITED_PAYLOADS[key])
            self.assertEqual(hashlib.sha256(ci_policy.normalized(record['integration_steps_yaml']).encode()).hexdigest(), digest)
            ci_policy.check_py008_integration(record)
            self.assertNotEqual(record['original_steps_yaml'], record['integration_steps_yaml'])
            mutated = copy.deepcopy(record)
            owner = next(s for s in mutated['integration_steps'] if s.get('run') and 'PY-008' not in s.get('name', ''))
            self.assertNotEqual(owner['run'], 'true\n')
            owner['run'] = 'true\n'
            with self.assertRaisesRegex(ValueError, 'PY-008 (mandatory additive command|integration removed/replaced inherited step)'):
                ci_policy.check_py008_integration(mutated)

    def test_inherited_test_cannot_be_replaced_and_repinned(self):
        for workflow, job, command in (
                ('ci.yml', 'python-model-backend', 'cargo test --locked -p ams-gra-codegen-oms --test python_inheritance -- --nocapture'),
                ('deep-ci.yml', 'real-uci-regressions', 'python3 scripts/test-py007-uci.py')):
            policy = copy.deepcopy(self.policy)
            record = next(r for r in policy['jobs'] if (r['workflow'], r['job_id']) == (workflow, job))
            self.assertEqual(record['integration_steps_yaml'].count(command), 1)
            record['integration_steps_yaml'] = record['integration_steps_yaml'].replace(command, 'true # unrelated success')
            with self.assertRaisesRegex(ValueError, 'PY-008 integration payload changed: ' + job):
                ci_policy.check(policy, self.workflows, self.sleet)
            record['original_steps_yaml'] = record['integration_steps_yaml']
            with self.assertRaisesRegex(ValueError, 'verified current-main baseline payload changed: ' + job):
                ci_policy.check(policy, self.workflows, self.sleet)

    def test_aggregate_step_arrays_cannot_drift_from_pinned_yaml(self):
        for key in ci_policy.PY008_INTEGRATED_PAYLOADS:
            for field in ('original_steps', 'integration_steps'):
                policy = copy.deepcopy(self.policy)
                record = next(r for r in policy['jobs'] if (r['workflow'], r['job_id']) == key)
                healthy = copy.deepcopy(record[field])
                record[field].pop()
                self.assertNotEqual(record[field], healthy)
                with self.assertRaisesRegex(ValueError, 'PY-008 aggregate step contract changed: ' + key[1]):
                    ci_policy.check(policy, self.workflows, self.sleet)

    def test_task096_oracle_retains_task100_and_python_ownership(self):
        for path in ('tests/ada-float64-probe/oracle.py', 'tests/ada-float64-probe/corpus-manifest.json'):
            manifest = selection.select([change(path)], self.policy)
            self.assertTrue(manifest['deep'])
            self.assertTrue({'python-model-backend-3.11', 'python-model-backend-3.13',
                             'ada-websocket-transport-14', 'ada-websocket-transport-16'} <= set(manifest['cells']))

    def test_task100_compiler_cells_and_artifacts_remain_mandatory(self):
        for workflow, job, kind in (('ci.yml', 'ada-websocket-transport', 'transport'),
                                    ('deep-ci.yml', 'ada-websocket-real-sleet', 'sleet')):
            for compiler in ('14', '16'):
                policy = copy.deepcopy(self.policy)
                record = next(r for r in policy['jobs'] if r['job_id'] == job)
                cell = next(c for c in record['cells'] if c['variant']['compiler'] == compiler)
                record['cells'].remove(cell)
                with self.assertRaisesRegex(ValueError, 'Task100 requires exactly GNAT14/GNAT16 cells and artifacts: ' + job):
                    ci_policy.check(policy, self.workflows, self.sleet)
                for artifacts in ([], [f'task100-ada-websocket-{kind}-{compiler}'] * 2):
                    policy = copy.deepcopy(self.policy)
                    record = next(r for r in policy['jobs'] if r['job_id'] == job)
                    cell = next(c for c in record['cells'] if c['variant']['compiler'] == compiler)
                    self.assertNotEqual(cell['artifacts'], artifacts)
                    cell['artifacts'] = artifacts
                    with self.assertRaisesRegex(ValueError, 'Task100 requires exactly GNAT14/GNAT16 cells and artifacts: ' + job):
                        ci_policy.check(policy, self.workflows, self.sleet)

    def test_primary_shards_cannot_be_skipped(self):
        for shard in ('real-uci-temporal-naming', 'real-uci-schema-ipv6', 'real-uci-integral-unicode'):
            self.reject('deep-ci.yml', '  ' + shard + ':\n', '  ' + shard + ':\n    if: false\n',
                        'primary shard.*conditional|qualification runner/matrix/dependency policy changed: ' + shard)

    def test_aggregate_evidence_and_security_cannot_be_bypassed(self):
        self.reject('ci.yml', 'python3 scripts/ci-required-gate.py', 'true # bypass evidence',
                    'mandatory baseline/final gate changed: pr-ci-required')
        policy = copy.deepcopy(self.policy)
        path = next(iter(policy['security_source_sha256']))
        self.assertNotEqual(policy['security_source_sha256'][path], '0' * 64)
        policy['security_source_sha256'][path] = '0' * 64
        with self.assertRaisesRegex(ValueError, 'missing/modified inherited CI002 source inventory'):
            ci_policy.check(policy, self.workflows, self.sleet)

    def test_rust_worker_join_sources_and_required_workspace_cannot_disappear(self):
        sources = {p: (selection.ROOT / p).read_bytes() for p in ci_policy.RUST_CI001_SOURCES}
        ci_policy.check_rust_ci001(sources)
        for path in sources:
            mutated = dict(sources)
            mutated[path] = b'// removed worker-join regression\n'
            self.assertNotEqual(mutated[path], sources[path])
            with self.assertRaisesRegex(ValueError, 'RUST-CI-001 worker-join qualification source changed: ' + re.escape(path)):
                ci_policy.check_rust_ci001(mutated)
        self.reject('ci.yml', '- run: cargo test --workspace', '- run: cargo test --workspace --exclude ams-gra-oms-runtime-rust-facade-tests',
                    'inherited qualification payload/evidence changed: rust')


if __name__ == '__main__':
    unittest.main()