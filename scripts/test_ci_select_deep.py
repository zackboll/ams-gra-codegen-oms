"""CI-SELECT-002 ownership, wire contract, job skip and immutable policy tests."""
import copy
import json
import unittest
from unittest.mock import patch

import ci_policy
import ci_selection as selection
from test_ci_select import GateTests, change, gate


class DeepSelection(unittest.TestCase):
    def setUp(self):
        self.policy = selection.inventory()
        self.all = set(selection.deep_jobs(self.policy))

    def test_reviewed_ownership_and_complete_cell_partition(self):
        cases = {
            'README.md': set(),
            'python/oms-lacal-runtime/src/oms_lacal/client.py': {'real-sleet', 'real-uci-regressions'},
            'tests/fixtures/python-inheritance/oam.xsd': {'real-sleet', 'real-uci-regressions'},
            'crates/backend-python/src/lib.rs': {'msrv-real-uci', 'real-sleet', 'real-uci-regressions'},
            'tests/ada-websocket/transport_probe.adb': {'ada-websocket-real-sleet', 'ada-real-uci-dlz-sleet'},
            'ada/oms-lacal-runtime/websocket/oms_ws_handshake.ads': {'ada-websocket-real-sleet', 'ada-real-uci-dlz-sleet'},
            'ada/oms-lacal-runtime/oms_lacal_websocket.gpr': {'ada-websocket-real-sleet', 'ada-real-uci-dlz-sleet'},
            'crates/runtime-rust-facade-tests/tests/task077_sleet.rs': {'msrv-real-uci', 'real-sleet'},
            'crates/runtime-rust/src/lib.rs': {'msrv-real-uci', 'real-sleet', 'real-uci-temporal-naming',
                                               'real-uci-schema-ipv6', 'real-uci-integral-unicode',
                                               'real-uci-regressions-alternating'},
            'crates/runtime-api-rust/src/lib.rs': {'msrv-real-uci', 'real-sleet', 'real-uci-temporal-naming',
                                                   'real-uci-schema-ipv6', 'real-uci-integral-unicode',
                                                   'real-uci-regressions-alternating'},
            'crates/ir/src/lib.rs': self.all,
            'crates/codegen-core/src/lib.rs': self.all,
            'crates/backend-rust/src/lib.rs': self.all,
            '.github/workflows/ci.yml': self.all,
            'scripts/task100/qualify.py': self.all,
            'scripts/check-py007-sleet.sh': self.all,
            'unknown-file': self.all,
        }
        for path, wanted in cases.items():
            with self.subTest(path=path):
                result = selection.select([change(path)], self.policy)
                selection.validate_deep(result, self.policy)
                self.assertEqual(set(result['deep_jobs']), wanted)
                self.assertEqual(result['deep_selected_cells'] + result['deep_skipped_cells'], 14)
                self.assertEqual(result['deep_jobs'], sorted(wanted))
                self.assertEqual(set(result['reasons'][0]['deep_jobs']), wanted)
                self.assertEqual(result['deep'], bool(wanted))

    def test_main_manual_schedule_and_fallback_are_full(self):
        for event in ('push', 'workflow_dispatch', 'schedule'):
            result = selection.classify(selection.ROOT, {}, event, 'a' * 40, self.policy)
            self.assertEqual(set(result['deep_jobs']), self.all)
            self.assertEqual(result['deep_selected_cells'], 14)
            self.assertTrue(all(result['jobs'].values()))
        for c in (change('README.md', 'D'), change('README.md', mode='100755'),
                  {'paths': []}, {'paths': ['README.md']}, change('python/new.py', 'R100')):
            self.assertEqual(set(selection.select([c], self.policy)['deep_jobs']), self.all)

    def test_union_is_deterministic_and_matrix_is_indivisible(self):
        paths = ['tests/ada-websocket/transport_probe.adb', 'crates/backend-python/src/lib.rs']
        a = selection.select([change(p) for p in paths], self.policy)
        b = selection.select([change(p) for p in reversed(paths)], self.policy)
        for field in ('jobs', 'cells', 'deep_jobs', 'deep_selected_cells', 'deep_skipped_cells'):
            self.assertEqual(a[field], b[field])
        self.assertEqual(a['deep_selected_cells'], 7)

    def test_dependency_closure_transitive(self):
        records = copy.deepcopy(selection.deep_jobs(self.policy))
        records['real-sleet']['prerequisites'] = ['msrv-real-uci']
        records['msrv-real-uci']['prerequisites'] = ['real-uci-regressions']
        self.assertEqual(selection.close_prerequisites(['real-sleet'], records),
                         {'real-sleet', 'msrv-real-uci', 'real-uci-regressions'})

    def test_incomplete_rule_fails_broad(self):
        for replacement in (None, [], ['unknown-job']):
            policy = copy.deepcopy(self.policy)
            rule = next(r for r in policy['rules'] if r['category'] == 'python-backend')
            if replacement is None:
                del rule['deep_jobs']
            else:
                rule['deep_jobs'] = replacement
            result = selection.select([change('crates/backend-python/src/lib.rs')], policy)
            self.assertEqual(result['mode'], 'full-pr')
            self.assertEqual(set(result['deep_jobs']), self.all)

    def test_malformed_manifest_rejected_by_own_guard(self):
        healthy = selection.select([change('crates/backend-python/src/lib.rs')], self.policy)
        mutations = [
            ('deep_jobs', ['unknown']), ('deep_jobs', ['real-sleet', 'real-sleet']),
            ('deep_jobs', ['real-uci-regressions', 'real-sleet']), ('deep_jobs', 'real-sleet'),
            ('deep_jobs', [None]), ('deep_jobs', ['real-sleet; false']),
            ('deep', False), ('deep', 1), ('version', 1),
            ('deep_selected_cells', 2), ('deep_skipped_cells', 10),
        ]
        selection.validate_deep(healthy, self.policy)
        for field, value in mutations:
            result = copy.deepcopy(healthy)
            result[field] = value
            with self.subTest(field=field, value=value), self.assertRaises(ValueError):
                selection.validate_deep(result, self.policy)
        for field in healthy:
            if field.startswith('deep'):
                result = copy.deepcopy(healthy)
                del result[field]
                with self.subTest(missing=field), self.assertRaises(ValueError):
                    selection.validate_deep(result, self.policy)
        result = selection.select([], self.policy, full_main=True)
        result.update(deep_jobs=['real-sleet'], deep_selected_cells=1, deep_skipped_cells=13)
        with self.assertRaisesRegex(ValueError, 'full qualification omitted'):
            selection.validate_deep(result, self.policy)
        result = copy.deepcopy(healthy)
        result.update(deep_jobs=['real-uci-schema-ipv6'], deep_selected_cells=1, deep_skipped_cells=13)
        with self.assertRaisesRegex(ValueError, 'dependency closure'):
            selection.validate_deep(result, self.policy)


