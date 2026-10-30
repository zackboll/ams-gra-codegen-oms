"""Current-main inheritance, security, Task099 and final-source controls."""
import copy
import hashlib
import unittest
from unittest.mock import patch

import ci_policy
import ci_selection as selection
import test_ci_select as original

change, gate, reporter = original.change, original.gate, original.reporter


class Py007Integration(unittest.TestCase):
    def test_reconciliation_base_cannot_be_removed_or_replaced(self):
        for field in ('reconciliation_main', 'reconciliation_main_tree'):
            for replacement in (None, '0' * 40):
                policy = selection.inventory()
                if replacement is None:
                    del policy[field]
                else:
                    policy[field] = replacement
                with self.subTest(field=field, replacement=replacement), self.assertRaisesRegex(
                        ValueError, 'reconciliation base identity changed'):
                    ci_policy.check(policy=policy)

    def test_aggregate_requires_integrated_uci_step_name(self):
        policy = selection.inventory()
        record = next(r for r in policy['jobs'] if
                      (r['workflow'], r['job_id']) == ('deep-ci.yml', 'real-uci-regressions'))
        step = next(s for s in record['original_steps'] if 'PY-007' in s.get('name', ''))
        self.assertIn('scripts/check-py007-uci.py --generator', step['run'])
        self.assertIn('scripts/test-py007-uci.py', step['run'])
        self.assertIn(step['name'], record['original_steps_yaml'])
        fast = next(r for r in policy['jobs'] if
                    (r['workflow'], r['job_id']) == ('ci.yml', 'python-model-backend'))
        self.assertTrue(any('--test python_inheritance -- --nocapture' in s.get('run', '')
                            for s in fast['original_steps']))

    def test_every_inheritance_surface_selects_both_pythons_and_deep(self):
        for path in ('crates/backend-python/src/lib.rs',
                     'crates/backend-python/tests/models.rs',
                     'crates/cli/tests/python_inheritance.rs',
                     'tests/fixtures/python-inheritance/oam.xsd',
                     'python/oms-lacal-runtime/tests/test_inheritance_sleet.py',
                     'scripts/check-py007-uci.py', 'scripts/test-py007-uci.py',
                     'scripts/check-py007-sleet.sh'):
            with self.subTest(path=path):
                manifest = selection.select([change(path)], selection.inventory())
                self.assertTrue(manifest['deep'])
                self.assertTrue({'python-model-backend-3.11', 'python-model-backend-3.13'}
                                <= set(manifest['cells']))

    def test_inheritance_commands_cannot_be_removed_disabled_or_filtered(self):
        workflows = {p.name: p.read_text() for p in
                     (selection.ROOT / '.github/workflows').glob('*.yml')}
        for workflow, command in (
                ('ci.yml', 'cargo test --locked -p ams-gra-codegen-oms --test python_inheritance -- --nocapture'),
                ('ci.yml', 'python scripts/test-py007-uci.py'),
                ('deep-ci.yml', 'python3 scripts/test-py007-uci.py'),
                ('deep-ci.yml', 'python3 scripts/check-py007-uci.py --generator')):
            for replacement in ('', 'true # ' + command, command + ' --skip inheritance'):
                with self.subTest(workflow=workflow, command=command, replacement=replacement):
                    mutated = dict(workflows)
                    self.assertEqual(mutated[workflow].count(command), 1)
                    mutated[workflow] = mutated[workflow].replace(command, replacement)
                    with self.assertRaisesRegex(ValueError, 'payload/evidence changed'):
                        ci_policy.check(workflows=mutated)


