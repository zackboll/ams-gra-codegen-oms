"""PY-010 mandatory additive Fast/Deep/Sleet controls; no lane/gate changes."""
import copy
import re
import unittest

import ci_policy
import ci_selection as selection
from py010_ci import FAST, DEEP, DEEP_SETUP, BASELINE, SLEET, SLEET_ARTIFACT
from py010_ci import project, project_sleet
import ci_select003
import py009_ci
from test_ci_select import change


class VisibleAsciiIntegration(unittest.TestCase):
    def setUp(self):
        self.workflows = {p.name: p.read_text() for p in
                          (selection.ROOT / '.github/workflows').glob('*.yml')}
        self.sleet = (selection.ROOT / 'scripts/run-real-sleet-test.sh').read_text()
        self.policy = selection.inventory()

    def test_current_contract(self):
        self.assertEqual(ci_policy.check(self.policy, self.workflows, self.sleet), 42)

    def test_projection_chain_preserves_each_independent_authority(self):
        projected = project(self.workflows)
        self.assertNotIn(FAST, projected['ci.yml'])
        self.assertIn(ci_select003.FAST, projected['ci.yml'])
        projected = ci_select003.project(projected)
        self.assertIn(py009_ci.FAST, projected['ci.yml'])
        self.assertIn(py009_ci.DEEP, projected['deep-ci.yml'])
        py009_ci.project(projected)
        sleet = project_sleet(self.sleet)
        self.assertIn(ci_select003.SLEET, sleet)
        sleet = ci_select003.project_sleet(sleet)
        self.assertIn(py009_ci.SLEET, sleet)
        py009_ci.project_sleet(sleet)
        # Missing owned hooks must reach their own verifier, not fail an
        # unrelated PY-010 adjacency test before it can inspect the defect.
        for workflow, hook in [('ci.yml', ci_select003.FAST),
                               ('deep-ci.yml', ci_select003.DEEP)]:
            mutated = dict(self.workflows)
            self.assertEqual(mutated[workflow].count(hook), 1)
            mutated[workflow] = mutated[workflow].replace(hook, '', 1)
            with self.assertRaisesRegex(ValueError, '^CI-SELECT-003 mandatory owned Python gate'):
                ci_policy.check(self.policy, mutated, self.sleet)
        with self.assertRaisesRegex(ValueError, '^CI-SELECT-003 mandatory per-interpreter'):
            ci_policy.check(self.policy, self.workflows,
                            self.sleet.replace(ci_select003.SLEET, '', 1))

    def test_task101_inventory_cannot_regress_to_40_runners_12_deep(self):
        counts = {workflow: sum(len(j['cells']) for j in self.policy['jobs']
                                if j['workflow'] == workflow)
                  for workflow in ('ci.yml', 'deep-ci.yml')}
        self.assertEqual(counts, {'ci.yml': 20, 'deep-ci.yml': 14})
        broken = copy.deepcopy(self.policy)
        broken['jobs'] = [j for j in broken['jobs'] if j['job_id'] != 'ada-real-uci-dlz-sleet']
        self.assertEqual(sum(len(j['cells']) for j in broken['jobs']
                             if j['workflow'] == 'deep-ci.yml'), 12)
        with self.assertRaisesRegex(ValueError, '^Task101 mandatory independent Deep job absent/duplicate$'):
            ci_policy.check(broken, self.workflows, self.sleet)
        for defect in ('compiler', 'artifact'):
            broken = copy.deepcopy(self.policy)
            task101 = next(j for j in broken['jobs'] if j['job_id'] == 'ada-real-uci-dlz-sleet')
            if defect == 'compiler':
                task101['cells'].pop()
            else:
                task101['cells'][0]['artifacts'] = []
            with self.assertRaisesRegex(ValueError, '^Task101 mandatory compiler/artifact/lane inventory changed$'):
                ci_policy.check(broken, self.workflows, self.sleet)

    def test_commands_cannot_be_removed_disabled_or_filtered(self):
        for workflow, addition in [('ci.yml', FAST), ('deep-ci.yml', DEEP)]:
            for line in addition.splitlines(keepends=True):
                for replacement in ['', line.replace('          ', '          true # ', 1),
                                    line.rstrip() + ' --skip datetime\n']:
                    if replacement == line:
                        continue
                    with self.subTest(line=line, replacement=replacement):
                        mutated = dict(self.workflows)
                        self.assertEqual(mutated[workflow].count(addition), 1)
                        self.assertEqual(addition.count(line), 1)
                        mutated[workflow] = mutated[workflow].replace(
                            addition, addition.replace(line, replacement, 1), 1)
                        with self.assertRaisesRegex(ValueError, 'mandatory PY-010'):
                            ci_policy.check(self.policy, mutated, self.sleet)

    def test_sleet_evidence_cannot_be_removed_disabled_or_moved(self):
        for replacement in ['', SLEET_ARTIFACT.replace('always()', 'false'),
                            SLEET_ARTIFACT.replace('if-no-files-found: error', 'if-no-files-found: ignore'),
                            SLEET_ARTIFACT.replace('py010-sleet/**', 'py009-sleet/**')]:
            mutated = dict(self.workflows)
            mutated['deep-ci.yml'] = mutated['deep-ci.yml'].replace(SLEET_ARTIFACT, replacement, 1)
            with self.assertRaisesRegex(ValueError, '^mandatory PY-010 shared-Sleet evidence changed$'):
                ci_policy.check(self.policy, mutated, self.sleet)
        mutated = dict(self.workflows)
        mutated['deep-ci.yml'] = mutated['deep-ci.yml'].replace(SLEET_ARTIFACT, '', 1) + SLEET_ARTIFACT
        with self.assertRaisesRegex(ValueError, '^mandatory PY-010 shared-Sleet evidence changed$'):
            ci_policy.check(self.policy, mutated, self.sleet)
        # A valid-looking upload in another real job is not owning evidence.
        mutated = dict(self.workflows)
        mutated['deep-ci.yml'] = mutated['deep-ci.yml'].replace(SLEET_ARTIFACT, '', 1)
        anchor = '  real-uci-regressions:\n'
        self.assertEqual(mutated['deep-ci.yml'].count(anchor), 1)
        mutated['deep-ci.yml'] = mutated['deep-ci.yml'].replace(anchor, SLEET_ARTIFACT + anchor, 1)
        with self.assertRaisesRegex(ValueError, '^mandatory PY-010 shared-Sleet evidence changed$'):
            ci_policy.check(self.policy, mutated, self.sleet)
        for replacement in ['', DEEP_SETUP.replace('3.13', '3.12')]:
            mutated = dict(self.workflows)
            mutated['deep-ci.yml'] = mutated['deep-ci.yml'].replace(DEEP_SETUP, replacement, 1)
            with self.assertRaisesRegex(ValueError, '^mandatory PY-010 pinned-profile interpreter setup changed$'):
                ci_policy.check(self.policy, mutated, self.sleet)

    def test_final_gate_and_infrastructure_stay_fail_closed(self):
        body = ci_policy.blocks(self.workflows['ci.yml'])[1]['pr-ci-required']
        original = '  pr-ci-required:\n' + body
        for replacement, reason in [('', 'unmapped/missing workflow job: ci.yml'),
                                    (original.replace('    if: ${{ always() }}', '    if: ${{ false }}'),
                                     'mandatory baseline/final gate changed: pr-ci-required')]:
            self.assertNotEqual(replacement, original)
            mutated = dict(self.workflows)
            self.assertEqual(mutated['ci.yml'].count(original), 1)
            mutated['ci.yml'] = mutated['ci.yml'].replace(original, replacement, 1)
            with self.assertRaisesRegex(ValueError, reason):
                ci_policy.check(self.policy, mutated, self.sleet)
        for path in ['.github/workflows/ci.yml', 'scripts/ci_select003.py',
                     'python/qualification/run.py', 'scripts/py010_ci.py', 'Cargo.lock']:
            result = selection.select([change(path)], self.policy)
            self.assertEqual(result['mode'], 'full-pr', path)
            self.assertTrue(all(result['jobs'].values()), path)
            self.assertEqual(result['deep_selected_cells'], 14, path)

    def test_final_gate_requires_fresh_unique_sleet_artifact(self):
        from test_ci_select import GateTests
        control = GateTests()
        control.setUp()
        control.validate()
        index = next(n for n, artifact in enumerate(control.artifacts) if artifact['name'] == 'py010-sleet')
        healthy = copy.deepcopy(control.artifacts[index])
        for mutation in [{'expired': True}, {'size_in_bytes': 0}, {'created_at': '2000-01-01T00:00:00Z'}]:
            control.artifacts[index].update(mutation)
            with self.assertRaisesRegex(ValueError, 'required evidence'):
                control.validate()
            control.artifacts[index] = copy.deepcopy(healthy)
        control.artifacts.pop(index)
        with self.assertRaisesRegex(ValueError, 'required evidence'):
            control.validate()
        control.artifacts.extend([healthy, healthy])
        with self.assertRaisesRegex(ValueError, 'required evidence'):
            control.validate()

    def test_successor_baseline_and_shared_broker_loop_required(self):
        mutated = dict(self.workflows)
        mutated['deep-ci.yml'] = mutated['deep-ci.yml'].replace(BASELINE, '', 1)
        with self.assertRaises(ValueError):
            ci_policy.check(self.policy, mutated, self.sleet)
        for replacement in ['', SLEET.replace('bash ', 'true # bash '), SLEET.rstrip() + ' --skip\n']:
            with self.assertRaisesRegex(ValueError, 'mandatory PY-010'):
                ci_policy.check(self.policy, self.workflows, self.sleet.replace(SLEET, replacement))

    def test_all_task_surfaces_select_both_python_versions_and_deep_owners(self):
        paths = ['crates/cli/tests/python_visible_ascii.rs', 'crates/backend-python/tests/visible_ascii.rs',
                 'crates/cli/examples/python_visible_ascii_profiles.rs', 'scripts/check-py010-parity.py',
                 'scripts/check-py010-uci.py', 'scripts/measure-py010-uci.py', 'scripts/test-py010-uci.py',
                 'scripts/check-py010-sleet.sh', 'scripts/py010_ci.py',
                 'scripts/test_ci_select_py010.py',
                 'python/oms-lacal-runtime/tests/test_visible_ascii_sleet.py']
        paths += [str(p.relative_to(selection.ROOT)) for directory in
                  ['crates/backend-python/src', 'tests/fixtures/python-visible-ascii']
                  for p in (selection.ROOT / directory).glob('*') if p.is_file()]
        for path in paths:
            with self.subTest(path=path):
                result = selection.select([change(path)], self.policy)
                self.assertTrue({'python-model-backend-3.11', 'python-model-backend-3.13'} <= set(result['cells']))
                self.assertTrue({'real-sleet', 'real-uci-regressions'} <= set(result['deep_jobs']))

    def test_commands_must_execute_in_owning_job_and_interpreter_loop(self):
        for workflow, addition in [('ci.yml', FAST), ('deep-ci.yml', DEEP)]:
            mutated = dict(self.workflows)
            self.assertEqual(mutated[workflow].count(addition), 1)
            mutated[workflow] = mutated[workflow].replace(addition, '', 1) + addition
            with self.assertRaisesRegex(ValueError, 'mandatory PY-010'):
                ci_policy.check(self.policy, mutated, self.sleet)
        moved = self.sleet.replace(SLEET, '', 1) + SLEET
        with self.assertRaisesRegex(ValueError, 'mandatory PY-010'):
            ci_policy.check(self.policy, self.workflows, moved)

    def test_additions_cannot_relocate_within_the_owning_job(self):
        cases = [
            ('ci.yml', FAST, '          python scripts/test-py007-uci.py\n', 'mandatory PY-010'),
            ('deep-ci.yml', DEEP, '          python3 python/qualification/run.py uci\n', 'mandatory PY-010'),
            ('deep-ci.yml', DEEP_SETUP,
             '      - name: Prepare regression inputs and release CLI\n',
             'mandatory PY-010 pinned-profile interpreter setup changed'),
            ('deep-ci.yml', SLEET_ARTIFACT,
             '      - name: Rust LA-CAL runtime against real pinned Sleet (e38f61d8)\n',
             'mandatory PY-010 shared-Sleet evidence changed'),
        ]
        for workflow, addition, anchor, reason in cases:
            with self.subTest(addition=addition):
                mutated = dict(self.workflows)
                job = ('python-model-backend' if workflow == 'ci.yml' else
                       'real-sleet' if addition == SLEET_ARTIFACT else 'real-uci-regressions')
                body = dict(re.findall(r'^  ([\w-]+):\n(.*?)(?=^  [\w-]+:|\Z)',
                                       mutated[workflow], re.M | re.S))[job]
                self.assertEqual(body.count(addition), 1)
                self.assertEqual(body.count(anchor), 1)
                changed = body.replace(addition, '', 1)
                if addition == FAST:
                    changed = changed.replace(anchor, anchor + addition, 1)
                else:
                    changed = changed.replace(anchor, addition + anchor, 1)
                mutated[workflow] = mutated[workflow].replace(body, changed, 1)
                self.assertNotEqual(mutated[workflow], self.workflows[workflow])
                with self.assertRaisesRegex(ValueError, reason):
                    ci_policy.check(self.policy, mutated, self.sleet)


if __name__ == '__main__':
    unittest.main()