class SelectiveDeepGate(unittest.TestCase):
    def setUp(self):
        GateTests.setUp(self)
        self.full_jobs = copy.deepcopy(self.jobs)
        self.full_artifacts = copy.deepcopy(self.artifacts)
        self.narrow('crates/backend-python/src/lib.rs')

    def narrow(self, path):
        self.manifest = selection.select([change(path)], self.policy)
        self.needs = {j: {'result': 'success' if enabled else 'skipped'}
                      for j, enabled in self.manifest['jobs'].items()}
        self.needs.update({j: {'result': 'success'} for j in ('changes', 'baseline')})
        self.needs['deep-qualification'] = {'result': 'success' if self.manifest['deep'] else 'skipped'}
        self.jobs, self.artifacts = [], []
        for record in self.policy['jobs']:
            if record['workflow'] not in ('ci.yml', 'deep-ci.yml'):
                continue
            deep = record['workflow'] == 'deep-ci.yml'
            selected = (record['job_id'] in self.manifest['deep_jobs'] if deep
                        else self.manifest['jobs'][record['job_id']])
            if selected:
                prefix = 'deep-qualification / ' if deep else ''
                names = {prefix + c['name'] for c in record['cells']}
                self.jobs.extend(copy.deepcopy(j) for j in self.full_jobs if j['name'] in names)
                names = {n for c in record['cells'] for n in c['artifacts']}
                self.artifacts.extend(copy.deepcopy(a) for a in self.full_artifacts if a['name'] in names)
            elif deep and self.manifest['deep']:
                self.jobs.append(dict(name='deep-qualification / ' + record['job_id'],
                                      status='completed', conclusion='skipped', steps=[], runner_id=0))

    def validate(self):
        GateTests.validate(self)

    def test_healthy_full_and_docs(self):
        for path in ('crates/backend-python/src/lib.rs', 'tests/ada-websocket/transport_probe.adb', 'README.md'):
            self.narrow(path)
            self.validate()

    def test_selected_deep_missing_skipped_failed_cancelled_or_no_steps(self):
        self.validate()
        for name in ['deep-qualification / ' + j for j in self.manifest['deep_jobs']]:
            index = next(n for n, j in enumerate(self.jobs) if j['name'] == name)
            healthy = copy.deepcopy(self.jobs[index])
            for conclusion in ('skipped', 'failure', 'cancelled'):
                self.jobs[index]['conclusion'] = conclusion
                with self.subTest(name=name, conclusion=conclusion), self.assertRaisesRegex(ValueError, 'cell not successful'):
                    self.validate()
            self.jobs[index] = copy.deepcopy(healthy)
            self.jobs[index]['steps'] = []
            with self.assertRaisesRegex(ValueError, 'qualification step'):
                self.validate()
            self.jobs.pop(index)
            with self.assertRaisesRegex(ValueError, 'authoritative matrix cell'):
                self.validate()
            self.jobs.insert(index, healthy)

    def test_unselected_execution_artifacts_missing_skip_and_expanded_matrix(self):
        self.validate()
        name = 'deep-qualification / ada-websocket-real-sleet'
        index = next(n for n, j in enumerate(self.jobs) if j['name'] == name)
        healthy = copy.deepcopy(self.jobs[index])
        for mutation in ({'conclusion': 'success'}, {'runner_id': 123},
                         {'steps': [{'name': 'qualification', 'conclusion': 'success'}]}):
            self.jobs[index].update(mutation)
            with self.assertRaisesRegex(ValueError, 'unselected Deep job executed'):
                self.validate()
            self.jobs[index] = copy.deepcopy(healthy)
        self.jobs.pop(index)
        with self.assertRaisesRegex(ValueError, 'missing/ambiguous skipped Deep'):
            self.validate()
        self.jobs.insert(index, healthy)
        for compiler in ('14', '16'):
            self.jobs.append(dict(healthy, name=name + ' (' + compiler + ')'))
        with self.assertRaisesRegex(ValueError, 'missing/ambiguous skipped Deep'):
            self.validate()
        self.jobs = self.jobs[:-2]
        self.artifacts.append(dict(name='task100-ada-websocket-sleet-14'))
        with self.assertRaisesRegex(ValueError, 'unexpected unselected Deep artifact'):
            self.validate()

    def test_hosted_literal_matrix_skip_has_no_runner_steps_or_artifacts(self):
        # Actual GitHub API record: run 40821921150, job 123276175940.
        name = 'deep-qualification / ada-websocket-real-sleet (${{ matrix.compiler }})'
        index = next(n for n, j in enumerate(self.jobs)
                     if j['name'] == 'deep-qualification / ada-websocket-real-sleet')
        self.jobs[index] = dict(name=name, status='completed', conclusion='skipped',
                                steps=[], runner_id=None, runner_name=None)
        self.validate()
        healthy = copy.deepcopy(self.jobs[index])
        for mutation in ({'conclusion': 'success'}, {'runner_id': 123},
                         {'steps': [{'name': 'qualification', 'conclusion': 'success'}]}):
            self.jobs[index].update(mutation)
            with self.subTest(mutation=mutation), self.assertRaisesRegex(
                    ValueError, 'unselected Deep job executed'):
                self.validate()
            self.jobs[index] = copy.deepcopy(healthy)
        self.artifacts.append(dict(name='task100-ada-websocket-sleet-16'))
        with self.assertRaisesRegex(ValueError, 'unexpected unselected Deep artifact'):
            self.validate()

    def test_literal_matrix_skip_rejects_duplicates_wrong_axes_and_selected_cells(self):
        name = 'deep-qualification / ada-websocket-real-sleet (${{ matrix.compiler }})'
        index = next(n for n, j in enumerate(self.jobs)
                     if j['name'] == 'deep-qualification / ada-websocket-real-sleet')
        healthy = copy.deepcopy(self.jobs[index])
        self.jobs.append(dict(healthy, name=name, runner_id=None))
        with self.assertRaisesRegex(ValueError, 'missing/ambiguous skipped Deep'):
            self.validate()
        self.jobs.pop()
        for wrong in ('(${{ matrix.python-version }})', '(${{ matrix.compiler}})', '(14, 16)'):
            self.jobs[index]['name'] = 'deep-qualification / ada-websocket-real-sleet ' + wrong
            with self.subTest(wrong=wrong), self.assertRaisesRegex(
                    ValueError, 'missing/ambiguous skipped Deep'):
                self.validate()
        self.narrow('tests/ada-websocket/transport_probe.adb')
        self.jobs = [j for j in self.jobs if not j['name'].startswith(
            'deep-qualification / ada-websocket-real-sleet')]
        self.jobs.append(dict(healthy, name=name))
        with self.assertRaisesRegex(ValueError, 'authoritative matrix cell'):
            self.validate()

    def test_selected_evidence_expired_stale_duplicate_missing_and_step_skip(self):
        self.validate()
        index = next(n for n, a in enumerate(self.artifacts) if a['name'] == 'py006-uci')
        healthy = copy.deepcopy(self.artifacts[index])
        for mutation in ({'expired': True}, {'created_at': '2000-01-01T00:00:00Z'}, {'size_in_bytes': 0}):
            self.artifacts[index].update(mutation)
            with self.assertRaisesRegex(ValueError, 'required evidence'):
                self.validate()
            self.artifacts[index] = copy.deepcopy(healthy)
        self.artifacts.append(copy.deepcopy(healthy))
        with self.assertRaisesRegex(ValueError, 'required evidence'):
            self.validate()
        self.artifacts.pop()
        self.artifacts.pop(index)
        with self.assertRaisesRegex(ValueError, 'required evidence'):
            self.validate()
        self.artifacts.insert(index, healthy)
        job = next(j for j in self.jobs if j['name'] == 'deep-qualification / real-uci-regressions')
        job['steps'][0]['conclusion'] = 'skipped'
        with self.assertRaisesRegex(ValueError, 'qualification step'):
            self.validate()

    def test_docs_cannot_hide_executed_deep(self):
        self.narrow('README.md')
        self.validate()
        self.jobs.append(next(copy.deepcopy(j) for j in self.full_jobs
                              if j['name'] == 'deep-qualification / real-sleet'))
        with self.assertRaisesRegex(ValueError, 'unselected Deep job executed'):
            self.validate()