class Task099Selection(unittest.TestCase):
    def test_runtime_fixtures_scripts_and_unknown_paths(self):
        for path in ('ada/oms-lacal-runtime/src/oms_lacal.ads',
                     'tests/ada-lacal-runtime-core/mock.adb',
                     'scripts/task099/qualify.py', 'tests/fixtures/task099/new.xsd',
                     'ada/new-lacal-runtime/unknown.ads'):
            with self.subTest(path=path):
                manifest = selection.select([change(path)], selection.inventory())
                self.assertTrue(manifest['jobs']['ada-lacal-runtime-core'])
                self.assertTrue({'ada-lacal-runtime-core-14', 'ada-lacal-runtime-core-16'} <= set(manifest['cells']))
                if 'unknown' in path or 'scripts/' in path or 'fixtures/' in path:
                    self.assertEqual(manifest['mode'], 'full-pr')


class Task099Gate(original.GateTests):
    def test_each_runtime_cell_and_evidence_required(self):
        for compiler in ('14', '16'):
            index = next(n for n, j in enumerate(self.jobs) if j['name'] == f'ada-lacal-runtime-core ({compiler})')
            original = copy.deepcopy(self.jobs[index])
            for status, conclusion in (('completed', 'failure'), ('completed', 'cancelled'), ('completed', 'skipped')):
                self.jobs[index].update(status=status, conclusion=conclusion)
                with self.assertRaises(ValueError):
                    self.validate()
            self.jobs[index] = copy.deepcopy(original)
            self.jobs[index]['steps'][0]['name'] = 'Not the compiler-backed qualification step'
            with self.assertRaisesRegex(ValueError, 'mandatory named step'):
                self.validate()
            self.jobs.pop(index)
            with self.assertRaises(ValueError):
                self.validate()
            self.jobs.insert(index, original)
            artifact = next(a for a in self.artifacts if a['name'] == f'task099-ada-lacal-runtime-core-{compiler}')
            self.artifacts.remove(artifact)
            with self.assertRaises(ValueError):
                self.validate()
            self.artifacts.append(artifact)
            artifact['created_at'] = '2020-01-01T00:00:00Z'
            with self.assertRaises(ValueError):
                self.validate()
            artifact['created_at'] = '2026-10-27T02:00:00Z'

    def test_final_source_and_expected_main_verified(self):
        sha, head, base = 'a' * 40, 'b' * 40, 'c' * 40
        manifest = dict(identities=dict(merge=sha, head=head, base=base))
        run = dict(event='pull_request', head_sha=head)
        with patch.object(gate, 'git', return_value=(sha + '\n').encode()) as git, patch.object(gate, 'complete_diff') as diff:
            gate.verify_source(manifest, self.policy, run, sha, 'pull_request')
            diff.assert_called_once_with(selection.ROOT, base, head, sha)
            self.assertIn(unittest.mock.call(selection.ROOT, 'merge-base', '--is-ancestor', self.policy['incorporated_main'], base), git.call_args_list)
            for source in (sha, base):
                self.assertIn(unittest.mock.call(selection.ROOT, 'merge-base', '--is-ancestor', self.policy['reconciliation_main'], source), git.call_args_list)
            with self.assertRaises(ValueError):
                gate.verify_source(manifest, self.policy, dict(event='pull_request', head_sha=base), sha, 'pull_request')
        with patch.object(gate, 'git', return_value=(head + '\n').encode()), self.assertRaises(ValueError):
            gate.verify_source(manifest, self.policy, run, sha, 'pull_request')


