"""Task100 dependency selection and adversarial four-cell final gate tests."""
import copy
import itertools
import json
import hashlib
import unittest
from unittest.mock import patch

import ci_selection as selection
import task100_evidence
from ci_retry_test_data import metadata
import test_ci_select as original


class Task100Selection(unittest.TestCase):
    def test_transport_runtime_json_and_fixtures_select_both_qualifications(self):
        paths = ('ada/oms-lacal-runtime/src/oms_lacal-transport.ads',
                 'ada/oms-lacal-runtime/oms_lacal_websocket.gpr',
                 'tests/ada-websocket/transport_probe.adb',
                 'scripts/task100/qualify.py',
                 'tests/fixtures/service-generate/lacal-websocket.xsd',
                 'tests/fixtures/service-generate/lacal-runtime.yaml',
                 'ada/oms-json-support/lossless/ams_gra_oms_json.adb',
                 'ada/oms-json-support/float32/ams_gra_float32.c',
                 'ada/oms-json-support/src/ams_gra_oms_json.ads',
                 'ada/unknown-websocket/transport.ads', 'unknown-path')
        for path in paths:
            with self.subTest(path=path):
                manifest = selection.select([original.change(path)], selection.inventory())
                self.assertTrue(manifest['jobs']['ada-websocket-transport'])
                self.assertTrue(manifest['jobs']['ada-lacal-runtime-core'])
                self.assertTrue(manifest['deep'])
                self.assertTrue({'ada-websocket-transport-14', 'ada-websocket-transport-16'} <= set(manifest['cells']))

    def test_workflows_full_pr_and_main_always_complete(self):
        policy = selection.inventory()
        for manifest in (selection.select([original.change('.github/workflows/ci.yml')], policy),
                         selection.select([], policy, full_main=True)):
            self.assertTrue(all(manifest['jobs'].values()))
            self.assertTrue(manifest['deep'])
        self.assertEqual(sum(len(r['cells']) for r in policy['jobs']) + len(policy['orchestration_jobs']), 48)
        self.assertEqual(sum(len(r['cells']) for r in policy['jobs'] if not r.get('main_eligible', True)), 1)


class Task100Gate(original.GateTests):
    def test_every_cell_missing_failed_cancelled_skipped_duplicate_or_stale(self):
        for kind, job_id in (('transport', 'ada-websocket-transport'), ('sleet', 'ada-websocket-real-sleet')):
            for compiler in ('14', '16'):
                prefix = 'deep-qualification / ' if kind == 'sleet' else ''
                index = next(n for n, j in enumerate(self.jobs) if j['name'] == prefix + f'{job_id} ({compiler})')
                healthy = copy.deepcopy(self.jobs[index])
                for status, conclusion in (('completed', 'failure'), ('completed', 'cancelled'),
                                           ('completed', 'skipped'), ('pending', None)):
                    with self.subTest(kind=kind, compiler=compiler, conclusion=conclusion):
                        self.jobs[index].update(status=status, conclusion=conclusion)
                        with self.assertRaises(ValueError):
                            self.validate()
                self.jobs[index] = copy.deepcopy(healthy)
                self.jobs.append(copy.deepcopy(healthy))
                with self.assertRaises(ValueError):
                    self.validate()
                self.jobs.pop()
                self.jobs.pop(index)
                with self.assertRaises(ValueError):
                    self.validate()
                self.jobs.insert(index, copy.deepcopy(healthy))
                for mutation in ('skipped', 'missing', 'duplicate'):
                    step = self.jobs[index]['steps'][-1]
                    if mutation == 'skipped':
                        step['conclusion'] = 'skipped'
                    elif mutation == 'missing':
                        self.jobs[index]['steps'].pop()
                    else:
                        self.jobs[index]['steps'].append(copy.deepcopy(step))
                    with self.assertRaises(ValueError):
                        self.validate()
                    self.jobs[index] = copy.deepcopy(healthy)
                artifact = next(a for a in self.artifacts if a['name'] == f'task100-ada-websocket-{kind}-{compiler}')
                self.artifacts.remove(artifact)
                with self.assertRaises(ValueError):
                    self.validate()
                self.artifacts.extend([artifact, copy.deepcopy(artifact)])
                with self.assertRaises(ValueError):
                    self.validate()
                self.artifacts.pop()
                for key, value in (('created_at', '2020-01-01T00:00:00Z'), ('expired', True), ('size_in_bytes', 0)):
                    saved = artifact[key]
                    artifact[key] = value
                    with self.assertRaises(ValueError):
                        self.validate()
                    artifact[key] = saved