class DeepPolicy(unittest.TestCase):
    def setUp(self):
        self.workflows = {p.name: p.read_text() for p in (selection.ROOT / '.github/workflows').glob('*.yml')}

    def test_every_deep_condition_removed_inverted_bypassed(self):
        self.assertEqual(ci_policy.check(workflows=self.workflows), 42)
        for job_id, record in selection.deep_jobs(selection.inventory()).items():
            line = '    if: ' + record['selection_condition'] + '\n'
            for replacement in ('', '    if: true\n', line.replace('contains(', '!contains('),
                                line.replace('version != 2', 'version == 2')):
                workflows = dict(self.workflows)
                self.assertEqual(workflows['deep-ci.yml'].count(line), 1)
                workflows['deep-ci.yml'] = workflows['deep-ci.yml'].replace(line, replacement)
                with self.subTest(job=job_id, replacement=replacement), self.assertRaisesRegex(
                        ValueError, 'qualification runner/matrix/dependency policy changed'):
                    ci_policy.check(workflows=workflows)

    def test_reusable_input_cannot_be_changed_or_bypassed(self):
        for workflow, before, after, guard in (
                ('ci.yml', 'selection: ${{ needs.changes.outputs.manifest }}', 'selection: \'{}\'', 'complete Deep dependency'),
                ('deep-ci.yml', '        type: string', '        type: boolean', 'Deep reusable input'),
                ('deep-ci.yml', '        default: \'\'', '        default: \'{}\'', 'Deep reusable input')):
            workflows = dict(self.workflows)
            self.assertIn(before, workflows[workflow])
            workflows[workflow] = workflows[workflow].replace(before, after)
            with self.assertRaisesRegex(ValueError, guard):
                ci_policy.check(workflows=workflows)

    def test_candidate_inventory_cannot_bless_removed_cells_commands_or_dependencies(self):
        for kind in ('cells', 'commands', 'dependencies', 'ownership'):
            policy = selection.inventory()
            record = selection.deep_jobs(policy)['real-uci-schema-ipv6']
            if kind == 'cells':
                record['cells'].clear()
            elif kind == 'commands':
                record['original_steps_yaml'] = '    steps:\n      - run: true\n'
            elif kind == 'dependencies':
                record['prerequisites'].clear()
            else:
                policy['rules'][0]['deep_jobs'] = ['real-sleet']
            with self.subTest(kind=kind), self.assertRaisesRegex(ValueError, 'Deep'):
                ci_policy.check(policy=policy, workflows=self.workflows)