class R1Policy(original.SchedulingTests):
    def test_security_and_runtime_mutations_rejected(self):
        cases = [
            ('oms-trusted-pr-signal.yml', 'permissions: {}', 'permissions:\n  contents: write'),
            ('oms-trusted-pr-signal.yml', 'types: [opened, synchronize, reopened, ready_for_review, edited]', 'types: [opened]'),
            ('oms-ryzen-trusted-dispatch.yml', "vars.OMS_TRUSTED_PR_DISPATCH_ENABLED == 'true'", 'true'),
            ('oms-ryzen-trusted-dispatch.yml', 'python3 scripts/ci002/dispatch.py', 'true'),
            ('oms-ryzen-trusted-dispatch.yml', '  workflow_run:', '  pull_request_target:'),
            ('oms-ryzen-trusted-dispatch.yml', '      queue: max', '      queue: single'),
            ('ci002-validation.yml', 'checks: read', 'checks: write'),
            ('ci002-validation.yml', 'python3 -m unittest discover', 'echo'),
            ('ci002-validation.yml', '      queue: max', '      queue: single'),
            ('ci.yml', "compiler: ['14', '16']", "compiler: ['14', '17']"),
            ('ci.yml', 'TASK099_HOSTED_REQUIRED_CELLS', 'TASK099_NOT_REQUIRED'),
            ('ci.yml', "c['checks'] != (62 if", "c['checks'] != (0 if"),
        ]
        original = copy.deepcopy(self.workflows)
        for workflow, before, after in cases:
            self.workflows = copy.deepcopy(original)
            self.assertIn(before, self.workflows[workflow])
            self.workflows[workflow] = self.workflows[workflow].replace(before, after, 1)
            with self.subTest(workflow=workflow, before=before), self.assertRaises(ValueError):
                self.check()

    def test_required_workflows_and_runtime_inventory_cannot_disappear(self):
        for workflow in self.policy['security_headers']:
            original = self.workflows.pop(workflow)
            with self.assertRaises(ValueError):
                self.check()
            self.workflows[workflow] = original
        runtime = next(r for r in self.policy['jobs'] if r['job_id'] == 'ada-lacal-runtime-core')
        runtime['cells'].pop()
        with self.assertRaises(ValueError):
            self.check()

    def test_modified_candidate_cannot_be_blessed_as_inherited_payload(self):
        runtime = next(r for r in self.policy['jobs'] if r['job_id'] == 'ada-lacal-runtime-core')
        runtime['original_steps_yaml'] = runtime['original_steps_yaml'].replace('TASK099_QUALIFICATION_COMPLETE', 'true')
        with self.assertRaisesRegex(ValueError, 'baseline payload changed'):
            self.check()

    def test_verified_main_is_the_inherited_baseline_not_candidate(self):
        main = self.policy['incorporated_main']
        self.assertEqual(main, 'd603077c6d38ccc0658391776e3af9eecb009255')
        for record in self.policy['jobs']:
            if record.get('inherited_from') != main:
                continue
            # The digests were derived from the verified main git object during
            # reconciliation, not from this candidate. No history is required:
            # inherited Rust/Pages qualification uses a shallow checkout.
            key = record['workflow'], record['job_id']
            digest = hashlib.sha256(ci_policy.normalized(record['original_steps_yaml']).encode()).hexdigest()
            self.assertEqual(digest, ci_policy.INHERITED_PAYLOADS[key])

    def test_comments_and_unrelated_metadata_are_harmless(self):
        self.workflows = {name: '# harmless comment\n' + text for name, text in self.workflows.items()}
        self.assertEqual(self.check(), 42)
        # Inventory descriptions do not influence selection or authorization.
        self.policy['description'] = 'Review note'
        self.assertEqual(self.check(), 42)


class LaneTimestamps(unittest.TestCase):
    def test_overlap_within_one_lane_rejected_even_below_global_limit(self):
        jobs = [dict(id=n, name=str(n), status='completed', conclusion='success',
                     completed_at='2026-10-27T00:01:00Z',
                     steps=[dict(started_at='2026-10-27T00:00:00Z', conclusion='success')]) for n in range(2)]
        with self.assertRaisesRegex(ValueError, 'multiple running jobs in lane'):
            reporter.report(jobs, lanes={0: 1, 1: 1})
        self.assertEqual(reporter.report(jobs, lanes={0: 1, 1: 2})['maximum_running'], 2)
        with self.assertRaisesRegex(ValueError, 'incomplete lane'):
            reporter.report(jobs, lanes={0: 1})
        with self.assertRaisesRegex(ValueError, 'invalid shared main lane'):
            reporter.report(jobs, lanes={0: 1, 1: 6})


if __name__ == '__main__':
    unittest.main()