class Task100Policy(original.SchedulingTests):
    def test_either_workflow_compiler_cell_cannot_be_omitted(self):
        healthy = copy.deepcopy(self.workflows)
        for workflow in ('ci.yml', 'deep-ci.yml'):
            for compiler in ('14', '16'):
                self.workflows = copy.deepcopy(healthy)
                matrix = "compiler: ['14', '16']"
                self.assertIn(matrix, self.workflows[workflow])
                other = '16' if compiler == '14' else '14'
                self.workflows[workflow] = self.workflows[workflow].replace(
                    matrix, f"compiler: ['{other}']", 1)
                with self.subTest(workflow=workflow, omitted=compiler), self.assertRaises(ValueError):
                    self.check()

    def test_missing_or_weakened_security_source_inventory_rejected(self):
        healthy = copy.deepcopy(self.policy)
        for mutation in ('missing', 'empty', 'removed-file', 'changed-digest'):
            self.policy = copy.deepcopy(healthy)
            if mutation == 'missing':
                del self.policy['security_source_sha256']
            elif mutation == 'empty':
                self.policy['security_source_sha256'] = {}
            elif mutation == 'removed-file':
                self.policy['security_source_sha256'].pop('scripts/ci002/gate.py')
            else:
                self.policy['security_source_sha256']['scripts/ci002/gate.py'] = '0' * 64
            with self.assertRaises(ValueError):
                self.check()

    def test_original_and_integrated_payloads_cannot_be_blessed(self):
        for field in ('original_steps_yaml', 'integration_steps_yaml'):
            record = next(r for r in self.policy['jobs'] if r['job_id'] == 'ada-websocket-real-sleet')
            saved = record[field]
            record[field] = saved.replace('TASK100_HOSTED_REQUIRED_CELLS', 'true')
            with self.assertRaises(ValueError):
                self.check()
            record[field] = saved

    def test_missing_inventory_compiler_artifact_or_aggregate_rejected(self):
        healthy = copy.deepcopy(self.policy)
        for job_id in ('ada-websocket-transport', 'ada-websocket-real-sleet'):
            for mutation in ('job', 'cell', 'artifact'):
                self.policy = copy.deepcopy(healthy)
                record = next(r for r in self.policy['jobs'] if r['job_id'] == job_id)
                if mutation == 'job':
                    self.policy['jobs'].remove(record)
                elif mutation == 'cell':
                    record['cells'].pop()
                else:
                    record['cells'][0]['artifacts'] = []
                with self.assertRaises(ValueError):
                    self.check()
        self.policy = healthy
        self.policy['orchestration_jobs']['pr-ci-required']['prerequisites'].remove('ada-websocket-transport')
        with self.assertRaises(ValueError):
            self.check()