class AuthoritativeIdentity(unittest.TestCase):
    def test_each_record_source_run_attempt_and_duplicate_identity(self):
        execution = dict(id=123, run_attempt=2, head_sha='a' * 40)
        healthy = dict(jobs=[dict(id=1, run_id=123, run_attempt=2, head_sha='a' * 40)],
                       artifacts=[dict(id=2, workflow_run=dict(id=123, head_sha='a' * 40))])
        gate.verify_records(healthy, execution)
        for field, value in (('run_id', 456), ('run_attempt', 1), ('head_sha', 'b' * 40)):
            records = copy.deepcopy(healthy)
            records['jobs'][0][field] = value
            with self.subTest(field=field), self.assertRaisesRegex(ValueError, 'job source/run/attempt'):
                gate.verify_records(records, execution)
        for field, value in (('id', 456), ('head_sha', 'b' * 40)):
            records = copy.deepcopy(healthy)
            records['artifacts'][0]['workflow_run'][field] = value
            with self.subTest(field=field), self.assertRaisesRegex(ValueError, 'artifact source/run'):
                gate.verify_records(records, execution)
        for kind in ('jobs', 'artifacts'):
            records = copy.deepcopy(healthy)
            records[kind].append(copy.deepcopy(records[kind][0]))
            with self.assertRaisesRegex(ValueError, 'duplicate authoritative'):
                gate.verify_records(records, execution)