def bundle(compiler='14', real_sleet=True):
    files = {}

    def put(path, value):
        files[path] = (value if isinstance(value, str) else json.dumps(value)).encode()

    version = {'14': '14.2.0', '16': '16.1.0'}[compiler]
    put('integration-identity.json', dict(sha='a'*40, tree='b'*40, run_id='123', run_attempt='2', compiler=compiler))
    put('source.txt', 'a'*40 + '\n' + 'b'*40 + '\n')
    put('campaign/source.stdout', 'a'*40 + '\n' + 'b'*40 + '\n')
    put('campaign/source-status.stdout', '')
    put('campaign/compiler.stdout', 'GNAT ' + version + '\n')
    for name, (revision, _) in task100_evidence.DEPENDENCIES.items():
        put(f'campaign/{name}-identity.stdout', revision + '\n')
        put(f'campaign/{name}-clean.stdout', '')
        put(f'campaign/deps/{name}/LICENSE', 'test license')
    put('campaign/source.ads', 'test source')
    sources = {'source.ads': hashlib.sha256(b'test source').hexdigest()}
    for path in task100_evidence.UPSTREAM_SOURCES:
        put('campaign/' + path, 'upstream source')
        sources[path] = hashlib.sha256(b'upstream source').hexdigest()
    put('campaign/sources.json', sources)
    put('campaign/deps/aht/LICENSES/Apache-2.0.txt', 'test SPDX license')
    cells, steps, log = [], [], []
    for n, p, a, o in itertools.product(('oam', 'qualified'), ('portable', 'lossless'), ('Normal', 'Check', 'Ignore'), ('0', '2')):
        cell = f'{n}-{p}-{a}-O{o}'
        cells.append(dict(namespace=n, profile=p, policy=a, optimization=o))
        log.append(f'TASK100_CELL_COMPLETE {version} {cell}')
        for phase in ('build', 'local'):
            steps.append(dict(label=cell+'-'+phase, exit=0))
            put(f'campaign/{cell}-{phase}.exit', '0\n')
        count = 44 if p == 'lossless' else 43
        cases = sorted(task100_evidence.LOCAL_CASES | ({'float64'} if p == 'lossless' else set()))
        put(f'campaign/{cell}/local/result.json', dict(completed=True, cases=cases))
        put(f'campaign/{cell}-local.stdout', f'TASK100_LOCAL_SUITE_COMPLETE {count}\n')
        put(f'campaign/{cell}/local/lifecycle.stdout', 'RESOURCE_LIFECYCLES_COMPLETE cycles=30 fd_growth=0 task_growth=0\n')
        put(f'campaign/{cell}/local/large.stderr', 'TASK100_FORCED_SHORT_SENDS=2\n')
        generated = ('OWNERSHIP_WEBSOCKET_COMPLETE\n'
                     'TASK100_GENERATED_CONSUMER_COMPLETE typed_A=1 typed_B=1 rejected=1\n')
        put(f'campaign/{cell}/local/generated.stdout', generated + 'OWP_EVENT DECODE_ERROR x\n' * 4)
        if p == 'lossless':
            put(f'campaign/{cell}/local/float64.stdout', 'TASK100_FLOAT64_TRANSPORT_COMPLETE values=4 exact_bytes=4\n')
        if real_sleet and n == 'oam' and p == 'portable':
            steps.append(dict(label=cell+'-sleet', exit=0))
            put(f'campaign/{cell}-sleet.exit', '0\n')
            put(f'campaign/{cell}/sleet/result.json', dict(completed=True, revision=task100_evidence.SLEET_REV,
                                                         generated_typed_handlers=True, real_uci=False))
            put(f'campaign/{cell}-sleet.stdout', 'TASK100_REAL_PINNED_SLEET_COMPLETE\n')
            put(f'campaign/{cell}/sleet/sleet-source.txt', task100_evidence.SLEET_REV + '\n' + task100_evidence.SLEET_TREE + '\n')
            put(f'campaign/{cell}/sleet/sleet-binary.sha256', 'd'*64+'\n')
            put(f'campaign/{cell}/sleet/consumer.stdout', generated)
            put(f'campaign/{cell}/sleet/consumer.exit', '0\n')
            put(f'campaign/{cell}/sleet/server.exit', '0\n')
    log.append(f'TASK100_QUALIFICATION_COMPLETE {version} 24')
    put('qualify.log', '\n'.join(log)+'\n')
    put('campaign/result.json', dict(completed=True, compiler=version, real_sleet=real_sleet, cells=cells, steps=steps))
    return files


class Task100Evidence(unittest.TestCase):
    def setUp(self):
        dependencies = {name: (revision, hashlib.sha256(b'test license').hexdigest())
                        for name, (revision, _) in task100_evidence.DEPENDENCIES.items()}
        patcher = patch.object(task100_evidence, 'DEPENDENCIES', dependencies)
        patcher.start()
        self.addCleanup(patcher.stop)
        patcher = patch.object(task100_evidence, 'AHT_SPDX_LICENSE_SHA256', hashlib.sha256(b'test SPDX license').hexdigest())
        patcher.start()
        self.addCleanup(patcher.stop)

    def verify(self, files, compiler='14', real_sleet=True):
        return task100_evidence.verify_bundle(files, compiler, real_sleet, 'a'*40, 'b'*40, '123', '2')

    def test_all_four_bundles_and_reconciliation(self):
        for compiler, real in itertools.product(('14', '16'), (False, True)):
            self.assertEqual(self.verify(bundle(compiler, real), compiler, real)['cells'], 24)
        manifest = selection.select([], selection.inventory(), full_main=True)
        artifacts = [dict(name=f'task100-ada-websocket-{kind}-{c}', expired=False, size_in_bytes=1)
                     for kind, c in itertools.product(('transport', 'sleet'), ('14', '16'))]
        def loader(artifact):
            return bundle(artifact['name'][-2:], '-sleet-' in artifact['name'])
        jobs, attempts = metadata(artifacts)
        result = original.gate.reconcile_task100(artifacts, manifest, 'a'*40, 'b'*40, '123', '2', loader,
                                                jobs=jobs, attempts=attempts)
        self.assertEqual(len(result), 4)
        for index in range(4):
            with self.assertRaises(ValueError):
                original.gate.reconcile_task100(artifacts[:index]+artifacts[index+1:], manifest, 'a'*40, 'b'*40, '123', '2', loader,
                                                jobs=jobs, attempts=attempts)

    def test_wrong_source_tree_run_attempt_compiler_and_dirty_source(self):
        for field in ('sha', 'tree', 'run_id', 'run_attempt', 'compiler'):
            files = bundle()
            identity = json.loads(files['integration-identity.json'])
            identity[field] = 'wrong'
            files['integration-identity.json'] = json.dumps(identity).encode()
            with self.assertRaises(ValueError):
                self.verify(files)
        files = bundle()
        files['campaign/source-status.stdout'] = b'M production.adb\n'
        with self.assertRaises(ValueError):
            self.verify(files)

    def test_missing_duplicate_failed_or_incomplete_evidence(self):
        for path in bundle():
            files = bundle()
            del files[path]
            with self.subTest(path=path), self.assertRaises(ValueError):
                self.verify(files)
        for path, value in [('campaign/oam-portable-Normal-O0-build.exit', b'1'),
                            ('qualify.log', b''),
                            ('campaign/compiler.stdout', b'GNAT 13.0.0\n'),
                            ('campaign/oam-portable-Normal-O0-sleet.stdout', b''),
                            ('campaign/deps/aht/LICENSE', b'changed license'),
                            ('campaign/aht-identity.stdout', b'wrong pin'),
                            ('campaign/source.ads', b'changed source'),
                            ('campaign/oam-lossless-Normal-O0/local/float64.stdout', b''),
                            ('campaign/oam-portable-Normal-O0/local/large.stderr', b'TASK100_FORCED_SHORT_SENDS=0'),
                            ('campaign/oam-portable-Normal-O0/local/lifecycle.stdout', b'RESOURCE_LIFECYCLES_COMPLETE cycles=29 fd_growth=0 task_growth=0\n'),
                            ('campaign/oam-portable-Normal-O0/sleet/sleet-source.txt', (task100_evidence.SLEET_REV+'\n'+'0'*40+'\n').encode()),
                            ('campaign/oam-portable-Normal-O0/sleet/sleet-binary.sha256', b'malformed')]:
            files = bundle()
            files[path] = value
            with self.assertRaises(ValueError):
                self.verify(files)
        for field in ('cells', 'steps'):
            files = bundle()
            result = json.loads(files['campaign/result.json'])
            result[field][-1] = result[field][0]
            files['campaign/result.json'] = json.dumps(result).encode()
            with self.assertRaises(ValueError):
                self.verify(files)
        files = bundle()
        files['duplicate/integration-identity.json'] = files['integration-identity.json']
        with self.assertRaises(ValueError):
            self.verify(files)

    def test_incomplete_source_map_rejected_even_with_file_present(self):
        files = bundle()
        sources = json.loads(files['campaign/sources.json'])
        path = sorted(task100_evidence.UPSTREAM_SOURCES)[0]
        del sources[path]
        self.assertIn('campaign/' + path, files)
        files['campaign/sources.json'] = json.dumps(sources).encode()
        with self.assertRaisesRegex(ValueError, 'incomplete Task100 pinned source provenance'):
            self.verify(files)


if __name__ == '__main__':
    unittest.main()