class FinalReportContract(unittest.TestCase):
    def test_written_and_printed_result_preserve_manifest_version_and_cells(self):
        for docs in (False, True):
            fixture = GateTests()
            fixture.setUp()
            manifest = fixture.manifest
            if docs:
                manifest = selection.select([change('README.md')], fixture.policy)
                fixture.needs = {j: dict(result='skipped') for j in manifest['jobs']}
                fixture.needs.update({j: dict(result='success') for j in ('changes', 'baseline')})
                fixture.needs['deep-qualification'] = dict(result='skipped')
                fixture.jobs, fixture.artifacts = [], []
            manifest['identities'] = dict(base='a' * 40, head='b' * 40, merge='c' * 40)
            environment = dict(SELECTION_MANIFEST=json.dumps(manifest),
                               NEEDS_JSON=json.dumps(fixture.needs), GITHUB_RUN_ID='123',
                               GITHUB_RUN_ATTEMPT='1', GITHUB_SHA='c' * 40,
                               GITHUB_EVENT_NAME='pull_request', GITHUB_JOB='pr-ci-required')
            records = dict(jobs=fixture.jobs, artifacts=fixture.artifacts, attempts={})
            with self.subTest(docs=docs), patch.dict('os.environ', environment), \
                    patch.object(gate, 'from_environment', return_value=manifest), \
                    patch.object(gate, 'api', return_value=dict(id=123, run_attempt=1)), \
                    patch.object(gate, 'verify_source'), patch.object(gate, 'verify_records'), \
                    patch.object(gate, 'verify_final_job'), \
                    patch.object(gate, 'evidence', return_value=records), \
                    patch.object(gate, 'git', return_value=('d' * 40 + '\n').encode()), \
                    patch.object(gate, 'reconcile_task100', return_value={}), \
                    patch.object(gate, 'reconcile_task101', return_value={}), \
                    patch.object(gate, 'reconcile_owned', return_value={}), \
                    patch.object(gate, 'reconcile_primary', return_value={}), \
                    patch.object(gate, 'Path') as path, patch('builtins.print') as output:
                gate.main()
                self.assertEqual(path.call_count, 2)
                path.assert_called_with('ci-required-result.json')
                path.return_value.unlink.assert_called_once_with(missing_ok=True)
                report = json.loads(path.return_value.write_text.call_args.args[0])
                printed = json.loads(output.call_args.args[1])
                self.assertEqual(output.call_args.args[0], 'PR_CI_REQUIRED_COMPLETE')
                self.assertEqual(printed, report)
                for field in ('version', 'identities', 'deep_jobs', 'selected_cells',
                              'skipped_cells', 'deep_selected_cells', 'deep_skipped_cells'):
                    self.assertEqual(report[field], manifest[field], field)
                self.assertEqual(report['run_id'], '123')
                self.assertEqual(report['gate_attempt'], '1')
                gate.verify_records.assert_called_once_with(records, dict(id=123, run_attempt=1))
                gate.verify_final_job.assert_called_once_with(
                    fixture.jobs, dict(id=123, run_attempt=1), 'pr-ci-required')
                gate.reconcile_task100.assert_called_once_with(
                    fixture.artifacts, manifest, 'c' * 40, 'd' * 40, '123', '1',
                    jobs=fixture.jobs, attempts={})
                gate.reconcile_owned.assert_called_once_with(
                    fixture.artifacts, manifest, 'c' * 40, 'd' * 40, '123', '1',
                    jobs=fixture.jobs, attempts={})
                if docs:
                    gate.reconcile_primary.assert_not_called()
                else:
                    gate.reconcile_primary.assert_called_once_with(
                        fixture.artifacts, 'c' * 40, 'd' * 40, '123', '1',
                        jobs=fixture.jobs, attempts={})
                self.assertEqual(report['version'], 2)
                self.assertEqual(report['deep_selected_cells'], 0 if docs else 14)
                self.assertEqual(report['conclusion'], 'success')


if __name__ == '__main__':
    unittest.main()